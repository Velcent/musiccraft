//! The synth: parameters, voice allocation and the block loop.

use std::sync::Arc;

use musiccraft_params::ParamSpec;

use crate::modulation::LfoShape;
use crate::noise::NoiseBank;
use crate::params::{
    self, GLIDE_MODE, LFOS, OSCS, PARAM_COUNT, POLYPHONY, SYNC_RATES, VOICE_MODE, lfo, lfo_param,
};
use crate::voice::{CONTROL, Ctx, State, Voice};
use crate::wavetable::Wavetable;

/// Most voices that play at once (the `polyphony` maximum).
pub const MAX_VOICES: usize = 32;
/// Extra slots where stolen voices fade out.
const SPARE_VOICES: usize = 8;
/// Keys a mono synth remembers.
const NOTE_STACK: usize = 16;

/// What every voice reads; kept apart from the voices so both can be
/// borrowed at once.
struct Shared {
    sample_rate: f32,
    specs: &'static [ParamSpec],
    base: Box<[f32; PARAM_COUNT]>,
    unit: Box<[f32; PARAM_COUNT]>,
    tables: [Arc<Wavetable>; OSCS],
    noise: NoiseBank,
    shapes: [LfoShape; LFOS],
    free_phase: [f32; LFOS],
    tempo: f32,
    bend: f32,
    wheel: f32,
    aftertouch: f32,
}

impl Shared {
    fn ctx(&self) -> Ctx<'_> {
        Ctx {
            specs: self.specs,
            base: &self.base,
            unit: &self.unit,
            tables: &self.tables,
            noise: &self.noise,
            shapes: &self.shapes,
            free_phase: &self.free_phase,
            sample_rate: self.sample_rate,
            tempo: self.tempo,
            bend: self.bend,
            wheel: self.wheel,
            aftertouch: self.aftertouch,
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
                noise: NoiseBank::new(),
                shapes: [LfoShape::default(); LFOS],
                free_phase: [0.0; LFOS],
                tempo: 120.0,
                bend: 0.0,
                wheel: 0.0,
                aftertouch: 0.0,
            },
            voices: (0..MAX_VOICES + SPARE_VOICES)
                .map(|_| Voice::new(sample_rate))
                .collect(),
            stack: [0; NOTE_STACK],
            stack_len: 0,
            last_pitch: None,
            age: 0,
            seed: 0x2545_f491,
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

    /// Key down. Velocity 0..1; velocity 0 is a key up.
    pub fn note_on(&mut self, key: u8, velocity: f32) {
        let velocity = finite(velocity);
        if velocity <= 0.0 {
            self.note_off(key);
            return;
        }
        let key = key.min(127);
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

    /// Key up.
    pub fn note_off(&mut self, key: u8) {
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

    /// Releases every note.
    pub fn all_notes_off(&mut self) {
        self.stack_len = 0;
        for voice in &mut self.voices {
            if voice.state == State::Playing {
                voice.release();
            }
        }
    }

    /// Silences everything at once and forgets held keys and glide.
    pub fn reset(&mut self) {
        self.stack_len = 0;
        self.last_pitch = None;
        for voice in &mut self.voices {
            voice.kill();
        }
    }

    /// Renders into `left` and `right` (overwriting them).
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32]) {
        let frames = left.len().min(right.len());
        let (left, right) = (&mut left[..frames], &mut right[..frames]);
        left.fill(0.0);
        right.fill(0.0);
        let mut start = 0;
        while start < frames {
            let end = (start + CONTROL).min(frames);
            self.shared.advance_free_lfos(end - start);
            let ctx = self.shared.ctx();
            let (Some(l), Some(r)) = (left.get_mut(start..end), right.get_mut(start..end)) else {
                break;
            };
            for voice in &mut self.voices {
                if voice.state != State::Idle {
                    voice.control(&ctx, end - start);
                    voice.render(&ctx, l, r);
                }
            }
            start = end;
        }
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
