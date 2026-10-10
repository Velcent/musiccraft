use std::sync::Arc;

use musiccraft_dsp::{ProcessContext, pan_gains};
use musiccraft_params::{PPQ, TICKS_PER_BAR};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::graph::{ChannelNode, TrackNode};
use crate::voice::{Voice, VoiceStart};
use crate::{
    EngineMessage, GARBAGE_CAPACITY, Garbage, Graph, LIVE_CAPACITY, LiveEvent, MAX_BLOCK,
    MAX_VOICES, MESSAGE_CAPACITY, Meters, NoteEvent, Sequence,
};

/// Safety cap on loop wraps handled within one block (for absurdly short loops).
const MAX_WRAPS_PER_BLOCK: usize = 64;
/// Metronome click length in seconds.
const CLICK_SECONDS: f64 = 0.03;
/// Metronome level.
const CLICK_GAIN: f32 = 0.35;

/// A synthesized metronome click: a short decaying sine, higher on the downbeat.
#[derive(Debug, Clone, Copy, Default)]
struct Click {
    /// Frames to wait before the click starts (within the current block).
    delay: Option<u32>,
    accent: bool,
    /// Frames already played; `None` when silent.
    played: Option<u32>,
}

impl Click {
    fn trigger(&mut self, delay: u32, accent: bool) {
        self.delay = Some(delay);
        self.accent = accent;
    }

    /// Adds the click into a block.
    fn render(&mut self, left: &mut [f32], right: &mut [f32], sample_rate: f64) {
        let length = (CLICK_SECONDS * sample_rate) as u32;
        let hz = if self.accent { 1_760.0 } else { 1_320.0 };
        for (i, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            if self.delay == Some(i as u32) {
                self.delay = None;
                self.played = Some(0);
            }
            let Some(n) = self.played else {
                continue;
            };
            if n >= length {
                self.played = None;
                continue;
            }
            let t = f64::from(n) / sample_rate;
            let env = (-t / (CLICK_SECONDS / 5.0)).exp() as f32;
            let s = (t * hz * std::f64::consts::TAU).sin() as f32 * env * CLICK_GAIN;
            *l += s;
            *r += s;
            self.played = Some(n + 1);
        }
        // A delay past this block's end carries into the next block.
        if let Some(delay) = self.delay {
            self.delay = Some(delay.saturating_sub(left.len() as u32));
        }
    }
}

/// Main-thread side of an engine: sends messages, drops returned values.
pub struct EngineHandle {
    messages: Producer<EngineMessage>,
    garbage: Consumer<Garbage>,
    live: Option<LiveSender>,
    meters: Arc<Meters>,
    sample_rate: u32,
}

impl EngineHandle {
    /// Queues a message for the engine.
    ///
    /// # Errors
    ///
    /// Returns the message back if the queue is full.
    pub fn send(&mut self, message: EngineMessage) -> Result<(), EngineMessage> {
        self.messages
            .push(message)
            .map_err(|rtrb::PushError::Full(m)| m)
    }

    /// Drops every value the engine has handed back. Call regularly.
    pub fn collect_garbage(&mut self) -> usize {
        let mut count = 0;
        while let Ok(item) = self.garbage.pop() {
            drop(item);
            count += 1;
        }
        count
    }

    /// Free space in the message queue.
    #[must_use]
    pub fn free_slots(&self) -> usize {
        self.messages.slots()
    }

    /// Shared meters published by the engine.
    #[must_use]
    pub fn meters(&self) -> &Arc<Meters> {
        &self.meters
    }

    /// The engine's sample rate.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Takes the live-note sender (for a MIDI thread). Returns `None` after the
    /// first call.
    pub fn take_live_sender(&mut self) -> Option<LiveSender> {
        self.live.take()
    }
}

/// Sends live notes straight to the engine from any one thread.
pub struct LiveSender {
    producer: Producer<LiveEvent>,
}

impl LiveSender {
    /// Queues a live event. Returns `false` if the queue is full.
    pub fn send(&mut self, event: LiveEvent) -> bool {
        self.producer.push(event).is_ok()
    }
}

