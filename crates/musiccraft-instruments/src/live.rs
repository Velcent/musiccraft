//! Notes played in real time, for the instrument plugins
//! (`docs/spec/plugin-instruments.md`): the channel's note functions (swing,
//! arpeggiator, time shift, gate, echo) applied as notes arrive, and a pool of
//! voices with the app's mono, polyphony, cut and glide rules.
//!
//! The app applies the same note functions when it builds a sequence
//! (`musiccraft-render`); here they run on the audio thread, so everything is
//! fixed-size: no allocation, locks or panics after construction.

use musiccraft_params::{ARP_RATES, PPQ, TICKS_PER_STEP, inst};

use crate::arp::{MAX_ORDER, hash, order};
use crate::{Instrument, NoteShape, Params, Voice, VoiceStart};

/// Voices one plugin instance plays at once.
pub const MAX_VOICES: usize = 64;
/// Notes waiting for their time.
const QUEUE: usize = 2048;

/// A note arriving from the host, or leaving for the voices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Note {
    On { key: u8, velocity: f32 },
    Off { key: u8 },
}

/// Where a block sits in the host's time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timing {
    /// Beats per minute.
    pub tempo: f64,
    pub sample_rate: f64,
    /// The host's beat position at the block's first frame, while it plays.
    pub beats: Option<f64>,
}

impl Timing {
    fn ticks_per_frame(&self) -> f64 {
        self.tempo.max(1.0) / 60.0 * f64::from(PPQ) / self.sample_rate.max(1.0)
    }

    /// Frames in `ticks` ticks.
    fn frames(&self, ticks: f64) -> u64 {
        (ticks.max(0.0) / self.ticks_per_frame()).round() as u64
    }
}

/// What a queued note goes through next.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stage {
    Arp,
    Delivery,
    Voices,
}

#[derive(Debug, Clone, Copy)]
struct Queued {
    at: u64,
    order: u64,
    stage: Stage,
    note: Note,
}

/// A channel's note-function settings, read from its parameters.
#[derive(Debug, Clone, Copy)]
struct Settings {
    arp_mode: u8,
    arp_rate: f64,
    arp_range: u8,
    arp_gate: f64,
    swing: f64,
    truncate: bool,
    shift: f64,
    gate: f64,
    echoes: u8,
    echo_time: f64,
    echo_feed: f32,
    echo_pitch: i8,
}

impl Settings {
    fn of(params: &Params, swing: f32) -> Self {
        let get = |i: usize| params.get(i).copied().unwrap_or(0.0);
        let int = |i: usize| get(i).round();
        let rate = ARP_RATES
            .get(int(inst::ARP_RATE).clamp(0.0, 4.0) as usize)
            .map_or(TICKS_PER_STEP, |(_, t)| *t);
        Self {
            arp_mode: int(inst::ARP_MODE).clamp(0.0, 4.0) as u8,
            arp_rate: f64::from(rate.max(1)),
            arp_range: int(inst::ARP_RANGE).clamp(1.0, 4.0) as u8,
            arp_gate: f64::from(get(inst::ARP_GATE).clamp(0.05, 1.0)),
            swing: f64::from((swing * get(inst::SWING_MIX)).clamp(0.0, 1.0)),
            truncate: get(inst::SWING_TRUNCATE) >= 0.5,
            shift: f64::from(int(inst::TIME_SHIFT)),
            gate: f64::from(int(inst::GATE).max(0.0)),
            echoes: int(inst::ECHO_COUNT).clamp(0.0, 8.0) as u8,
            echo_time: f64::from(int(inst::ECHO_TIME).max(1.0)),
            echo_feed: get(inst::ECHO_FEED).clamp(0.0, 1.0),
            echo_pitch: int(inst::ECHO_PITCH).clamp(-12.0, 12.0) as i8,
        }
    }
}

