use std::sync::Arc;

use musiccraft_dsp::ProcessContext;
use musiccraft_params::{PPQ, TICKS_PER_BAR};
use rtrb::{Consumer, Producer, RingBuffer};

use crate::graph::{ChannelNode, Instrument, TrackNode};
use crate::voice::{Voice, VoiceStart};
use crate::{
    EngineMessage, External, ExternalTransport, GARBAGE_CAPACITY, Garbage, Graph, LIVE_CAPACITY,
    LiveEvent, MAX_BLOCK, MAX_EXTERNALS, MAX_VOICES, MESSAGE_CAPACITY, Meters, NoteEvent, Sequence,
};

/// The engine's external processor slots.
type Externals = [Option<Box<dyn External>>];

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
    // Messages are fixed-size values stored inline in the queue; handing a
    // rejected one back by value is the point, so its size is fine here.
    #[allow(clippy::result_large_err)]
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
    externals: Vec<Option<Box<dyn External>>>,
    /// Beats elapsed on the free-running clock (used while stopped).
    free_beats: f64,
    /// Pitch of each channel's last note, for portamento notes.
    last_keys: Vec<Option<f32>>,
}

impl Drop for Engine {
    fn drop(&mut self) {
        // Let external processors end processing on the thread that ran them
        // (or the one tearing the engine down, which is now the only one).
        for external in self.externals.iter_mut().flatten() {
            external.stop();
        }
    }
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
        externals: (0..MAX_EXTERNALS).map(|_| None).collect(),
        free_beats: 0.0,
        last_keys: vec![None; PORTA_CHANNELS],
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

    /// Where the loop restarts (and Stop returns to), in ticks.
    fn loop_start(&self) -> f64 {
        let start = self.sequence.as_deref().map_or(0, Sequence::loop_start) as f64;
        if start < self.loop_length() {
            start
        } else {
            0.0
        }
    }