/// The audio-thread side of the engine.
pub struct Engine {
    messages: Consumer<EngineMessage>,
    garbage: Producer<Garbage>,
    live: Consumer<LiveEvent>,
    meters: Arc<Meters>,
    sample_rate: f64,
    tempo_bpm: f64,
    playing: bool,
    position: f64,
    cursor: usize,
    frames_played: u64,
    graph: Option<Box<Graph>>,
    sequence: Option<Box<Sequence>>,
    voices: Vec<Voice>,
    voice_counter: u64,
    metronome: bool,
    click: Click,
    preview: Option<Box<crate::Preview>>,
}

/// Creates an engine and the handle that controls it. Call on the main thread,
/// then move the [`Engine`] into the audio callback.
#[must_use]
pub fn engine_pair(sample_rate: u32) -> (Engine, EngineHandle) {
    let sample_rate = sample_rate.max(1);
    let (message_tx, message_rx) = RingBuffer::new(MESSAGE_CAPACITY);
    let (garbage_tx, garbage_rx) = RingBuffer::new(GARBAGE_CAPACITY);
    let (live_tx, live_rx) = RingBuffer::new(LIVE_CAPACITY);
    let meters = Arc::new(Meters::new());
    let engine = Engine {
        messages: message_rx,
        garbage: garbage_tx,
        live: live_rx,
        meters: Arc::clone(&meters),
        sample_rate: f64::from(sample_rate),
        tempo_bpm: 130.0,
        playing: false,
        position: 0.0,
        cursor: 0,
        frames_played: 0,
        graph: None,
        sequence: None,
        voices: (0..MAX_VOICES)
            .map(|_| Voice::new(sample_rate as f32))
            .collect(),
        voice_counter: 0,
        metronome: false,
        click: Click::default(),
        preview: None,
    };
    let handle = EngineHandle {
        messages: message_tx,
        garbage: garbage_rx,
        live: Some(LiveSender { producer: live_tx }),
        meters,
        sample_rate,
    };
    (engine, handle)
}