/// Frames of latency a channel's negative time shift asks the host for: the
/// plugin plays such notes at once and the host moves its output earlier.
#[must_use]
pub fn latency(params: &Params, tempo: f64, sample_rate: f64) -> u32 {
    let shift = params.get(inst::TIME_SHIFT).copied().unwrap_or(0.0).round();
    if shift >= 0.0 {
        return 0;
    }
    let timing = Timing {
        tempo,
        sample_rate,
        beats: None,
    };
    u32::try_from(timing.frames(f64::from(-shift))).unwrap_or(u32::MAX)
}

/// The arpeggiator's state.
#[derive(Debug, Clone, Copy)]
struct Arp {
    /// Velocity of each held key.
    held: [Option<f32>; 128],
    /// Steps played since the keys were last all let go.
    counter: usize,
    /// Whether this step period has played its step.
    stepped: bool,
    /// A key press asked for a step, in the period starting at this tick.
    pressed: Option<f64>,
}

/// The note functions of one channel, in real time. See the module docs.
#[derive(Debug, Clone)]
pub struct NoteFunctions {
    queue: Box<[Queued; QUEUE]>,
    len: usize,
    next_order: u64,
    /// Absolute frame of the next block's first frame.
    now: u64,
    /// Ticks run while the host is stopped.
    free_ticks: f64,
    /// Frames each key's note-on was swung by, and the frame it reaches the
    /// arpeggiator (for its note-off).
    swung: [(u64, u64); 128],
    /// Echo count and pitch step each key's note-on had (for its note-off).
    echoes: [(u8, i8); 128],
    arp: Arp,
}

impl Default for NoteFunctions {
    fn default() -> Self {
        Self::new()
    }
}

/// The block being run: its first absolute frame, its tick position and
/// tempo, and the settings.
#[derive(Debug, Clone, Copy)]
struct Block {
    start: u64,
    ticks: f64,
    timing: Timing,
    settings: Settings,
}

impl Block {
    fn ticks_at(&self, frame: u64) -> f64 {
        self.ticks + frame.saturating_sub(self.start) as f64 * self.timing.ticks_per_frame()
    }

    /// The first frame at or after `tick`.
    fn frame_of(&self, tick: f64) -> u64 {
        let frames = (tick - self.ticks) / self.timing.ticks_per_frame();
        self.start + (frames - 1e-6).ceil().max(0.0) as u64
    }
}

impl NoteFunctions {
    /// Allocates the queue (main thread).
    #[must_use]
    pub fn new() -> Self {
        let empty = Queued {
            at: 0,
            order: 0,
            stage: Stage::Voices,
            note: Note::Off { key: 0 },
        };
        Self {
            queue: Box::new([empty; QUEUE]),
            len: 0,
            next_order: 0,
            now: 0,
            free_ticks: 0.0,
            swung: [(0, 0); 128],
            echoes: [(0, 0); 128],
            arp: Arp {
                held: [None; 128],
                counter: 0,
                stepped: false,
                pressed: None,
            },
        }
    }

    /// Forgets every waiting note and held key (all notes off, transport
    /// jumps).
    pub fn reset(&mut self) {
        self.len = 0;
        self.arp.held = [None; 128];
        self.arp.counter = 0;
        self.arp.stepped = false;
        self.arp.pressed = None;
    }

