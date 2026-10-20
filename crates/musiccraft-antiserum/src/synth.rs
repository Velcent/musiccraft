//! The synth: parameters, voice allocation and the block loop.

use std::sync::Arc;

use musiccraft_dsp::ProcessContext;
use musiccraft_params::{ARP_RATES, PPQ, ParamSpec};

use crate::fx::{FX_PARAMS, FX_SLOTS, Rack};
use crate::modulation::LfoShape;
use crate::noise::NoiseBank;
use crate::notes::{Clock, Event, Events, Generator, Pattern, Sequence};
use crate::params::{
    self, ARP_BASE, FX_BASE, FX_SLOT_BASE, GLIDE_MODE, LFOS, OSCS, PARAM_COUNT, POLYPHONY,
    SEQ_BASE, SYNC_RATES, VOICE_MODE, arp, lfo, lfo_param, seq,
};
use crate::player::{SINE_LEN, sine_table};
use crate::source::Source;
use crate::voice::{CONTROL, Ctx, State, Voice};
use crate::wavetable::Wavetable;

/// Most voices that play at once (the `polyphony` maximum).
pub const MAX_VOICES: usize = 32;
/// Extra slots where stolen voices fade out.
const SPARE_VOICES: usize = 8;
/// Keys a mono synth remembers.
const NOTE_STACK: usize = 16;

/// Data that is not a parameter, built off the audio thread and taken in by
/// a running synth with [`Antiserum::swap`] (the app's engine and the plugin
/// both use it).
#[derive(Debug, Clone)]
pub enum Data {
    /// Oscillator (0-based) and its new table; holds the old one after.
    Table(usize, Arc<Wavetable>),
    /// LFO (0-based) and its new drawn shape.
    Shape(usize, Box<LfoShape>),
    /// The sequencer's new pattern.
    Sequence(Box<Sequence>),
    /// Oscillator (0-based) and its new source; holds the old one after.
    Source(usize, Arc<Source>),
}

/// What every voice reads; kept apart from the voices so both can be
/// borrowed at once.
struct Shared {
    sample_rate: f32,
    specs: &'static [ParamSpec],
    base: Box<[f32; PARAM_COUNT]>,
    unit: Box<[f32; PARAM_COUNT]>,
    tables: [Arc<Wavetable>; OSCS],
    sources: [Arc<Source>; OSCS],
    sine: Box<[f32; SINE_LEN + 1]>,
    noise: NoiseBank,
    shapes: [LfoShape; LFOS],
    free_phase: [f32; LFOS],
    tempo: f32,
    /// Beat position for the effects: free-running, or the host's.
    beats: f64,
    bend: f32,
    wheel: f32,
    aftertouch: f32,
    /// Semitones every note moves by (the app's channel pitch knob).
    transpose: f32,
}

impl Shared {
    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            specs: self.specs,
            base: &self.base,
            unit: &self.unit,
            tables: &self.tables,
            sources: &self.sources,
            sine: &self.sine,
            noise: &self.noise,
            shapes: &self.shapes,
            free_phase: &self.free_phase,
            sample_rate: self.sample_rate,
            tempo: self.tempo,
            bend: self.bend,
            wheel: self.wheel,
            aftertouch: self.aftertouch,
            transpose: self.transpose,
        }
    }
}

/// The Antiserum synthesizer.
pub struct Antiserum {
    shared: Shared,
    voices: Vec<Voice>,
    stack: [u8; NOTE_STACK],
    stack_len: usize,
    last_pitch: Option<f32>,
    age: u64,
    seed: u32,
    rack: Rack,
    /// The effects' modulation (from the newest playing voice), as offsets.
    fx_offsets: Box<[f32; FX_PARAMS]>,
    /// The effect modules' parameter values with that modulation.
    fx_values: Box<[f32; FX_PARAMS]>,
    generator: Generator,
    /// Generated note events and the sample they happened on (tests).
    #[cfg(test)]
    pub(crate) log: Vec<(u64, Event)>,
    #[cfg(test)]
    clock: u64,
}