impl Engine {
    /// Whether the playhead is advancing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playing
    }

    /// Current tempo in beats per minute.
    #[must_use]
    pub fn tempo_bpm(&self) -> f64 {
        self.tempo_bpm
    }

    /// Playhead position in beats.
    #[must_use]
    pub fn position_beats(&self) -> f64 {
        self.position / f64::from(PPQ)
    }

    /// Playhead position in ticks.
    #[must_use]
    pub fn position_ticks(&self) -> f64 {
        self.position
    }

    /// Total frames rendered while playing.
    #[must_use]
    pub fn frames_played(&self) -> u64 {
        self.frames_played
    }

    /// Number of sounding voices.
    #[must_use]
    pub fn active_voices(&self) -> usize {
        self.voices.iter().filter(|v| v.active).count()
    }

    fn ticks_per_frame(&self) -> f64 {
        self.tempo_bpm / 60.0 * f64::from(PPQ) / self.sample_rate
    }

    fn loop_length(&self) -> f64 {
        match self.sequence.as_deref() {
            Some(seq) if seq.length() > 0 => seq.length() as f64,
            _ => f64::from(TICKS_PER_BAR),
        }
    }

    fn resync_cursor(&mut self) {
        self.cursor = self
            .sequence
            .as_deref()
            .map_or(0, |seq| seq.first_at_or_after(self.position));
    }

    fn release_all(&mut self, force: bool) {
        for voice in &mut self.voices {
            if voice.active {
                voice.release(force);
            }
        }
    }

    fn discard(&mut self, garbage: Garbage) {
        // `drain_messages` guarantees a free slot before applying a message that
        // produces garbage, so this push cannot fail when called from there.
        if let Err(rtrb::PushError::Full(item)) = self.garbage.push(garbage) {
            // Unreachable in practice; never drop on the audio thread.
            std::mem::forget(item);
        }
    }

    /// Applies one message immediately.
    pub fn apply(&mut self, message: EngineMessage) {
        match message {
            EngineMessage::Play => {
                if !self.playing {
                    self.playing = true;
                    self.resync_cursor();
                }
            }
            EngineMessage::Stop => {
                self.playing = false;
                self.position = 0.0;
                self.cursor = 0;
                self.release_all(true);
            }
            EngineMessage::Pause => {
                self.playing = false;
                self.release_all(true);
            }
            EngineMessage::SetTempo(bpm) => {
                if bpm.is_finite() && bpm > 0.0 {
                    self.tempo_bpm = bpm;
                }
            }
            EngineMessage::Seek(ticks) => {
                if ticks.is_finite() {
                    self.position = ticks.max(0.0) % self.loop_length();
                    self.resync_cursor();
                }
            }
            EngineMessage::Channel { channel, params } => {
                if let Some(node) = self
                    .graph
                    .as_deref_mut()
                    .and_then(|g| g.channels.get_mut(usize::from(channel)))
                {
                    node.params = params;
                }
            }
            EngineMessage::Track { track, params } => {
                if let Some(node) = self
                    .graph
                    .as_deref_mut()
                    .and_then(|g| g.tracks.get_mut(usize::from(track)))
                {
                    node.params = params;
                }
            }
            EngineMessage::Effect {
                track,
                slot,
                params,
            } => {
                if let Some(slot) = self
                    .graph
                    .as_deref_mut()
                    .and_then(|g| g.tracks.get_mut(usize::from(track)))
                    .and_then(|t| t.effects.get_mut(usize::from(slot)))
                {
                    slot.params = params;
                }
            }
            EngineMessage::NoteOn {
                channel,
                key,
                velocity,
            } => self.live_note_on(channel, key, velocity),
            EngineMessage::NoteOff { channel, key } => self.live_note_off(channel, key),
            EngineMessage::AllNotesOff => self.release_all(true),
            EngineMessage::SetMetronome(on) => self.metronome = on,
            EngineMessage::PlayPreview(preview) => {
                if let Some(old) = self.preview.replace(preview) {
                    self.discard(Garbage::Preview(old));
                }
            }
            EngineMessage::StopPreview => {
                if let Some(preview) = self.preview.as_deref_mut() {
                    preview.position = f64::INFINITY;
                }
            }
            EngineMessage::SetGraph(graph) => {
                for voice in &mut self.voices {
                    voice.active = false;
                }
                if let Some(old) = self.graph.replace(graph) {
                    self.discard(Garbage::Graph(old));
                }
            }
            EngineMessage::SetSequence(sequence) => {
                let old = self.sequence.replace(sequence);
                let length = self.loop_length();
                if self.position >= length {
                    self.position %= length;
                }
                self.resync_cursor();
                if let Some(old) = old {
                    self.discard(Garbage::Sequence(old));
                }
            }
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(message) = self.messages.peek() {
            if message.returns_garbage() && self.garbage.is_full() {
                // Wait for the main thread to collect garbage; never drop here.
                break;
            }
            match self.messages.pop() {
                Ok(message) => self.apply(message),
                Err(_) => break,
            }
        }
        while let Ok(event) = self.live.pop() {
            match event {
                LiveEvent::NoteOn {
                    channel,
                    key,
                    velocity,
                } => self.live_note_on(channel, key, velocity),
                LiveEvent::NoteOff { channel, key } => self.live_note_off(channel, key),
            }
        }
    }

    fn live_note_on(&mut self, channel: u16, key: u8, velocity: f32) {
        let start = VoiceStart {
            channel,
            key,
            velocity,
            delay: 0,
            gate: None,
            live: true,
        };
        if let Some(graph) = self.graph.as_deref() {
            trigger(
                &mut self.voices,
                &mut self.voice_counter,
                &graph.channels,
                start,
            );
        }
    }

    fn live_note_off(&mut self, channel: u16, key: u8) {
        for voice in &mut self.voices {
            if voice.active && voice.live && voice.channel == channel && voice.key == key {
                voice.release(true);
            }
        }
    }

    /// Fills an interleaved output buffer with `channels` channels. The first two
    /// channels receive left and right; any others are silent; mono receives the
    /// average of left and right.
    pub fn process(&mut self, output: &mut [f32], channels: usize) {
        let started = std::time::Instant::now();
        self.drain_messages();
        output.fill(0.0);
        if channels == 0 {
            return;
        }
        for block in output.chunks_mut(channels * MAX_BLOCK) {
            let frames = block.len() / channels;
            self.render_block(frames);
            self.write_block(block, channels, frames);
            if let Some(master) = self.graph.as_deref().and_then(|g| g.tracks.first()) {
                let mono = master.left.iter().zip(&master.right).take(frames);
                self.meters.push_scope(mono.map(|(l, r)| (l + r) * 0.5));
            }
        }
        let budget = (output.len() / channels) as f64 / self.sample_rate;
        if budget > 0.0 {
            self.meters
                .set_load((started.elapsed().as_secs_f64() / budget) as f32);
        }
        let voices = self.voices.iter().filter(|v| v.active).count() as u32;
        self.meters.publish(self.position, self.playing, voices);
    }

    fn write_block(&self, block: &mut [f32], channels: usize, frames: usize) {
        let Some(master) = self.graph.as_deref().and_then(|g| g.tracks.first()) else {
            return;
        };
        let frames_out = block.chunks_mut(channels).take(frames);
        for ((frame, l), r) in frames_out.zip(&master.left).zip(&master.right) {
            let (l, r) = (sanitize(*l), sanitize(*r));
            match frame {
                [mono] => *mono = (l + r) * 0.5,
                [left, right, ..] => {
                    *left = l;
                    *right = r;
                }
                [] => {}
            }
        }
    }

    fn render_block(&mut self, frames: usize) {
        let frames = frames.min(MAX_BLOCK);
        if self.playing {
            self.schedule(frames);
            self.frames_played += frames as u64;
        }
        let sample_rate = self.sample_rate as f32;
        let ctx = ProcessContext {
            sample_rate,
            tempo_bpm: self.tempo_bpm,
        };
        let Some(graph) = self.graph.as_deref_mut() else {
            return;
        };
        let Graph { channels, tracks } = graph;
        for node in channels.iter_mut() {
            clear(&mut node.left, frames);
            clear(&mut node.right, frames);
        }
        for track in tracks.iter_mut() {
            clear(&mut track.left, frames);
            clear(&mut track.right, frames);
        }
        for voice in self.voices.iter_mut().filter(|v| v.active) {
            match channels.get_mut(usize::from(voice.channel)) {
                Some(ChannelNode {
                    instrument,
                    params,
                    left,
                    right,
                }) => {
                    let (l, r) = (prefix(left, frames), prefix(right, frames));
                    voice.render(instrument, params, l, r, sample_rate);
                }
                None => voice.active = false,
            }
        }
        mix_channels(channels, tracks, frames);
        mix_tracks(tracks, frames, ctx, &self.meters);
        if let Some(master) = tracks.first_mut() {
            let TrackNode { left, right, .. } = master;
            let (l, r) = (prefix(left, frames), prefix(right, frames));
            self.click.render(l, r, self.sample_rate);
            if let Some(preview) = self.preview.as_deref_mut() {
                preview.render(l, r, self.sample_rate);
            }
        }
    }

    /// Starts every sequence event that falls inside the next `frames` frames and
    /// advances the playhead, wrapping at the loop end.
    fn schedule(&mut self, frames: usize) {
        let tpf = self.ticks_per_frame();
        let length = self.loop_length();
        let total = frames as f64;
        let mut done = 0.0;
        let mut position = self.position;
        for _ in 0..MAX_WRAPS_PER_BLOCK {
            let segment_end = position + (total - done) * tpf;
            let wraps = segment_end >= length;
            let stop = if wraps { length } else { segment_end };
            if self.metronome {
                let ppq = f64::from(PPQ);
                let mut beat = (position / ppq).ceil() * ppq;
                while beat < stop {
                    let offset = done + (beat - position) / tpf;
                    let accent = (beat as u64) % u64::from(TICKS_PER_BAR) == 0;
                    self.click
                        .trigger((offset - 1e-6).ceil().max(0.0) as u32, accent);
                    beat += ppq;
                }
            }
            if let (Some(seq), Some(graph)) = (self.sequence.as_deref(), self.graph.as_deref()) {
                let events = seq.events();
                while let Some(event) = events.get(self.cursor) {
                    let start = event.start as f64;
                    if start >= stop {
                        break;
                    }
                    if start >= position {
                        let offset = done + (start - position) / tpf;
                        let start = event_start(event, offset, tpf);
                        trigger(
                            &mut self.voices,
                            &mut self.voice_counter,
                            &graph.channels,
                            start,
                        );
                    }
                    self.cursor += 1;
                }
            }
            if !wraps {
                position = segment_end;
                break;
            }
            done += (length - position) / tpf;
            position = 0.0;
            self.cursor = 0;
            if done >= total {
                break;
            }
        }
        self.position = position;
    }
}