    /// Runs one block of `frames` frames: `inputs` are the host's notes with
    /// their frame in the block, in order. Every note for the voices goes to
    /// `out` with its frame in the block, in order; notes due later wait for
    /// a later block.
    pub fn run(
        &mut self,
        params: &Params,
        swing: f32,
        timing: Timing,
        frames: u32,
        inputs: &[(u32, Note)],
        out: &mut impl FnMut(u32, Note),
    ) {
        let settings = Settings::of(params, swing);
        let ticks = timing.beats.map_or(self.free_ticks, |b| b * f64::from(PPQ));
        let block = Block {
            start: self.now,
            ticks,
            timing,
            settings,
        };
        let end = block.start + u64::from(frames);
        self.free_ticks += f64::from(frames) * timing.ticks_per_frame();
        self.now = end;
        let rate = settings.arp_rate;
        let mut boundary = (settings.arp_mode > 0).then(|| {
            let tick = ((ticks - 1e-6) / rate).ceil() * rate;
            (tick, block.frame_of(tick))
        });
        let mut next_input = 0;
        loop {
            let t_input = inputs
                .get(next_input)
                .map(|(f, _)| block.start + u64::from(*f));
            let t_queue = self
                .earliest()
                .map(|i| self.queue.get(i).map_or(end, |q| q.at));
            let t_boundary = boundary.map(|(_, f)| f);
            let Some(t) = [t_input, t_queue, t_boundary]
                .into_iter()
                .flatten()
                .min()
                .filter(|t| *t < end)
            else {
                break;
            };
            while let Some(&(f, note)) = inputs.get(next_input) {
                if block.start + u64::from(f) != t {
                    break;
                }
                self.input(&block, t, note);
                next_input += 1;
            }
            // Keys let go on a step boundary are not held for its step.
            self.drain(
                &block,
                t,
                |q| q.stage == Stage::Arp && matches!(q.note, Note::Off { .. }),
                out,
            );
            if let Some((tick, frame)) = boundary
                && frame == t
            {
                self.boundary(&block, t, tick);
                let next = tick + rate;
                boundary = Some((next, block.frame_of(next)));
            }
            self.drain(&block, t, |_| true, out);
            // A step started by a key press waits for every key pressed on
            // the same frame (a chord).
            if let Some(tick) = self.arp.pressed.take() {
                if !self.arp.stepped {
                    self.step(&block, t, tick);
                }
                self.drain(&block, t, |_| true, out);
            }
        }
    }

    /// Takes every queued note due by `t` that `pick` accepts, in order, and
    /// passes it on (notes it queues for `t` are taken too).
    fn drain(
        &mut self,
        block: &Block,
        t: u64,
        pick: impl Fn(&Queued) -> bool,
        out: &mut impl FnMut(u32, Note),
    ) {
        while let Some(i) = self.earliest_where(|q| q.at <= t && pick(q)) {
            let Some(item) = self.queue.get(i).copied() else {
                break;
            };
            self.remove(i);
            match item.stage {
                Stage::Arp => self.arp_stage(block, t, item.note),
                Stage::Delivery => self.delivery(block, t, item.note),
                Stage::Voices => out((t - block.start) as u32, item.note),
            }
        }
    }

    /// Index of the queued note due first (earliest queued among equals).
    fn earliest(&self) -> Option<usize> {
        self.earliest_where(|_| true)
    }

    fn earliest_where(&self, pick: impl Fn(&Queued) -> bool) -> Option<usize> {
        self.queue
            .get(..self.len)?
            .iter()
            .enumerate()
            .filter(|(_, q)| pick(q))
            .min_by_key(|(_, q)| (q.at, q.order))
            .map(|(i, _)| i)
    }

    fn remove(&mut self, i: usize) {
        if i < self.len {
            self.queue.copy_within(i + 1..self.len, i);
            self.len -= 1;
        }
    }

    fn push(&mut self, at: u64, stage: Stage, note: Note) {
        if let Some(slot) = self.queue.get_mut(self.len) {
            *slot = Queued {
                at,
                order: self.next_order,
                stage,
                note,
            };
            self.len += 1;
            self.next_order += 1;
        }
    }

    /// A host note: swing, then on to the arpeggiator.
    fn input(&mut self, block: &Block, t: u64, note: Note) {
        let s = block.settings;
        let delay = match note {
            Note::On { key, .. } => {
                let tick = block.ticks_at(t);
                let step = f64::from(TICKS_PER_STEP);
                let offbeat = (tick.rem_euclid(2.0 * step) - step).abs() <= 1.0;
                let delay = if block.timing.beats.is_some() && offbeat && s.swing > 0.0 {
                    block.timing.frames((s.swing * step / 2.0).round())
                } else {
                    0
                };
                if let Some(slot) = self.swung.get_mut(usize::from(key)) {
                    *slot = (delay, t + delay);
                }
                delay
            }
            Note::Off { key } => {
                let (delay, on_at) = self.swung.get(usize::from(key)).copied().unwrap_or((0, 0));
                if s.truncate {
                    // The end stays put, but a note lasts at least a tick.
                    (on_at + block.timing.frames(1.0)).saturating_sub(t)
                } else {
                    delay
                }
            }
        };
        self.push(t + delay, Stage::Arp, note);
    }