    /// Folds a position into the loop.
    fn fold(&self, position: f64) -> f64 {
        let (start, end) = (self.loop_start(), self.loop_length());
        if position < end {
            position
        } else {
            start + (position - start) % (end - start).max(1.0)
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
        for external in self.externals.iter_mut().flatten() {
            external.all_notes_off();
        }
    }

    /// Whether external slot `slot` holds a processor.
    #[must_use]
    pub fn has_external(&self, slot: usize) -> bool {
        self.externals.get(slot).is_some_and(Option::is_some)
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
                self.position = self.loop_start();
                self.resync_cursor();
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
                    self.position = self.fold(ticks.max(0.0));
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
            EngineMessage::SetExternal { slot, external } => {
                if let Some(entry) = self.externals.get_mut(usize::from(slot)) {
                    let old = std::mem::replace(entry, external);
                    if let Some(mut old) = old {
                        old.stop();
                        self.discard(Garbage::External(old));
                    }
                } else if let Some(external) = external {
                    // Out of range: hand it straight back.
                    self.discard(Garbage::External(external));
                }
            }
            EngineMessage::ExternalParam { slot, id, value } => {
                if let Some(Some(external)) = self.externals.get_mut(usize::from(slot)) {
                    external.set_param(id, value);
                }
            }
            EngineMessage::SetGraph(graph) => {
                for voice in &mut self.voices {
                    voice.active = false;
                }
                for external in self.externals.iter_mut().flatten() {
                    external.all_notes_off();
                }
                if let Some(old) = self.graph.replace(graph) {
                    self.discard(Garbage::Graph(old));
                }
            }
            EngineMessage::SetSequence(sequence) => {
                let old = self.sequence.replace(sequence);
                self.position = self.fold(self.position);
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
            shape: crate::NoteShape::default(),
            slide: false,
            channel,
            key,
            velocity,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from: None,
        };
        if let Some(graph) = self.graph.as_deref() {
            trigger(
                &mut self.voices,
                &mut self.voice_counter,
                &graph.channels,
                &mut self.externals,
                &mut self.last_keys,
                start,
            );
        }
    }

    fn live_note_off(&mut self, channel: u16, key: u8) {
        // A layer releases the notes it started on its channels.
        let mut targets = [channel; MAX_LAYER + 1];
        let mut count = 1;
        if let Some(Instrument::Layer(children)) = self
            .graph
            .as_deref()
            .and_then(|g| g.channels.get(usize::from(channel)))
            .map(|n| &n.instrument)
        {
            for child in children.iter().take(MAX_LAYER) {
                if let Some(slot) = targets.get_mut(count) {
                    *slot = *child;
                    count += 1;
                }
            }
        }
        for target in targets.iter().take(count).copied() {
            if let Some(external) = self.channel_external(target) {
                external.note_off(0, key);
            }
            for voice in &mut self.voices {
                if voice.active && voice.live && voice.channel == target && voice.key == key {
                    voice.release(true);
                }
            }
        }
    }

    /// The external processor playing `channel`, if it is a plugin channel.
    fn channel_external(&mut self, channel: u16) -> Option<&mut Box<dyn External>> {
        let node = self.graph.as_deref()?.channels.get(usize::from(channel))?;
        let Instrument::External(slot) = node.instrument else {
            return None;
        };
        self.externals.get_mut(usize::from(slot))?.as_mut()
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
        let transport = ExternalTransport {
            playing: self.playing,
            tempo_bpm: self.tempo_bpm,
            position_beats: self.position_beats(),
            sample_rate: self.sample_rate,
        };
        if self.playing {
            self.schedule(frames);
            self.frames_played += frames as u64;
        }
        let sample_rate = self.sample_rate as f32;
        // Tempo-synced effects follow the song while playing and a free
        // clock at the same tempo while stopped.
        let beats = if transport.playing {
            transport.position_beats
        } else {
            self.free_beats
        };
        self.free_beats += frames as f64 * self.tempo_bpm / 60.0 / self.sample_rate;
        let ctx = ProcessContext {
            beats,
            sample_rate,
            tempo_bpm: self.tempo_bpm,
        };
        let Some(graph) = self.graph.as_deref_mut() else {
            return;
        };
        if self.playing {
            if let Some(sequence) = self.sequence.as_deref() {
                apply_automation(graph, sequence, self.position);
            }
        }
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
                    ..
                }) => {
                    let (l, r) = (prefix(left, frames), prefix(right, frames));
                    voice.render(instrument, params, l, r, sample_rate);
                }
                None => voice.active = false,
            }
        }
        for node in channels.iter_mut() {
            if let Instrument::External(slot) = node.instrument {
                if let Some(Some(external)) = self.externals.get_mut(usize::from(slot)) {
                    let (l, r) = (
                        prefix(&mut node.left, frames),
                        prefix(&mut node.right, frames),
                    );
                    external.process(l, r, &transport);
                    // A plugin's bad output must not reach effect tails.
                    scrub(l, r);
                }
            }
        }
        mix_channels(channels, tracks, frames);
        let fx = FxContext {
            frames,
            ctx,
            transport,
        };
        mix_tracks(tracks, fx, &mut self.externals, &self.meters);
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
        let loop_start = self.loop_start();
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
                            &mut self.externals,
                            &mut self.last_keys,
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
            position = loop_start;
            self.cursor = self
                .sequence
                .as_deref()
                .map_or(0, |seq| seq.first_at_or_after(loop_start));
            if done >= total {
                break;
            }
        }
        self.position = position;
    }
}

/// Writes every automation lane's current value into the graph.
fn apply_automation(graph: &mut Graph, sequence: &Sequence, position: f64) {
    use crate::AutomationTarget as T;
    for lane in sequence.automation() {
        let Some(value) = lane.value_at(position) else {
            continue;
        };
        match lane.target {
            T::ChannelVolume(c) => {
                if let Some(node) = graph.channels.get_mut(usize::from(c)) {
                    node.params.volume = value;
                }
            }
            T::ChannelPan(c) => {
                if let Some(node) = graph.channels.get_mut(usize::from(c)) {
                    node.params.pan = value;
                }
            }
            T::ChannelParam(c, p) => {
                if let Some(slot) = graph
                    .channels
                    .get_mut(usize::from(c))
                    .and_then(|n| n.params.instrument.get_mut(usize::from(p)))
                {
                    *slot = value;
                }
            }
            T::TrackVolume(t) => {
                if let Some(track) = graph.tracks.get_mut(usize::from(t)) {
                    track.params.volume = value;
                }
            }
            T::TrackPan(t) => {
                if let Some(track) = graph.tracks.get_mut(usize::from(t)) {
                    track.params.pan = value;
                }
            }
            T::EffectParam(t, s, p) => {
                if let Some(slot) = graph
                    .tracks
                    .get_mut(usize::from(t))
                    .and_then(|track| track.effects.get_mut(usize::from(s)))
                    .and_then(|slot| slot.params.values.get_mut(usize::from(p)))
                {
                    *slot = value;
                }
            }
        }
    }
}