fn event_start(event: &NoteEvent, offset_frames: f64, tpf: f64) -> VoiceStart {
    VoiceStart {
        channel: event.channel,
        key: event.key,
        velocity: event.velocity,
        // First frame at or after the event time, tolerant of rounding error.
        delay: (offset_frames - 1e-6).ceil().max(0.0) as u32,
        gate: Some((event.length as f64 / tpf).max(1.0) as u64),
        live: false,
    }
}

fn trigger(voices: &mut [Voice], counter: &mut u64, channels: &[ChannelNode], start: VoiceStart) {
    let Some(node) = channels.get(usize::from(start.channel)) else {
        return;
    };
    let index = voices.iter().position(|v| !v.active).or_else(|| {
        voices
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.age)
            .map(|(i, _)| i)
    });
    if let Some(voice) = index.and_then(|i| voices.get_mut(i)) {
        *counter += 1;
        voice.start(start, &node.instrument, &node.params, *counter);
    }
}

fn mix_channels(channels: &[ChannelNode], tracks: &mut [TrackNode], frames: usize) {
    let track_count = tracks.len();
    for node in channels {
        if node.params.mute {
            continue;
        }
        let target = usize::from(node.params.track);
        let target = if target < track_count { target } else { 0 };
        let Some(track) = tracks.get_mut(target) else {
            continue;
        };
        let (pl, pr) = pan_gains(node.params.pan);
        let gl = node.params.volume * pl;
        let gr = node.params.volume * pr;
        add_scaled(&mut track.left, &node.left, gl, frames);
        add_scaled(&mut track.right, &node.right, gr, frames);
    }
}