    /// A step boundary of the arpeggiator.
    fn boundary(&mut self, block: &Block, t: u64, tick: f64) {
        self.arp.stepped = false;
        if self.arp.held.iter().any(Option::is_some) {
            self.step(block, t, tick);
        } else {
            self.arp.counter = 0;
        }
    }

    fn arp_stage(&mut self, block: &Block, t: u64, note: Note) {
        if block.settings.arp_mode == 0 {
            self.delivery(block, t, note);
            return;
        }
        match note {
            Note::On { key, velocity } => {
                if let Some(slot) = self.arp.held.get_mut(usize::from(key)) {
                    *slot = Some(velocity);
                }
                // A key pressed in a step period that has not played yet
                // starts its step at once (see `run`).
                if !self.arp.stepped {
                    let rate = block.settings.arp_rate;
                    self.arp.pressed = Some((block.ticks_at(t) / rate).floor() * rate);
                }
            }
            Note::Off { key } => {
                if let Some(slot) = self.arp.held.get_mut(usize::from(key)) {
                    *slot = None;
                }
            }
        }
    }

    /// Plays the next arpeggio key at `t`, in the period starting at `tick`.
    fn step(&mut self, block: &Block, t: u64, tick: f64) {
        let s = block.settings;
        let mut keys = [0_u8; 128];
        let mut count = 0;
        let mut loudest: f32 = 0.0;
        for (key, velocity) in self.arp.held.iter().enumerate() {
            if let (Some(v), Some(slot)) = (velocity, keys.get_mut(count)) {
                *slot = key as u8;
                count += 1;
                loudest = loudest.max(*v);
            }
        }
        let mut cycle = [0_u8; MAX_ORDER];
        let len = order(
            keys.get(..count).unwrap_or(&[]),
            s.arp_mode,
            s.arp_range,
            &mut cycle,
        );
        if len == 0 {
            return;
        }
        let index = if s.arp_mode == 4 {
            (hash(tick.max(0.0) as u64) % len as u64) as usize
        } else {
            self.arp.counter % len
        };
        let Some(&key) = cycle.get(index) else {
            return;
        };
        self.arp.counter += 1;
        self.arp.stepped = true;
        let left = tick + s.arp_rate - block.ticks_at(t);
        let length = (s.arp_rate * s.arp_gate).round().max(1.0).min(left);
        let frames = block.timing.frames(length).max(1);
        self.delivery(
            block,
            t,
            Note::On {
                key,
                velocity: loudest,
            },
        );
        self.push(t + frames, Stage::Delivery, Note::Off { key });
    }

    /// Time shift, gate and echo, then on to the voices.
    fn delivery(&mut self, block: &Block, t: u64, note: Note) {
        let s = block.settings;
        let timing = block.timing;
        let at = t + timing.frames(s.shift);
        let echo_step = timing.frames(s.echo_time);
        let gate = (s.gate > 0.0).then(|| timing.frames(s.gate).max(1));
        match note {
            Note::On { key, velocity } => {
                if let Some(slot) = self.echoes.get_mut(usize::from(key)) {
                    *slot = (s.echoes, s.echo_pitch);
                }
                self.push(at, Stage::Voices, note);
                if let Some(length) = gate {
                    self.push(at + length, Stage::Voices, Note::Off { key });
                }
                let mut velocity = velocity;
                for i in 1..=s.echoes {
                    velocity *= s.echo_feed;
                    let Some(echo) = echo_key(key, s.echo_pitch, i) else {
                        break;
                    };
                    if velocity < 1.0 / 127.0 {
                        break;
                    }
                    let start = at + echo_step * u64::from(i);
                    self.push(
                        start,
                        Stage::Voices,
                        Note::On {
                            key: echo,
                            velocity,
                        },
                    );
                    if let Some(length) = gate {
                        self.push(start + length, Stage::Voices, Note::Off { key: echo });
                    }
                }
            }
            Note::Off { key } => {
                // With a fixed gate, notes end by themselves.
                if gate.is_some() {
                    return;
                }
                self.push(at, Stage::Voices, note);
                let (count, pitch) = self.echoes.get(usize::from(key)).copied().unwrap_or((0, 0));
                for i in 1..=count {
                    let Some(echo) = echo_key(key, pitch, i) else {
                        break;
                    };
                    self.push(
                        at + echo_step * u64::from(i),
                        Stage::Voices,
                        Note::Off { key: echo },
                    );
                }
            }
        }
    }
}

