use musiccraft_dsp::{Adsr, AdsrParams, Oscillator, Svf, SvfMode, Waveform, key_to_hz, pan_gains};
use musiccraft_params::inst;

use crate::{ChannelParams, Instrument};

/// Output level of one synth voice at full velocity, leaving headroom for chords.
const SYNTH_GAIN: f32 = 0.25;
/// Frames between updates of modulated values (filter, pitch, LFO).
const FILTER_UPDATE_INTERVAL: u32 = 32;
/// Most unison voices per note.
const MAX_UNISON: usize = 7;
/// Length of each voice's string delay line (lowest pluck ~12 Hz at 48 kHz).
const STRING_LEN: usize = 4096;
/// Output level of the drum synth and pluck at full velocity.
const DRUM_GAIN: f32 = 0.8;
const PLUCK_GAIN: f32 = 0.35;

fn param(params: &ChannelParams, index: usize) -> f32 {
    params.instrument.get(index).copied().unwrap_or(0.0)
}

fn envelope_params(params: &ChannelParams) -> AdsrParams {
    AdsrParams {
        attack: param(params, inst::ATTACK),
        decay: param(params, inst::DECAY),
        sustain: param(params, inst::SUSTAIN),
        release: param(params, inst::RELEASE),
    }
}

/// How a voice should start.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VoiceStart {
    pub channel: u16,
    pub key: u8,
    pub velocity: f32,
    /// Frames to wait inside the current block before sounding.
    pub delay: u32,
    /// Frames to hold before releasing; `None` holds until released explicitly.
    pub gate: Option<u64>,
    pub live: bool,
    /// Note pan, -1..=1.
    pub pan: f32,
    /// Pitch (in keys) of the note this one glides from, if any.
    pub glide_from: Option<f32>,
    /// A slide note: move the sounding voice instead of starting one.
    pub slide: bool,
}

/// One sounding note.
#[derive(Debug, Clone)]
pub(crate) struct Voice {
    pub active: bool,
    pub live: bool,
    pub channel: u16,
    pub key: u8,
    pub age: u64,
    velocity: f32,
    delay: u32,
    gate: Option<u64>,
    one_shot: bool,
    /// For a sliced sample: where the slice ends, in sample frames.
    slice_end: Option<f64>,
    env: Adsr,
    osc1: [Oscillator; MAX_UNISON],
    osc2: [Oscillator; MAX_UNISON],
    sub: Oscillator,
    lfo_phase: f32,
    filter_l: Svf,
    filter_r: Svf,
    filter_countdown: u32,
    /// Frames since the voice started sounding (drum synth).
    elapsed: u32,
    /// Oscillator phase in cycles (drum synth).
    drum_phase: f64,
    noise: u32,
    /// Plucked-string delay line, its write position, the delay in frames,
    /// the last read sample and the loudest output of the current period.
    string: Vec<f32>,
    string_write: usize,
    string_delay: f32,
    string_last: f32,
    string_peak: f32,
    string_count: u32,
    sample_pos: f64,
    pan: (f32, f32),
    /// Current glide offset in keys (moves toward 0) and its change per frame.
    glide: f32,
    glide_rate: f32,
    sample_rate: f32,
}