/// Processes inserts in index order, then the master. Sends only go to
/// higher-numbered inserts, so every track has received all of its input by the
/// time it is processed.
fn mix_tracks(tracks: &mut [TrackNode], frames: usize, ctx: ProcessContext, meters: &Meters) {
    for i in 1..tracks.len() {
        let (before, rest) = tracks.split_at_mut(i);
        let Some((track, after)) = rest.split_first_mut() else {
            continue;
        };
        process_track(track, frames, ctx);
        meters.record_peak(i, peak(&track.left, frames), peak(&track.right, frames));
        if track.params.to_master {
            if let Some(master) = before.first_mut() {
                add_scaled(&mut master.left, &track.left, 1.0, frames);
                add_scaled(&mut master.right, &track.right, 1.0, frames);
            }
        }
        for send in track.params.active_sends() {
            let target = usize::from(send.target);
            let Some(dest) = target.checked_sub(i + 1).and_then(|j| after.get_mut(j)) else {
                continue;
            };
            add_scaled(&mut dest.left, &track.left, send.level, frames);
            add_scaled(&mut dest.right, &track.right, send.level, frames);
        }
    }
    if let Some(master) = tracks.first_mut() {
        process_track(master, frames, ctx);
        meters.record_peak(0, peak(&master.left, frames), peak(&master.right, frames));
    }
}

fn process_track(track: &mut TrackNode, frames: usize, ctx: ProcessContext) {
    let TrackNode {
        params,
        effects,
        left,
        right,
    } = track;
    let (l, r) = (prefix(left, frames), prefix(right, frames));
    for slot in effects.iter_mut().filter(|s| s.params.enabled) {
        slot.effect.process(l, r, &slot.params.values, ctx);
    }
    let (pl, pr) = pan_gains(params.pan);
    let volume = if params.mute { 0.0 } else { params.volume };
    for s in l.iter_mut() {
        *s *= volume * pl;
    }
    for s in r.iter_mut() {
        *s *= volume * pr;
    }
}

fn prefix(buffer: &mut [f32], frames: usize) -> &mut [f32] {
    let len = buffer.len().min(frames);
    buffer.get_mut(..len).unwrap_or_default()
}

fn clear(buffer: &mut [f32], frames: usize) {
    prefix(buffer, frames).fill(0.0);
}

fn add_scaled(dest: &mut [f32], src: &[f32], gain: f32, frames: usize) {
    for (d, s) in dest.iter_mut().zip(src).take(frames) {
        *d += *s * gain;
    }
}

fn peak(buffer: &[f32], frames: usize) -> f32 {
    buffer.iter().take(frames).fold(0.0, |p, s| p.max(s.abs()))
}

fn sanitize(sample: f32) -> f32 {
    if sample.is_finite() {
        sample.clamp(-4.0, 4.0)
    } else {
        0.0
    }
}