/// The key of echo `i` of `key`, if it is a MIDI key.
fn echo_key(key: u8, pitch: i8, i: u8) -> Option<u8> {
    let k = i32::from(key) + i32::from(pitch) * i32::from(i);
    u8::try_from(k).ok().filter(|k| *k <= 127)
}

/// A plugin's voices, with the engine's rules for mono, polyphony limits,
/// cut groups (inside the instance) and glide.
#[derive(Debug, Clone)]
pub struct Poly {
    voices: Vec<Voice>,
    counter: u64,
    sample_rate: f32,
}

impl Poly {
    /// [`MAX_VOICES`] idle voices (main thread).
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: (0..MAX_VOICES).map(|_| Voice::new(sample_rate)).collect(),
            counter: 0,
            sample_rate,
        }
    }

    /// Voices sounding.
    #[must_use]
    pub fn active(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    /// Starts a note.
    pub fn note_on(&mut self, key: u8, velocity: f32, instrument: &Instrument, params: &Params) {
        let int = |i: usize| params.get(i).map_or(0, |v| v.round().max(0.0) as u32);
        let group = int(inst::CUT_GROUP);
        if group > 0 && int(inst::CUT_BY) == group {
            for voice in &mut self.voices {
                voice.cut(0);
            }
        }
        let limit = if int(inst::MONO) > 0 {
            1
        } else {
            int(inst::POLY_MAX) as usize
        };
        if limit > 0 {
            let sounding = |v: &Voice| v.active && !v.is_cut();
            let mut count = self.voices.iter().filter(|v| sounding(v)).count();
            while count >= limit {
                let Some(oldest) = self
                    .voices
                    .iter_mut()
                    .filter(|v| sounding(v))
                    .min_by_key(|v| v.age)
                else {
                    break;
                };
                oldest.cut(0);
                count -= 1;
            }
        }
        let glide_from = self
            .voices
            .iter()
            .filter(|v| v.held())
            .max_by_key(|v| v.age)
            .map(Voice::current_key);
        let index = self.voices.iter().position(|v| !v.active).or_else(|| {
            self.voices
                .iter()
                .enumerate()
                .min_by_key(|(_, v)| v.age)
                .map(|(i, _)| i)
        });
        let Some(voice) = index.and_then(|i| self.voices.get_mut(i)) else {
            return;
        };
        self.counter += 1;
        let start = VoiceStart {
            channel: 0,
            key,
            velocity,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from,
            slide: false,
            shape: NoteShape::default(),
        };
        voice.start(start, instrument, params, self.counter);
    }

    /// Ends a note (one-shot samples play on).
    pub fn note_off(&mut self, key: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.key == key {
                voice.release(false);
            }
        }
    }

    /// Cuts every voice quickly (choke, all notes off).
    pub fn cut_all(&mut self) {
        for voice in &mut self.voices {
            voice.cut(0);
        }
    }

    /// Silences every voice at once (the host reset the plugin).
    pub fn reset(&mut self) {
        for voice in &mut self.voices {
            voice.active = false;
        }
    }

    /// Cuts the voices playing `key`.
    pub fn choke(&mut self, key: u8) {
        for voice in &mut self.voices {
            if voice.key == key {
                voice.cut(0);
            }
        }
    }

    /// Adds every voice's output into `left`/`right`.
    pub fn render(
        &mut self,
        instrument: &Instrument,
        params: &Params,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            voice.render(instrument, params, left, right, self.sample_rate);
        }
    }
}

#[cfg(test)]
mod tests;
