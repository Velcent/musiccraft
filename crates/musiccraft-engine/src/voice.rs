use musiccraft_dsp::{Adsr, AdsrParams, Oscillator, Svf, SvfMode, Waveform, key_to_hz};
use musiccraft_params::inst;

use crate::{ChannelParams, Instrument};

/// Output level of one synth voice at full velocity, leaving headroom for chords.
const SYNTH_GAIN: f32 = 0.25;
/// Frames between filter coefficient updates while the envelope modulates cutoff.
const FILTER_UPDATE_INTERVAL: u32 = 32;

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
    env: Adsr,
    osc1: Oscillator,
    osc2: Oscillator,
    filter: Svf,
    filter_countdown: u32,
    sample_pos: f64,
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
            env: Adsr::new(sample_rate),
            osc1: Oscillator::new(),
            osc2: Oscillator::new(),
            filter: Svf::new(),
            filter_countdown: 0,
            sample_pos: 0.0,
        }
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
        self.one_shot = matches!(instrument, Instrument::Sampler(_))
            && param(params, inst::ONE_SHOT) >= 0.5
            && !start.live;
        self.env.reset();
        self.env.set_params(envelope_params(params));
        self.env.gate_on();
        self.osc1.reset(0.0);
        self.osc2.reset(0.25);
        self.filter.reset();
        self.filter_countdown = 0;
        self.sample_pos = 0.0;
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
                let ratio = 2.0_f64.powf(f64::from(pitch - root) / 12.0);
                let step = buffer.sample_rate() / f64::from(sample_rate) * ratio;
                let end = buffer.len() as f64;
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    if !self.advance_gate() {
                        continue;
                    }
                    let level = self.env.next_level() * self.velocity;
                    let (sl, sr) = buffer.read(self.sample_pos);
                    *l += sl * level;
                    *r += sr * level;
                    self.sample_pos += step;
                    if self.sample_pos >= end || self.env.is_idle() {
                        self.active = false;
                        break;
                    }
                }
            }
            Instrument::Synth => {
                let wave1 = Waveform::from_param(param(params, inst::WAVE1));
                let wave2 = Waveform::from_param(param(params, inst::WAVE2));
                let mix = param(params, inst::OSC_MIX).clamp(0.0, 1.0);
                let hz = f64::from(key_to_hz(pitch));
                let detune = 2.0_f64.powf(f64::from(param(params, inst::DETUNE)) / 1200.0);
                let sr = f64::from(sample_rate);
                self.osc1.set_frequency(hz, sr);
                self.osc2.set_frequency(hz * detune, sr);
                let cutoff = param(params, inst::CUTOFF);
                let resonance = param(params, inst::RESONANCE);
                let env_amount = param(params, inst::ENV_AMOUNT);
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    if !self.advance_gate() {
                        continue;
                    }
                    let level = self.env.next_level();
                    if self.filter_countdown == 0 {
                        let modulated = cutoff * 2.0_f32.powf(env_amount * level);
                        self.filter.set(modulated, resonance, sample_rate);
                        self.filter_countdown = FILTER_UPDATE_INTERVAL;
                    }
                    self.filter_countdown -= 1;
                    let a = self.osc1.next_sample(wave1);
                    let b = self.osc2.next_sample(wave2);
                    let raw = a + (b - a) * mix;
                    let out = self.filter.process(raw, SvfMode::LowPass)
                        * level
                        * self.velocity
                        * SYNTH_GAIN;
                    *l += out;
                    *r += out;
                    if self.env.is_idle() {
                        self.active = false;
                        break;
                    }
                }
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