fn event_start(event: &NoteEvent, offset_frames: f64, tpf: f64) -> VoiceStart {
    VoiceStart {
        shape: event.shape,
        channel: event.channel,
        key: event.key,
        velocity: event.velocity,
        // First frame at or after the event time, tolerant of rounding error.
        delay: (offset_frames - 1e-6).ceil().max(0.0) as u32,
        gate: Some((event.length as f64 / tpf).max(1.0) as u64),
        live: false,
        pan: event.pan,
        glide_from: None,
        slide: event.slide,
    }
}

/// Most channels whose last note is remembered for portamento.
const PORTA_CHANNELS: usize = 512;

/// Most channels one layer plays.
const MAX_LAYER: usize = 16;

fn trigger(
    voices: &mut [Voice],
    counter: &mut u64,
    channels: &[ChannelNode],
    externals: &mut Externals,
    last_keys: &mut [Option<f32>],
    mut start: VoiceStart,
) {
    // A layer passes the note on to its channels (layers do not nest).
    if let Some(ChannelNode {
        instrument: Instrument::Layer(children),
        ..
    }) = channels.get(usize::from(start.channel))
    {
        for child in children.iter().take(MAX_LAYER) {
            let nested = matches!(
                channels.get(usize::from(*child)).map(|n| &n.instrument),
                Some(Instrument::Layer(_))
            );
            if !nested {
                let mut forwarded = start;
                forwarded.channel = *child;
                trigger(voices, counter, channels, externals, last_keys, forwarded);
            }
        }
        return;
    }
    if let Some(ChannelNode {
        instrument: Instrument::External(slot),
        ..
    }) = channels.get(usize::from(start.channel))
    {
        // Plugins voice their own notes.
        if let Some(Some(external)) = externals.get_mut(usize::from(*slot)) {
            external.note_on(
                start.delay,
                start.key,
                start.velocity,
                start.pan,
                start.gate,
            );
        }
        return;
    }
    // A slide note moves the sounding note of its channel instead of
    // starting one. Instruments whose pitch is fixed when a note starts (kit
    // pads, drum synth, pluck, sliced samples) play it as a normal note.
    let slides = start.slide
        && channels
            .get(usize::from(start.channel))
            .is_some_and(|node| crate::voice::glides(&node.instrument));
    if slides {
        let held = voices
            .iter_mut()
            .filter(|v| v.channel == start.channel && v.held() && v.pitched())
            .max_by_key(|v| v.age);
        if let Some(voice) = held {
            let frames = start.gate.unwrap_or(1);
            voice.slide_to(start.key, start.delay, frames, start.gate);
            // Later portamento notes glide from where the slide ends.
            if let Some(slot) = last_keys.get_mut(usize::from(start.channel)) {
                *slot = Some(f32::from(start.key) + start.shape.fine);
            }
            return;
        }
    }
    // A portamento note glides from the channel's previous note, held or not.
    let previous = last_keys.get(usize::from(start.channel)).copied().flatten();
    if let Some(slot) = last_keys.get_mut(usize::from(start.channel)) {
        *slot = Some(f32::from(start.key) + start.shape.fine);
    }
    if start.shape.porta {
        start.glide_from = previous;
        let Some(node) = channels.get(usize::from(start.channel)) else {
            return;
        };
        if let Some(voice) = free_voice(voices) {
            *counter += 1;
            voice.start(start, &node.instrument, &node.params, *counter);
        }
        return;
    }
    // A held note on the same channel is where a glide starts from.
    start.glide_from = voices
        .iter()
        .filter(|v| v.channel == start.channel && v.held())
        .max_by_key(|v| v.age)
        .map(Voice::current_key);
    let Some(node) = channels.get(usize::from(start.channel)) else {
        return;
    };
    if let Some(voice) = free_voice(voices) {
        *counter += 1;
        voice.start(start, &node.instrument, &node.params, *counter);
    }
}

/// An idle voice, or the oldest one to steal.
fn free_voice(voices: &mut [Voice]) -> Option<&mut Voice> {
    let index = voices.iter().position(|v| !v.active).or_else(|| {
        voices
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.age)
            .map(|(i, _)| i)
    })?;
    voices.get_mut(index)
}