impl Voice {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            active: false,
            live: false,
            channel: 0,
            key: 0,
            age: 0,
            velocity: 0.0,
            delay: 0,
            gate: None,
            one_shot: false,
            slice_end: None,
            env: Adsr::new(sample_rate),
            osc1: std::array::from_fn(|_| Oscillator::new()),
            osc2: std::array::from_fn(|_| Oscillator::new()),
            sub: Oscillator::new(),
            lfo_phase: 0.0,
            filter_l: Svf::new(),
            filter_r: Svf::new(),
            elapsed: 0,
            drum_phase: 0.0,
            noise: 0x9E37_79B9,
            string: vec![0.0; STRING_LEN],
            string_write: 0,
            string_delay: 100.0,
            string_last: 0.0,
            string_peak: 0.0,
            string_count: 0,
            filter_countdown: 0,
            sample_pos: 0.0,
            pan: (1.0, 1.0),
            glide: 0.0,
            glide_rate: 0.0,
            sample_rate: sample_rate.max(1.0),
        }
    }

    /// Whether the gate is still open (not releasing).
    pub fn held(&self) -> bool {
        self.active
            && self.delay == 0
            && !self.env.is_releasing()
            && (self.gate.is_some() || self.live || self.one_shot)
    }

    /// The pitch in keys the voice is sounding now (before the channel pitch).
    pub fn current_key(&self) -> f32 {
        f32::from(self.key) + self.glide
    }

    pub fn start(
        &mut self,
        start: VoiceStart,
        instrument: &Instrument,
        params: &ChannelParams,
        age: u64,
    ) {
        self.active = true;
        self.live = start.live;
        self.channel = start.channel;
        self.key = start.key;
        self.velocity = start.velocity.clamp(0.0, 1.0);
        self.delay = start.delay;
        self.gate = start.gate;
        self.age = age;
        self.one_shot = (matches!(instrument, Instrument::Sampler(_))
            && param(params, inst::ONE_SHOT) >= 0.5
            && !start.live)
            || matches!(instrument, Instrument::DrumSynth);
        self.elapsed = 0;
        self.drum_phase = 0.0;

        match instrument {
            Instrument::DrumSynth => self.start_drum(params),
            Instrument::Pluck => self.start_pluck(params, start.key),
            _ => {}
        }
        self.env.reset();
        self.env.set_params(envelope_params(params));
        self.env.gate_on();
        // Spread unison phases so stacked oscillators do not start in step.
        for (i, (a, b)) in self.osc1.iter_mut().zip(self.osc2.iter_mut()).enumerate() {
            let offset = (i as f64 * 0.618_034).fract();
            a.reset(offset);
            b.reset((offset + 0.25).fract());
        }
        self.sub.reset(0.0);
        self.lfo_phase = 0.0;
        self.filter_l.reset();
        self.filter_r.reset();
        self.filter_countdown = 0;
        self.sample_pos = 0.0;
        self.slice_end = None;
        if let Instrument::Sampler(buffer) = instrument {
            let slices = param(params, inst::SLICES).round().max(1.0) as i32;
            if slices > 1 {
                // Key root + i plays slice i (wrapping), as a one-shot.
                let root = param(params, inst::ROOT_KEY).round() as i32;
                let index = (i32::from(start.key) - root).rem_euclid(slices);
                let len = buffer.len() as f64;
                self.sample_pos = len * f64::from(index) / f64::from(slices);
                self.slice_end = Some(len * f64::from(index + 1) / f64::from(slices));
                self.one_shot = !start.live;
            }
        }
        self.pan = pan_gains(start.pan);
        let glide_time = param(params, inst::GLIDE);
        match start.glide_from {
            Some(from) if glide_time > 0.0 && matches!(instrument, Instrument::Synth) => {
                self.glide = from - f32::from(start.key);
                self.glide_rate = self.glide.abs() / (glide_time * self.env_rate());
            }
            _ => {
                self.glide = 0.0;
                self.glide_rate = 0.0;
            }
        }
    }

    fn env_rate(&self) -> f32 {
        self.sample_rate
    }

    /// Glides this voice to `key` over `frames` frames (a slide note), and
    /// keeps it sounding for `delay + gate` frames from now.
    pub fn slide_to(&mut self, key: u8, delay: u32, frames: u64, gate: Option<u64>) {
        let from = self.current_key();
        self.key = key;
        self.glide = from - f32::from(key);
        self.glide_rate = self.glide.abs() / (frames.max(1) as f32);
        if let Some(gate) = gate {
            self.gate = Some(u64::from(delay) + gate);
        }
    }

    /// Closes the gate (one-shot samples keep playing unless `force`).
    pub fn release(&mut self, force: bool) {
        if !self.one_shot || force {
            self.env.gate_off();
            self.gate = None;
            self.one_shot = false;
        }
    }

    /// Adds this voice's output for the block into `left`/`right`.
    pub fn render(
        &mut self,
        instrument: &Instrument,
        params: &ChannelParams,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: f32,
    ) {
        if !self.active {
            return;
        }
        self.env.set_params(envelope_params(params));
        let pitch = f32::from(self.key) + param(params, inst::PITCH);
        match instrument {
            Instrument::Sampler(buffer) => {
                let root = param(params, inst::ROOT_KEY);
                // Slices play at their own pitch: the key picks the slice.
                let pitch = if self.slice_end.is_some() {
                    root + param(params, inst::PITCH)
                } else {
                    pitch
                };
                let rate = buffer.sample_rate() / f64::from(sample_rate);
                let step_for =
                    |glide: f32| rate * 2.0_f64.powf(f64::from(pitch + glide - root) / 12.0);
                let mut step = step_for(self.glide);
                let end = self
                    .slice_end
                    .unwrap_or(f64::INFINITY)
                    .min(buffer.len() as f64);
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    if !self.advance_gate() {
                        continue;
                    }
                    if self.glide != 0.0 {
                        // A slide moves the pitch offset toward zero.
                        let change = self.glide_rate.min(self.glide.abs());
                        self.glide -= change * self.glide.signum();
                        step = step_for(self.glide);
                    }
                    let level = self.env.next_level() * self.velocity;
                    let (sl, sr) = buffer.read(self.sample_pos);
                    *l += sl * level * self.pan.0;
                    *r += sr * level * self.pan.1;
                    self.sample_pos += step;
                    if self.sample_pos >= end || self.env.is_idle() {
                        self.active = false;
                        break;
                    }
                }
            }
            // External instruments never get voices; see `engine::trigger`.
            Instrument::External(_) => self.active = false,
            Instrument::Synth => self.render_synth(params, pitch, left, right, sample_rate),
            Instrument::DrumSynth => self.render_drum(params, left, right),
            Instrument::Pluck => self.render_pluck(params, left, right),
        }
    }

    fn next_noise(&mut self) -> f32 {
        let mut x = self.noise;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.noise = x;
        x as f32 / u32::MAX as f32 * 2.0 - 1.0
    }

    /// Sets up the noise filter for the drum type.
    fn start_drum(&mut self, params: &ChannelParams) {
        let tone = param(params, inst::DRUM_TONE).clamp(0.0, 1.0);
        let sr = self.sample_rate;
        match param(params, inst::DRUM_TYPE).round() as i32 {
            1 => self.filter_l.set(1_800.0 + 4_000.0 * tone, 0.2, sr),
            2 => self.filter_l.set(5_000.0 + 7_000.0 * tone, 0.1, sr),
            4 => self.filter_l.set(900.0 + 1_200.0 * tone, 0.35, sr),
            _ => {}
        }
        self.filter_l.reset();
    }

    /// Drum synth: swept sine bodies, filtered noise and a click; every hit
    /// decays on its own and ignores the note's end.
    fn render_drum(&mut self, params: &ChannelParams, left: &mut [f32], right: &mut [f32]) {
        let kind = param(params, inst::DRUM_TYPE).round() as i32;
        let tune = param(params, inst::DRUM_TUNE);
        let decay = param(params, inst::DRUM_DECAY).max(0.01);
        let sweep = param(params, inst::DRUM_SWEEP);
        let tone = param(params, inst::DRUM_TONE).clamp(0.0, 1.0);
        let click = param(params, inst::DRUM_CLICK).clamp(0.0, 1.0);
        let sr = self.sample_rate;
        let octave_down = match kind {
            0 => 24.0,
            _ => 12.0,
        };
        let base = key_to_hz(f32::from(self.key) + tune - octave_down + param(params, inst::PITCH));
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            let t = self.elapsed as f32 / sr;
            self.elapsed = self.elapsed.saturating_add(1);
            let amp = (-t / decay).exp();
            let noise = self.next_noise();
            let attack = if t < 0.004 {
                noise * click * (1.0 - t / 0.004)
            } else {
                0.0
            };
            let sample = match kind {
                // Snare: a short tone body plus band-limited noise.
                1 => {
                    let f = base * 2.0_f32.powf(sweep * 0.25 * (-t / 0.03).exp() / 12.0);
                    self.drum_phase = (self.drum_phase + f64::from(f / sr)).fract();
                    let body = (self.drum_phase as f32 * std::f32::consts::TAU).sin()
                        * (-t / (decay * 0.4)).exp();
                    let rattle = self.filter_l.process(noise, SvfMode::BandPass) * 1.3;
                    body * (1.0 - tone) + rattle * tone * amp + attack
                }
                // Hat: bright noise.
                2 => self.filter_l.process(noise, SvfMode::HighPass) * amp + attack * 0.5,
                // Clap: three quick bursts, then a tail.
                4 => {
                    let bursts: f32 = (0..3)
                        .map(|k| {
                            let start = k as f32 * 0.011;
                            if t >= start {
                                (-(t - start) / 0.004).exp()
                            } else {
                                0.0
                            }
                        })
                        .sum();
                    let tail = if t >= 0.022 { amp } else { 0.0 };
                    self.filter_l.process(noise, SvfMode::BandPass) * 0.8 * (bursts + tail).min(1.2)
                }
                // Kick and tom: a sine falling in pitch.
                _ => {
                    let f = base * 2.0_f32.powf(sweep * (-t / 0.04).exp() / 12.0);
                    self.drum_phase = (self.drum_phase + f64::from(f / sr)).fract();
                    (self.drum_phase as f32 * std::f32::consts::TAU).sin() * amp + attack
                }
            };
            let out = sample * self.velocity * DRUM_GAIN;
            *l += out * self.pan.0;
            *r += out * self.pan.1;
            if t > 0.05 && amp < 1e-4 {
                self.active = false;
                break;
            }
        }
    }

    /// Fills the string with a burst of (lowpassed) noise one period long.
    fn start_pluck(&mut self, params: &ChannelParams, key: u8) {
        let tone = param(params, inst::PLUCK_TONE).clamp(0.0, 1.0);
        let hz = key_to_hz(f32::from(key) + param(params, inst::PITCH)).max(1.0);
        self.string_delay = (self.sample_rate / hz).clamp(2.0, (STRING_LEN - 4) as f32);
        let mut smooth = 0.0;
        let coeff = 0.05 + 0.95 * tone;
        for i in 0..STRING_LEN {
            let n = self.next_noise();
            smooth += (n - smooth) * coeff;
            if let Some(slot) = self.string.get_mut(i) {
                *slot = if (i as f32) < self.string_delay {
                    smooth
                } else {
                    0.0
                };
            }
        }
        self.string_write = self.string_delay.ceil() as usize % STRING_LEN;
        self.string_last = 0.0;
        self.string_peak = 1.0;
        self.string_count = 0;
    }

    /// The string's output `delay` frames back.
    fn string_read(&self, delay: f32) -> f32 {
        let len = STRING_LEN as f32;
        let position = (self.string_write as f32 - delay).rem_euclid(len);
        let whole = position as usize;
        let frac = position - whole as f32;
        let a = self.string.get(whole % STRING_LEN).copied().unwrap_or(0.0);
        let b = self
            .string
            .get((whole + 1) % STRING_LEN)
            .copied()
            .unwrap_or(0.0);
        a + (b - a) * frac
    }

    /// Plucked string (Karplus-Strong): the burst circulates through a damped
    /// delay tuned to the note.
    fn render_pluck(&mut self, params: &ChannelParams, left: &mut [f32], right: &mut [f32]) {
        let damping = param(params, inst::PLUCK_DAMPING).clamp(0.0, 1.0) * 0.5;
        let sr = self.sample_rate;
        let period = self.string_delay;
        let sustain = 0.001_f32.powf(period / (param(params, inst::PLUCK_DECAY).max(0.05) * sr));
        let release = 0.001_f32.powf(period / (0.08 * sr));
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            // The damping filter delays by `damping` frames; shorten the
            // line to match so the pitch stays true.
            let read = self.string_read((period - damping).max(1.0));
            let filtered = read * (1.0 - damping) + self.string_last * damping;
            self.string_last = read;
            let feedback = if self.env.is_releasing() {
                release
            } else {
                sustain
            };
            if let Some(slot) = self.string.get_mut(self.string_write) {
                // Each sample passes once per period, so the per-period
                // factor applies directly.
                *slot = filtered * feedback;
            }
            self.string_write = (self.string_write + 1) % STRING_LEN;
            let out = read * self.velocity * PLUCK_GAIN;
            *l += out * self.pan.0;
            *r += out * self.pan.1;
            // Stop once a whole period stays below -80 dB.
            self.string_peak = self.string_peak.max(read.abs());
            self.string_count += 1;
            if self.string_count as f32 >= period {
                if self.string_peak < 1e-4 {
                    self.active = false;
                    break;
                }
                self.string_peak = 0.0;
                self.string_count = 0;
            }
        }
    }

    /// The subtractive synth: up to seven detuned unison pairs of oscillators
    /// spread across the stereo field, a sub oscillator an octave below, a
    /// drive stage, a filter per side and an LFO on pitch, cutoff or volume.
    fn render_synth(
        &mut self,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: f32,
    ) {
        let wave1 = Waveform::from_param(param(params, inst::WAVE1));
        let wave2 = Waveform::from_param(param(params, inst::WAVE2));
        let mix = param(params, inst::OSC_MIX).clamp(0.0, 1.0);
        let detune = 2.0_f64.powf(f64::from(param(params, inst::DETUNE)) / 1200.0);
        let sr = f64::from(sample_rate);
        let cutoff = param(params, inst::CUTOFF);
        let resonance = param(params, inst::RESONANCE);
        let env_amount = param(params, inst::ENV_AMOUNT);
        let unison = (param(params, inst::UNISON).round() as usize).clamp(1, MAX_UNISON);
        let spread = param(params, inst::SPREAD);
        let sub_level = param(params, inst::SUB).clamp(0.0, 1.0);
        let lfo_step = param(params, inst::LFO_RATE).max(0.0) / sample_rate.max(1.0);
        let lfo_depth = param(params, inst::LFO_DEPTH).clamp(0.0, 1.0);
        let lfo_target = param(params, inst::LFO_TARGET).round() as i32;
        let mode = match param(params, inst::FILTER_TYPE).round() as i32 {
            1 => SvfMode::BandPass,
            2 => SvfMode::HighPass,
            _ => SvfMode::LowPass,
        };
        let drive = param(params, inst::DRIVE).clamp(0.0, 1.0);
        let drive_gain = 1.0 + drive * 8.0;
        let drive_norm = 1.0 / drive_gain.tanh();
        // Unison voices sit evenly across -1..1; the stack is normalized so
        // more voices are not louder.
        let position = |i: usize| {
            if unison == 1 {
                0.0
            } else {
                i as f32 / (unison - 1) as f32 * 2.0 - 1.0
            }
        };
        let norm = 1.0 / (unison as f32).sqrt();
        let mut lfo = 0.0;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            let level = self.env.next_level();
            if self.filter_countdown == 0 {
                if self.glide != 0.0 {
                    // Portamento: move the pitch offset toward zero.
                    let step =
                        (self.glide_rate * FILTER_UPDATE_INTERVAL as f32).min(self.glide.abs());
                    self.glide -= step * self.glide.signum();
                }
                lfo = (self.lfo_phase * std::f32::consts::TAU).sin() * lfo_depth;
                let vibrato = if lfo_target == 0 { lfo * 2.0 } else { 0.0 };
                let base = f64::from(key_to_hz(pitch + self.glide + vibrato));
                for i in 0..unison {
                    let cents = f64::from(spread * position(i));
                    let hz = base * 2.0_f64.powf(cents / 1200.0);
                    if let (Some(a), Some(b)) = (self.osc1.get_mut(i), self.osc2.get_mut(i)) {
                        a.set_frequency(hz, sr);
                        b.set_frequency(hz * detune, sr);
                    }
                }
                self.sub.set_frequency(base * 0.5, sr);
                let lfo_octaves = if lfo_target == 1 { lfo * 3.0 } else { 0.0 };
                let modulated = cutoff * 2.0_f32.powf(env_amount * level + lfo_octaves);
                self.filter_l.set(modulated, resonance, sample_rate);
                self.filter_r.set(modulated, resonance, sample_rate);
                self.filter_countdown = FILTER_UPDATE_INTERVAL;
            }
            self.filter_countdown -= 1;
            self.lfo_phase = (self.lfo_phase + lfo_step).fract();
            let (mut sum_l, mut sum_r) = (0.0, 0.0);
            for i in 0..unison {
                let (Some(a), Some(b)) = (self.osc1.get_mut(i), self.osc2.get_mut(i)) else {
                    continue;
                };
                let a = a.next_sample(wave1);
                let b = b.next_sample(wave2);
                let sample = (a + (b - a) * mix) * norm;
                let p = position(i);
                sum_l += sample * (1.0 - p);
                sum_r += sample * (1.0 + p);
            }
            let sub = self.sub.next_sample(Waveform::Sine) * sub_level;
            let (mut raw_l, mut raw_r) = (sum_l + sub, sum_r + sub);
            if drive > 0.001 {
                raw_l = (raw_l * drive_gain).tanh() * drive_norm;
                raw_r = (raw_r * drive_gain).tanh() * drive_norm;
            }
            let tremolo = if lfo_target == 2 {
                1.0 - (0.5 + 0.5 * lfo) * lfo_depth
            } else {
                1.0
            };
            let gain = level * self.velocity * SYNTH_GAIN * tremolo;
            *l += self.filter_l.process(raw_l, mode) * gain * self.pan.0;
            *r += self.filter_r.process(raw_r, mode) * gain * self.pan.1;
            if self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// Handles the start delay and the gate countdown for one frame. Returns
    /// whether the voice sounds on this frame.
    fn advance_gate(&mut self) -> bool {
        if self.delay > 0 {
            self.delay -= 1;
            return false;
        }
        if let Some(remaining) = self.gate {
            if remaining == 0 {
                self.gate = None;
                if !self.one_shot {
                    self.env.gate_off();
                }
            } else {
                self.gate = Some(remaining - 1);
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glide_moves_linearly_to_the_new_key() {
        let mut params = ChannelParams::default();
        params.instrument[inst::GLIDE] = 0.5;
        let start = VoiceStart {
            slide: false,
            channel: 0,
            key: 69,
            velocity: 1.0,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from: Some(57.0),
        };
        let mut voice = Voice::new(1_000.0);
        voice.start(start, &Instrument::Synth, &params, 1);
        assert!((voice.current_key() - 57.0).abs() < 1e-4);
        let mut l = vec![0.0; 250];
        let mut r = vec![0.0; 250];
        voice.render(&Instrument::Synth, &params, &mut l, &mut r, 1_000.0);
        // Glide moves at control rate (every 32 frames), so allow one step.
        assert!(
            (voice.current_key() - 63.0).abs() < 0.2,
            "halfway: {}",
            voice.current_key()
        );
        voice.render(&Instrument::Synth, &params, &mut l, &mut r, 1_000.0);
        voice.render(&Instrument::Synth, &params, &mut l, &mut r, 1_000.0);
        assert!(
            (voice.current_key() - 69.0).abs() < 1e-4,
            "arrived: {}",
            voice.current_key()
        );
    }

    fn render_synth_note(params: &ChannelParams) -> (Vec<f32>, Vec<f32>) {
        let start = VoiceStart {
            slide: false,
            channel: 0,
            key: 57,
            velocity: 1.0,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from: None,
        };
        let mut voice = Voice::new(48_000.0);
        voice.start(start, &Instrument::Synth, params, 1);
        let mut l = vec![0.0; 4_800];
        let mut r = vec![0.0; 4_800];
        voice.render(&Instrument::Synth, params, &mut l, &mut r, 48_000.0);
        (l, r)
    }

    fn rms(data: &[f32]) -> f32 {
        (data.iter().map(|s| s * s).sum::<f32>() / data.len() as f32).sqrt()
    }

    #[test]
    fn unison_spreads_stereo_without_getting_louder() {
        let mut params = ChannelParams::default();
        let (l1, r1) = render_synth_note(&params);
        assert!(
            l1.iter().zip(&r1).all(|(a, b)| (a - b).abs() < 1e-6),
            "one voice is centered"
        );
        params.instrument[inst::UNISON] = 5.0;
        params.instrument[inst::SPREAD] = 30.0;
        let (l5, r5) = render_synth_note(&params);
        assert!(
            l5.iter().zip(&r5).any(|(a, b)| (a - b).abs() > 1e-3),
            "unison is stereo"
        );
        let (a, b) = (rms(&l1), rms(&l5));
        assert!(b < a * 1.6 && b > a * 0.4, "levels {a} vs {b}");
        assert!(l5.iter().chain(&r5).all(|s| s.is_finite()));
    }

    #[test]
    fn sub_lfo_drive_and_filter_types_change_the_sound() {
        let base = ChannelParams::default();
        let (reference, _) = render_synth_note(&base);
        for (index, value) in [
            (inst::SUB, 1.0),
            (inst::DRIVE, 1.0),
            (inst::FILTER_TYPE, 2.0),
            (inst::LFO_DEPTH, 1.0),
        ] {
            let mut params = base;
            params.instrument[index] = value;
            if index == inst::LFO_DEPTH {
                params.instrument[inst::LFO_TARGET] = 2.0;
                params.instrument[inst::LFO_RATE] = 10.0;
            }
            let (out, _) = render_synth_note(&params);
            let diff: f32 = out.iter().zip(&reference).map(|(a, b)| (a - b).abs()).sum();
            assert!(diff > 1.0, "parameter {index} made no difference");
            assert!(out.iter().all(|s| s.is_finite() && s.abs() < 2.0));
        }
    }

    #[test]
    fn no_glide_without_a_glide_time() {
        let params = ChannelParams::default();
        let start = VoiceStart {
            slide: false,
            channel: 0,
            key: 69,
            velocity: 1.0,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from: Some(57.0),
        };
        let mut voice = Voice::new(1_000.0);
        voice.start(start, &Instrument::Synth, &params, 1);
        assert!((voice.current_key() - 69.0).abs() < 1e-6);
    }
}

#[cfg(test)]
mod instrument_tests {
    use super::*;

    fn render(
        instrument: &Instrument,
        params: &ChannelParams,
        key: u8,
        frames: usize,
        gate: Option<u64>,
    ) -> Vec<f32> {
        let start = VoiceStart {
            channel: 0,
            key,
            velocity: 1.0,
            delay: 0,
            gate,
            live: false,
            pan: 0.0,
            glide_from: None,
            slide: false,
        };
        let mut voice = Voice::new(48_000.0);
        voice.start(start, instrument, params, 1);
        let mut l = vec![0.0; frames];
        let mut r = vec![0.0; frames];
        for (bl, br) in l.chunks_mut(256).zip(r.chunks_mut(256)) {
            voice.render(instrument, params, bl, br, 48_000.0);
        }
        l
    }

    fn peak(data: &[f32]) -> f32 {
        data.iter().fold(0.0, |p, s| p.max(s.abs()))
    }

    /// Pitch by autocorrelation: the lag (20–1200 frames) that best matches
    /// the signal with itself.
    fn autocorrelation_hz(data: &[f32]) -> f32 {
        let score = |lag: usize| -> f32 { data.iter().zip(&data[lag..]).map(|(a, b)| a * b).sum() };
        let lag = (20..1_200)
            .max_by(|a, b| score(*a).total_cmp(&score(*b)))
            .unwrap_or(1);
        48_000.0 / lag as f32
    }

    fn crossings_hz(data: &[f32]) -> f32 {
        let n = data
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        n as f32 * 48_000.0 / data.len() as f32
    }

    #[test]
    fn every_drum_type_hits_and_decays() {
        for kind in 0..5 {
            let mut params = ChannelParams::default();
            params.instrument[inst::DRUM_TYPE] = kind as f32;
            params.instrument[inst::DRUM_DECAY] = 0.1;
            let out = render(&Instrument::DrumSynth, &params, 60, 48_000, Some(10));
            let start = peak(&out[..4_800]);
            assert!(start > 0.05 && start < 2.0, "type {kind} peak {start}");
            assert!(peak(&out[40_000..]) < 1e-3, "type {kind} decays");
            assert!(out.iter().all(|s| s.is_finite()));
        }
    }

    #[test]
    fn kick_sweeps_down_to_its_tuned_pitch() {
        let params = ChannelParams::default();
        let out = render(&Instrument::DrumSynth, &params, 60, 24_000, None);
        // C5 two octaves down is C3, about 65 Hz, once the sweep settles.
        let settled = crossings_hz(&out[9_600..19_200]);
        assert!((settled - 65.4).abs() < 6.0, "kick at {settled} Hz");
        let early = crossings_hz(&out[..1_200]);
        assert!(early > settled * 2.0, "starts higher: {early}");
    }

    #[test]
    fn pluck_is_in_tune_decays_and_damps_on_release() {
        let mut params = ChannelParams::default();
        params.instrument[inst::PLUCK_DECAY] = 1.0;
        let out = render(&Instrument::Pluck, &params, 69, 96_000, None);
        let hz = autocorrelation_hz(&out[4_800..14_400]);
        assert!((hz - 440.0).abs() < 8.0, "pluck at {hz} Hz");
        let early = peak(&out[..4_800]);
        let late = peak(&out[48_000..52_800]);
        assert!(
            early > 0.1 && late < early * 0.01,
            "decays 60 dB in a second: {early} -> {late}"
        );
        // Ending the note damps it fast.
        let short = render(&Instrument::Pluck, &params, 69, 24_000, Some(4_800));
        assert!(peak(&short[14_400..]) < 1e-3);
    }
}