impl Antiserum {
    /// A synth with default parameters and the basic wavetable on every
    /// oscillator. Main thread (allocates).
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() && sample_rate > 0.0 {
            sample_rate
        } else {
            48_000.0
        };
        let specs = params::params();
        let base = Box::new(params::defaults());
        let mut unit = Box::new([0.0; PARAM_COUNT]);
        for ((u, spec), value) in unit.iter_mut().zip(specs).zip(base.iter()) {
            *u = spec.to_unit(*value);
        }
        let basic = Arc::new(Wavetable::basic());
        Self {
            shared: Shared {
                sample_rate,
                specs,
                base,
                unit,
                tables: [basic.clone(), basic.clone(), basic],
                sources: std::array::from_fn(|_| Arc::new(Source::default())),
                sine: sine_table(),
                noise: NoiseBank::new(),
                shapes: [LfoShape::default(); LFOS],
                free_phase: [0.0; LFOS],
                tempo: 120.0,
                beats: 0.0,
                bend: 0.0,
                wheel: 0.0,
                aftertouch: 0.0,
                transpose: 0.0,
            },
            voices: (0..MAX_VOICES + SPARE_VOICES)
                .map(|_| Voice::new(sample_rate))
                .collect(),
            stack: [0; NOTE_STACK],
            stack_len: 0,
            last_pitch: None,
            age: 0,
            seed: 0x2545_f491,
            rack: Rack::new(sample_rate),
            fx_offsets: Box::new([0.0; FX_PARAMS]),
            fx_values: Box::new([0.0; FX_PARAMS]),
            generator: Generator::default(),
            #[cfg(test)]
            log: Vec::new(),
            #[cfg(test)]
            clock: 0,
        }
    }

    /// The sequencer's pattern.
    #[must_use]
    pub fn sequence(&self) -> &Sequence {
        self.generator.sequence()
    }

    /// Replaces the sequencer's pattern (values are clamped into range).
    pub fn set_sequence(&mut self, sequence: &Sequence) {
        self.generator.set_sequence(sequence);
    }

    /// The host's beat position, for tempo-synced effects; call it each
    /// block while the host plays. Otherwise the effects' clock runs freely.
    pub fn set_beat_position(&mut self, beats: f64) {
        if beats.is_finite() {
            self.shared.beats = beats;
        }
    }

    /// The sample rate it renders at.
    #[must_use]
    pub fn sample_rate(&self) -> f32 {
        self.shared.sample_rate
    }

    /// Sets parameter `index` (sanitized into range); unknown indices are
    /// ignored.
    pub fn set_param(&mut self, index: usize, value: f32) {
        let (Some(spec), Some(base), Some(unit)) = (
            self.shared.specs.get(index),
            self.shared.base.get_mut(index),
            self.shared.unit.get_mut(index),
        ) else {
            return;
        };
        let value = if value.is_finite() {
            spec.sanitize(value)
        } else {
            spec.default
        };
        *base = value;
        *unit = spec.to_unit(value);
    }

    /// Parameter `index`'s value (0 if unknown).
    #[must_use]
    pub fn param(&self, index: usize) -> f32 {
        self.shared.base.get(index).copied().unwrap_or(0.0)
    }

    /// Replaces oscillator `osc`'s sample source (sample, analysis, zones),
    /// returning the old one so the caller drops it off the audio thread.
    /// Notes already playing keep their positions.
    pub fn set_source(&mut self, osc: usize, source: Arc<Source>) -> Option<Arc<Source>> {
        self.shared
            .sources
            .get_mut(osc)
            .map(|slot| std::mem::replace(slot, source))
    }

    /// Oscillator `osc`'s sample source.
    #[must_use]
    pub fn source(&self, osc: usize) -> Option<&Arc<Source>> {
        self.shared.sources.get(osc)
    }

    /// Replaces oscillator `osc`'s wavetable, returning the old one so the
    /// caller drops it off the audio thread.
    pub fn set_wavetable(&mut self, osc: usize, table: Arc<Wavetable>) -> Option<Arc<Wavetable>> {
        self.shared
            .tables
            .get_mut(osc)
            .map(|slot| std::mem::replace(slot, table))
    }

    /// Oscillator `osc`'s wavetable.
    #[must_use]
    pub fn wavetable(&self, osc: usize) -> Option<&Arc<Wavetable>> {
        self.shared.tables.get(osc)
    }

    /// Takes in `data` in one step on the audio thread (allocation-free).
    /// What it replaces goes back into `data`, so the caller frees it off
    /// the audio thread.
    pub fn swap(&mut self, data: &mut Data) {
        match data {
            Data::Table(osc, table) => {
                if let Some(old) = self.set_wavetable(*osc, Arc::clone(table)) {
                    *table = old;
                }
            }
            Data::Source(osc, source) => {
                if let Some(old) = self.set_source(*osc, Arc::clone(source)) {
                    *source = old;
                }
            }
            Data::Shape(lfo, shape) => self.set_lfo_shape(*lfo, **shape),
            Data::Sequence(sequence) => self.set_sequence(sequence),
        }
    }

    /// Sets LFO `lfo`'s drawn shape.
    pub fn set_lfo_shape(&mut self, lfo: usize, shape: LfoShape) {
        if let Some(slot) = self.shared.shapes.get_mut(lfo) {
            *slot = shape;
        }
    }

    /// LFO `lfo`'s drawn shape.
    #[must_use]
    pub fn lfo_shape(&self, lfo: usize) -> Option<&LfoShape> {
        self.shared.shapes.get(lfo)
    }

    /// The host tempo, for synced LFOs.
    pub fn set_tempo(&mut self, bpm: f32) {
        if bpm.is_finite() && bpm > 0.0 {
            self.shared.tempo = bpm;
        }
    }

    /// Moves every note, sounding ones included, by `semitones` (the app's
    /// channel pitch knob, −48..48).
    pub fn set_transpose(&mut self, semitones: f32) {
        self.shared.transpose = finite(semitones).clamp(-48.0, 48.0);
    }

    /// Pitch bend, −1..1.
    pub fn set_pitch_bend(&mut self, value: f32) {
        self.shared.bend = finite(value).clamp(-1.0, 1.0);
    }

    /// Mod wheel, 0..1.
    pub fn set_mod_wheel(&mut self, value: f32) {
        self.shared.wheel = finite(value).clamp(0.0, 1.0);
    }

    /// Channel aftertouch, 0..1.
    pub fn set_aftertouch(&mut self, value: f32) {
        self.shared.aftertouch = finite(value).clamp(0.0, 1.0);
    }

    /// Aftertouch for the voices playing `key`, 0..1.
    pub fn set_poly_aftertouch(&mut self, key: u8, value: f32) {
        let value = finite(value).clamp(0.0, 1.0);
        for voice in &mut self.voices {
            if voice.state == State::Playing && voice.key == key {
                voice.poly_aftertouch = value;
            }
        }
    }

    /// Number of voices sounding (fading ones included).
    #[must_use]
    pub fn active_voices(&self) -> usize {
        self.voices
            .iter()
            .filter(|v| v.state != State::Idle)
            .count()
    }

    /// The pitches of the playing voices (tests).
    #[cfg(test)]
    pub(crate) fn pitches(&self) -> Vec<f32> {
        self.voices
            .iter()
            .filter(|v| v.state == State::Playing)
            .map(Voice::pitch)
            .collect()
    }

    fn next_seed(&mut self) -> u32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        self.seed
    }

    /// What the arpeggiator or sequencer plays, if either is on.
    fn pattern(&self) -> Option<Pattern> {
        if self.param(SEQ_BASE + seq::ON) >= 0.5 {
            return Some(Pattern::Seq {
                length: self.param(SEQ_BASE + seq::LENGTH) as usize,
            });
        }
        let mode = self.param(ARP_BASE + arp::MODE) as u8;
        (mode > 0).then(|| Pattern::Arp {
            mode,
            octaves: self.param(ARP_BASE + arp::RANGE) as u8,
            gate: self.param(ARP_BASE + arp::GATE),
        })
    }

    /// The length of the pattern's steps.
    fn clock(&self) -> Clock {
        let rate = if self.param(SEQ_BASE + seq::ON) >= 0.5 {
            self.param(SEQ_BASE + seq::RATE)
        } else {
            self.param(ARP_BASE + arp::RATE)
        };
        let ticks = ARP_RATES.get(rate as usize).map_or(PPQ / 4, |r| r.1);
        let beats = f64::from(ticks) / f64::from(PPQ);
        let seconds = beats * 60.0 / f64::from(self.shared.tempo);
        Clock {
            step_samples: seconds * f64::from(self.shared.sample_rate),
        }
    }

    /// Plays generated events on the voices.
    fn apply(&mut self, events: &Events) {
        for &event in events.as_slice() {
            #[cfg(test)]
            self.log.push((self.clock, event));
            match event {
                Event::On(key, velocity) => self.key_on(key, velocity),
                Event::Off(key) => self.key_off(key),
            }
        }
    }

    /// Key down. Velocity 0..1; velocity 0 is a key up. With the arpeggiator
    /// or sequencer on, the key feeds the pattern instead of playing.
    pub fn note_on(&mut self, key: u8, velocity: f32) {
        let velocity = finite(velocity);
        if velocity <= 0.0 {
            self.note_off(key);
            return;
        }
        let key = key.min(127);
        if self.pattern().is_some() {
            self.generator.press(key, velocity.min(1.0));
        } else {
            self.key_on(key, velocity);
        }
    }

    /// Key up.
    pub fn note_off(&mut self, key: u8) {
        let generated = self.generator.is_sounding(key);
        let events = self.generator.release(key);
        self.apply(&events);
        // A key played directly before the pattern was switched on.
        if !generated {
            self.key_off(key);
        }
    }

    fn key_on(&mut self, key: u8, velocity: f32) {
        let mode = self.param(VOICE_MODE) as u8;
        let others_held = self.stack_len > 0;
        self.push_key(key);
        let legato_only = self.param(GLIDE_MODE) >= 0.5;
        let glide_from = match self.last_pitch {
            Some(pitch) if !legato_only || others_held => Some(pitch),
            _ => None,
        };
        self.age += 1;
        let age = self.age;
        let seed = self.next_seed();
        if mode == 0 {
            self.start_poly(key, velocity, seed, age, glide_from);
        } else {
            let retrigger = mode == 1 || !others_held;
            let playing = self.voices.iter().position(|v| v.state == State::Playing);
            let ctx = self.shared.ctx();
            match playing.and_then(|i| self.voices.get_mut(i)) {
                Some(voice) => {
                    let from = glide_from.map(|_| voice.pitch());
                    voice.retarget(&ctx, key, velocity, retrigger, from);
                    voice.age = age;
                }
                None => {
                    if let Some(voice) = self.voices.iter_mut().find(|v| v.state == State::Idle) {
                        voice.start(&ctx, key, velocity, seed, age, glide_from);
                    }
                }
            }
        }
        self.last_pitch = Some(f32::from(key));
    }

    fn start_poly(&mut self, key: u8, velocity: f32, seed: u32, age: u64, glide_from: Option<f32>) {
        let polyphony = (self.param(POLYPHONY) as usize).clamp(1, MAX_VOICES);
        let playing = self
            .voices
            .iter()
            .filter(|v| v.state == State::Playing)
            .count();
        if playing >= polyphony {
            // Steal the oldest releasing voice, else the oldest.
            let victim = self
                .voices
                .iter()
                .enumerate()
                .filter(|(_, v)| v.state == State::Playing)
                .min_by_key(|(_, v)| (!v.releasing(), v.age))
                .map(|(i, _)| i);
            if let Some(voice) = victim.and_then(|i| self.voices.get_mut(i)) {
                voice.steal();
            }
        }
        let slot = self
            .voices
            .iter()
            .position(|v| v.state == State::Idle)
            .or_else(|| {
                // Every slot busy: take the fading voice that started first.
                self.voices
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.state == State::Fading)
                    .min_by_key(|(_, v)| v.age)
                    .map(|(i, _)| i)
            });
        let ctx = self.shared.ctx();
        if let Some(voice) = slot.and_then(|i| self.voices.get_mut(i)) {
            voice.start(&ctx, key, velocity, seed, age, glide_from);
        }
    }

    fn key_off(&mut self, key: u8) {
        let was_top = self.stack_len > 0 && self.stack.get(self.stack_len - 1) == Some(&key);
        self.remove_key(key);
        let mode = self.param(VOICE_MODE) as u8;
        if mode == 0 {
            for voice in &mut self.voices {
                if voice.state == State::Playing && voice.held && voice.key == key {
                    voice.release();
                }
            }
            return;
        }
        let back_to = self
            .stack_len
            .checked_sub(1)
            .and_then(|i| self.stack.get(i))
            .copied();
        let ctx = self.shared.ctx();
        for voice in &mut self.voices {
            if voice.state != State::Playing || voice.key != key {
                continue;
            }
            match back_to {
                // Back to the most recent key still held, without retriggering.
                Some(previous) if was_top => {
                    let from = voice.pitch();
                    let velocity = 1.0;
                    voice.retarget(&ctx, previous, velocity, false, Some(from));
                    self.last_pitch = Some(f32::from(previous));
                }
                Some(_) => {}
                None => voice.release(),
            }
        }
    }

    fn push_key(&mut self, key: u8) {
        self.remove_key(key);
        if self.stack_len == NOTE_STACK {
            self.stack.copy_within(1.., 0);
            self.stack_len -= 1;
        }
        if let Some(slot) = self.stack.get_mut(self.stack_len) {
            *slot = key;
            self.stack_len += 1;
        }
    }

    fn remove_key(&mut self, key: u8) {
        let len = self.stack_len;
        if let Some(i) = self
            .stack
            .get(..len)
            .and_then(|s| s.iter().position(|&k| k == key))
        {
            self.stack.copy_within(i + 1..len, i);
            self.stack_len -= 1;
        }
    }

    /// Releases every note and stops the arpeggiator or sequencer.
    pub fn all_notes_off(&mut self) {
        let _ = self.generator.stop();
        self.stack_len = 0;
        for voice in &mut self.voices {
            if voice.state == State::Playing {
                voice.release();
            }
        }
    }

    /// Silences everything at once (effect tails too) and forgets held keys
    /// and glide.
    pub fn reset(&mut self) {
        let _ = self.generator.stop();
        self.stack_len = 0;
        self.last_pitch = None;
        for voice in &mut self.voices {
            voice.kill();
        }
        self.rack.reset();
    }

    /// Renders into `left` and `right` (overwriting them).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        let (left, right) = (&mut left[..frames], &mut right[..frames]);
        left.fill(0.0);
        right.fill(0.0);
        let mut start = 0;
        while start < frames {
            let mut end = (start + CONTROL).min(frames);
            // Pattern events, then a block that stops at the next one.
            match self.pattern() {
                Some(pattern) => {
                    let events = self.generator.due(pattern, self.clock());
                    self.apply(&events);
                }
                None if self.generator.busy() => {
                    let events = self.generator.stop();
                    self.apply(&events);
                }
                None => {}
            }
            if let Some(next) = self.generator.until_next() {
                let next = next.ceil().max(1.0) as usize;
                end = end.min(start + next);
            }
            let len = end - start;
            self.shared.advance_free_lfos(len);
            let ctx = self.shared.ctx();
            let (Some(l), Some(r)) = (left.get_mut(start..end), right.get_mut(start..end)) else {
                break;
            };
            for voice in &mut self.voices {
                if voice.state != State::Idle {
                    voice.control(&ctx, len);
                    voice.render(&ctx, l, r);
                }
            }
            self.run_effects(l, r);
            self.generator.advance(len as f64);
            let beats = len as f64 * f64::from(self.shared.tempo)
                / 60.0
                / f64::from(self.shared.sample_rate);
            self.shared.beats += beats;
            #[cfg(test)]
            {
                self.clock += len as u64;
            }
            start = end;
        }
    }

    /// The effects rack over one control block.
    fn run_effects(&mut self, left: &mut [f32], right: &mut [f32]) {
        let mut slots = [0_usize; FX_SLOTS];
        let mut any = false;
        for (i, slot) in slots.iter_mut().enumerate() {
            *slot = self.param(FX_SLOT_BASE + i) as usize;
            any |= *slot != 0;
        }
        if !any {
            return;
        }
        // Modulation follows the newest playing voice; with none, it holds.
        let newest = self
            .voices
            .iter()
            .filter(|v| v.state == State::Playing)
            .max_by_key(|v| v.age);
        if let Some(voice) = newest {
            for (i, offset) in self.fx_offsets.iter_mut().enumerate() {
                *offset = voice.offset(FX_BASE + i);
            }
        }
        let shared = &self.shared;
        for (i, (value, offset)) in self
            .fx_values
            .iter_mut()
            .zip(self.fx_offsets.iter())
            .enumerate()
        {
            let index = FX_BASE + i;
            let base = shared.base.get(index).copied().unwrap_or(0.0);
            *value = match (shared.specs.get(index), shared.unit.get(index)) {
                (Some(spec), Some(unit)) if *offset != 0.0 => spec.from_unit(unit + offset),
                _ => base,
            };
        }
        let ctx = ProcessContext {
            sample_rate: shared.sample_rate,
            tempo_bpm: f64::from(shared.tempo),
            beats: shared.beats,
        };
        self.rack.process(left, right, &slots, &self.fx_values, ctx);
    }
}

impl Shared {
    /// Free-running LFOs follow one shared phase per LFO.
    fn advance_free_lfos(&mut self, frames: usize) {
        let dt = frames as f32 / self.sample_rate;
        for (l, phase) in self.free_phase.iter_mut().enumerate() {
            let base = |field| self.base.get(lfo_param(l, field)).copied().unwrap_or(0.0);
            let rate = if base(lfo::SYNC) >= 0.5 {
                let beats = SYNC_RATES
                    .get(base(lfo::SYNC_RATE) as usize)
                    .map_or(1.0, |r| r.1);
                self.tempo / 60.0 / beats
            } else {
                base(lfo::RATE)
            };
            *phase = (*phase + rate * dt).fract();
        }
    }
}

fn finite(x: f32) -> f32 {
    if x.is_finite() { x } else { 0.0 }
}