fn mix_channels(channels: &mut [ChannelNode], tracks: &mut [TrackNode], frames: usize) {
    let track_count = tracks.len();
    for node in channels.iter_mut() {
        let from = node.gains;
        let to = node.params.gains();
        node.gains = to;
        if from == (0.0, 0.0) && to == (0.0, 0.0) {
            continue;
        }
        let target = usize::from(node.params.track);
        let target = if target < track_count { target } else { 0 };
        let Some(track) = tracks.get_mut(target) else {
            continue;
        };
        add_ramped(&mut track.left, &node.left, from.0, to.0, frames);
        add_ramped(&mut track.right, &node.right, from.1, to.1, frames);
    }
}

/// Adds `src * gain` into `dest`, the gain moving linearly from `from` to `to`
/// across the block so changes never click.
fn add_ramped(dest: &mut [f32], src: &[f32], from: f32, to: f32, frames: usize) {
    if from == to {
        add_scaled(dest, src, to, frames);
        return;
    }
    let step = (to - from) / frames.max(1) as f32;
    let mut gain = from;
    for (d, s) in dest.iter_mut().zip(src).take(frames) {
        gain += step;
        *d += *s * gain;
    }
}

/// Multiplies `buffer` by a gain ramping from `from` to `to`.
fn scale_ramped(buffer: &mut [f32], from: f32, to: f32) {
    let step = (to - from) / buffer.len().max(1) as f32;
    let mut gain = from;
    for s in buffer.iter_mut() {
        gain += step;
        *s *= gain;
    }
}

/// What effect processing needs to know about the current block.
#[derive(Clone, Copy)]
struct FxContext {
    frames: usize,
    ctx: ProcessContext,
    transport: ExternalTransport,
}

/// Processes inserts in index order, then the master. Sends only go to
/// higher-numbered inserts, so every track has received all of its input by the
/// time it is processed.
fn mix_tracks(tracks: &mut [TrackNode], fx: FxContext, externals: &mut Externals, meters: &Meters) {
    let frames = fx.frames;
    for i in 1..tracks.len() {
        let (before, rest) = tracks.split_at_mut(i);
        let Some((track, after)) = rest.split_first_mut() else {
            continue;
        };
        // Inserts below this one are finished and can key its compressors.
        process_track(track, fx, externals, before, 0);
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
    if let Some((master, inserts)) = tracks.split_first_mut() {
        process_track(master, fx, externals, inserts, 1);
        meters.record_peak(0, peak(&master.left, frames), peak(&master.right, frames));
    }
}

/// Runs a track's effects and fader. `keys` are already-processed tracks that
/// may feed effect sidechains; `keys[i]` is mixer track `i + key_offset`.
fn process_track(
    track: &mut TrackNode,
    fx: FxContext,
    externals: &mut Externals,
    keys: &[TrackNode],
    key_offset: usize,
) {
    let FxContext {
        frames,
        ctx,
        transport,
    } = fx;
    let TrackNode {
        params,
        gains,
        effects,
        left,
        right,
    } = track;
    let (l, r) = (prefix(left, frames), prefix(right, frames));
    // Invalid samples would otherwise circulate in reverb and delay feedback
    // forever, silencing the track until the graph is rebuilt.
    scrub(l, r);
    for slot in effects.iter_mut().filter(|s| s.params.enabled) {
        if let Some(index) = slot.external {
            if let Some(Some(external)) = externals.get_mut(usize::from(index)) {
                external.process(l, r, &transport);
                scrub(l, r);
            }
            continue;
        }
        let source = slot
            .effect
            .kind()
            .sidechain_index()
            .and_then(|i| slot.params.values.get(i))
            .copied()
            .unwrap_or(0.0);
        let key = match source {
            source if source >= 0.5 => (source.round() as usize)
                .checked_sub(key_offset)
                .and_then(|i| keys.get(i))
                .map(|t| {
                    let n = frames.min(t.left.len()).min(t.right.len());
                    (
                        t.left.get(..n).unwrap_or_default(),
                        t.right.get(..n).unwrap_or_default(),
                    )
                }),
            _ => None,
        };
        slot.effect
            .process_keyed(l, r, &slot.params.values, ctx, key);
        if scrub(l, r) {
            // The effect's own state went bad; start it afresh.
            slot.effect.reset();
        }
    }
    let from = *gains;
    let to = params.gains();
    *gains = to;
    scale_ramped(l, from.0, to.0);
    scale_ramped(r, from.1, to.1);
}

/// Replaces non-finite samples with silence. Returns whether any were found.
fn scrub(left: &mut [f32], right: &mut [f32]) -> bool {
    let mut found = false;
    for sample in left.iter_mut().chain(right.iter_mut()) {
        if !sample.is_finite() {
            *sample = 0.0;
            found = true;
        }
    }
    found
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
