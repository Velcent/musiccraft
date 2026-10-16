use musiccraft_dsp::{Adsr, AdsrParams, Oscillator, Svf, SvfMode, Waveform, key_to_hz, pan_gains};
use musiccraft_params::inst;

use crate::modmatrix::{ModSources, ModState, SynthMods};
use crate::strings::{KsString, MAX_STRINGS};
use crate::{ChannelParams, Instrument};

/// Output level of one synth voice at full velocity, leaving headroom for chords.
const SYNTH_GAIN: f32 = 0.25;
/// Frames between updates of modulated values (filter, pitch, LFO).
const FILTER_UPDATE_INTERVAL: u32 = 32;
/// Most unison voices per note.
const MAX_UNISON: usize = 7;
/// Length of each voice's string delay line (lowest pluck ~12 Hz at 48 kHz).
/// Output level of the drum synth and pluck at full velocity.
const DRUM_GAIN: f32 = 0.8;
const PLUCK_GAIN: f32 = 0.35;
/// Most partials (and phases) a voice keeps for additive instruments.
const PARTIALS: usize = 32;
/// FM operators.
const FM_OPS: usize = 6;

/// Normalized harmonic levels of an additive spectrum: a `bright` slope
/// and an `even` share for even harmonics, summing to 1 over `count`.
fn additive_spectrum(bright: f32, even: f32, count: usize) -> [f32; PARTIALS] {
    let slope = 2.0 - 1.6 * bright.clamp(0.0, 1.0);
    let even = even.clamp(0.0, 1.0);
    let mut levels: [f32; PARTIALS] = std::array::from_fn(|i| {
        let n = i + 1;
        if n > count {
            0.0
        } else {
            (n as f32).powf(-slope) * if n % 2 == 0 { even } else { 1.0 }
        }
    });
    let norm = levels.iter().sum::<f32>().max(1e-3);
    for level in &mut levels {
        *level /= norm;
    }
    levels
}

/// How many strings the pluck sounds.
fn pluck_strings(params: &ChannelParams) -> usize {
    (param(params, inst::PLUCK_STRINGS).round().max(1.0) as usize).min(MAX_STRINGS)
}
/// FM algorithms, by `fm_algo`: for each operator (1 first) a bit mask of the
/// operators modulating it, the bit mask of carriers (heard operators), and
/// the operator with feedback.
const FM_ALGOS: [([u8; FM_OPS], u8, usize); 12] = [
    ([0b10, 0b100, 0b1000, 0, 0, 0], 0b1, 3),
    ([0b110, 0, 0b1000, 0, 0, 0], 0b1, 3),
    ([0b10, 0, 0b1000, 0, 0, 0], 0b101, 3),
    ([0b1000, 0b1000, 0b1000, 0, 0, 0], 0b111, 3),
    ([0b10, 0, 0, 0, 0, 0], 0b1101, 3),
    ([0, 0, 0, 0, 0, 0], 0b1111, 3),
    ([0b10, 0b100, 0b1000, 0b1_0000, 0b10_0000, 0], 0b1, 5),
    ([0b10, 0b100, 0, 0b1_0000, 0b10_0000, 0], 0b1001, 5),
    ([0b10, 0, 0b1000, 0, 0b10_0000, 0], 0b1_0101, 5),
    ([0b11_1110, 0, 0, 0, 0, 0], 0b1, 5),
    (
        [0b10_0000, 0b10_0000, 0b10_0000, 0b10_0000, 0b10_0000, 0],
        0b1_1111,
        5,
    ),
    ([0, 0, 0, 0, 0, 0], 0b11_1111, 5),
];

/// An FM operator's waveform at phase `x` (radians): sine, half sine, soft
/// square or soft saw, all within −1..1.
fn fm_wave(wave: i32, x: f32) -> f32 {
    match wave {
        1 => x.sin().max(0.0) * 2.0 - 0.64,
        2 => (x.sin() * 3.0).tanh(),
        3 => (x.sin() + (2.0 * x).sin() * 0.5 + (3.0 * x).sin() / 3.0) * 0.55,
        _ => x.sin(),
    }
}
/// Most grains sounding at once in one granular voice.
const GRAINS: usize = 24;

/// One grain of the granular sampler; `length == 0` means unused.
#[derive(Debug, Clone, Copy, Default)]
struct Grain {
    position: f64,
    step: f64,
    age: u32,
    length: u32,
}

/// Whether an instrument's pitch can move while a note sounds (portamento and
/// slide notes). Kit pads, the drum synth and the pluck fix their pitch when a
/// note starts.
pub(crate) fn glides(instrument: &Instrument) -> bool {
    matches!(
        instrument,
        Instrument::Synth
            | Instrument::Sampler(_)
            | Instrument::Fm
            | Instrument::Keys
            | Instrument::Additive
            | Instrument::Granular(_)
    )
}

/// How long a cut voice takes to fade out.
pub(crate) const CUT_SECONDS: f32 = 0.003;

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
    /// Per-note shaping.
    pub shape: crate::NoteShape,
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
    /// Per-note shaping of the note this voice plays.
    shape: crate::NoteShape,
    /// For a sliced sample: where the slice ends, in sample frames.
    slice_end: Option<f64>,
    env: Adsr,
    osc1: [Oscillator; MAX_UNISON],
    osc2: [Oscillator; MAX_UNISON],
    sub: Oscillator,
    osc3: Oscillator,
    lfo_phase: f32,
    filter_l: Svf,
    filter_r: Svf,
    filter2_l: Svf,
    filter2_r: Svf,
    /// Second LFO and modulation envelope.
    mod_state: ModState,
    /// The matrix's latest output, and the gain and pan it currently
    /// applies (ramped toward the latest output between updates).
    mods: SynthMods,
    mod_gain: f32,
    mod_pan: f32,
    filter_countdown: u32,
    /// Frames since the voice started sounding (drum synth).
    elapsed: u32,
    /// Oscillator phase in cycles (drum synth).
    drum_phase: f64,
    noise: u32,
    /// Plucked-string delay line, its write position, the delay in frames,
    /// the last read sample and the loudest output of the current period.
    strings: [KsString; MAX_STRINGS],
    /// Low-passed noise that bows the strings.
    bow_noise: f32,
    string_peak: f32,
    string_count: u32,
    sample_pos: f64,
    /// Sampler: playing backwards.
    reverse: bool,
    /// Sampler: output frames played, for the fade-in.
    sample_frames: f64,
    pan: (f32, f32),
    /// Current glide offset in keys (moves toward 0) and its change per frame.
    glide: f32,
    glide_rate: f32,
    /// Frames before a slide's glide begins (the slide note's offset in the block).
    glide_wait: u32,
    sample_rate: f32,
    /// Oscillator phases for FM, keys and additive.
    phases: [f64; PARTIALS],
    /// Operator 4's last output, for feedback.
    fm_prev: f32,
    /// The kit pad this voice plays.
    pad: usize,
    grains: [Grain; GRAINS],
    /// Frames until the next grain.
    grain_timer: f32,
    /// A cut (choke, mono, polyphony limit): frames to wait before fading
    /// out, counted once the voice's own start delay has passed.
    cut_wait: Option<u32>,
    /// Velocity lost per frame while a cut fades out.
    cut_step: f32,
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
            shape: crate::NoteShape::default(),
            slice_end: None,
            env: Adsr::new(sample_rate),
            osc1: std::array::from_fn(|_| Oscillator::new()),
            osc2: std::array::from_fn(|_| Oscillator::new()),
            sub: Oscillator::new(),
            osc3: Oscillator::new(),
            lfo_phase: 0.0,
            filter_l: Svf::new(),
            filter_r: Svf::new(),
            filter2_l: Svf::new(),
            filter2_r: Svf::new(),
            mod_state: ModState::default(),
            mods: SynthMods::default(),
            mod_gain: 1.0,
            mod_pan: 0.0,
            elapsed: 0,
            drum_phase: 0.0,
            noise: 0x9E37_79B9,
            strings: std::array::from_fn(|_| KsString::new()),
            bow_noise: 0.0,
            string_peak: 0.0,
            string_count: 0,
            phases: [0.0; PARTIALS],
            fm_prev: 0.0,
            pad: 0,
            grains: [Grain::default(); GRAINS],
            grain_timer: 0.0,
            cut_wait: None,
            cut_step: 0.0,
            filter_countdown: 0,
            sample_pos: 0.0,
            reverse: false,
            sample_frames: 0.0,
            pan: (1.0, 1.0),
            glide: 0.0,
            glide_rate: 0.0,
            glide_wait: 0,
            sample_rate: sample_rate.max(1.0),
        }
    }

    /// Whether the gate is still open (not releasing).
    pub fn held(&self) -> bool {
        self.active
            && self.cut_wait.is_none()
            && self.delay == 0
            && !self.env.is_releasing()
            && (self.gate.is_some() || self.live || self.one_shot)
    }

    /// The pitch in keys the voice is sounding now (before the channel pitch).
    pub fn current_key(&self) -> f32 {
        f32::from(self.key) + self.glide
    }

    /// Whether the voice's pitch follows its key (false for a sample slice,
    /// where the key only picks the slice).
    pub fn pitched(&self) -> bool {
        self.slice_end.is_none()
    }

    pub fn start(
        &mut self,
        start: VoiceStart,
        instrument: &Instrument,
        params: &ChannelParams,
        age: u64,
    ) {
        self.active = true;
        self.cut_wait = None;
        self.live = start.live;
        self.shape = start.shape;
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

        self.phases = [0.0; PARTIALS];
        self.fm_prev = 0.0;
        match instrument {
            Instrument::DrumSynth => self.start_drum(params),
            Instrument::Pluck => self.start_pluck(params, start.key),
            Instrument::Kit(_) => {
                let root = param(params, inst::ROOT_KEY).round() as i32;
                self.pad = (i32::from(start.key) - root).rem_euclid(16) as usize;
                self.one_shot = !start.live;
            }
            Instrument::Granular(_) => {
                self.grains = [Grain::default(); GRAINS];
                self.grain_timer = 0.0;
            }
            _ => {}
        }
        self.env.reset();
        self.env.set_params(self.envelope(params));
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
        self.filter2_l.reset();
        self.filter2_r.reset();
        self.mod_state.start();
        self.mods = SynthMods {
            gain: 1.0,
            ..SynthMods::default()
        };
        self.mod_gain = 1.0;
        self.mod_pan = 0.0;
        self.filter_countdown = 0;
        self.sample_pos = 0.0;
        if let Instrument::Granular(buffer) = instrument {
            self.sample_pos =
                f64::from(param(params, inst::GRAN_POSITION).clamp(0.0, 1.0)) * buffer.len() as f64;
        }
        self.slice_end = None;
        self.reverse = false;
        self.sample_frames = 0.0;
        if let Instrument::Sampler(buffer) = instrument {
            let slices = param(params, inst::SLICES).round().max(1.0) as i32;
            // A sliced audio clip starts part-way; a reversed sample plays
            // from its end.
            let offset = f64::from(start.shape.start.max(0.0)) * buffer.sample_rate();
            self.reverse = param(params, inst::REVERSE) >= 0.5 && slices <= 1;
            self.sample_pos = if self.reverse {
                buffer.len() as f64 - 1.0 - offset
            } else {
                offset
            };
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
        // Portamento notes glide even without a glide time (0.1 s then).
        let glide_time = if start.shape.porta {
            param(params, inst::GLIDE).max(0.1)
        } else {
            param(params, inst::GLIDE)
        };
        // The synth's glide time applies to overlapping notes; portamento notes
        // glide on every instrument whose pitch can move.
        let glide_on = matches!(instrument, Instrument::Synth)
            || (start.shape.porta && glides(instrument) && self.pitched());
        self.glide_wait = 0;
        match start.glide_from {
            Some(from) if glide_time > 0.0 && glide_on => {
                self.glide = from - f32::from(start.key);
                self.glide_rate = self.glide.abs() / (glide_time * self.env_rate());
            }
            _ => {
                self.glide = 0.0;
                self.glide_rate = 0.0;
            }
        }
    }

    /// The channel's envelope with this note's release velocity applied.
    fn envelope(&self, params: &ChannelParams) -> AdsrParams {
        let mut env = envelope_params(params);
        env.release *= self.shape.release;
        env
    }

    fn env_rate(&self) -> f32 {
        self.sample_rate
    }

    /// The colour group of the note this voice plays.
    pub fn color(&self) -> u8 {
        self.shape.color
    }

    /// How many frames this voice still sounds for, counted from `at` frames
    /// into the block: `None` when it is not sounding by then (idle, starting
    /// later, released or already ended), `u64::MAX` when nothing ends it
    /// (a live key or a one-shot sample).
    pub fn sounding_for(&self, at: u32) -> Option<u64> {
        if !self.active || self.cut_wait.is_some() || self.env.is_releasing() || self.delay > at {
            return None;
        }
        let elapsed = u64::from(at - self.delay);
        match self.gate {
            Some(gate) => gate.checked_sub(elapsed).filter(|&left| left > 0),
            None if self.live || self.one_shot => Some(u64::MAX),
            None => None,
        }
    }

    /// Moves this voice's key by `semitones` (a slide note), gliding there over
    /// `frames` frames that start `at` frames into the block. The voice keeps
    /// its own end.
    pub fn slide_by(&mut self, semitones: i16, at: u32, frames: u64) {
        let from = self.current_key();
        let key = (i16::from(self.key) + semitones).clamp(0, 127);
        self.key = u8::try_from(key).unwrap_or(self.key);
        self.glide = from - f32::from(self.key);
        self.glide_rate = self.glide.abs() / (frames.max(1) as f32);
        // The glide starts on the slide note's own frame; the voice's gate and
        // glide count only once its own start delay has passed.
        self.glide_wait = at.saturating_sub(self.delay);
    }

    /// Whether a cut has been scheduled (the voice is fading out or about to).
    pub fn is_cut(&self) -> bool {
        self.cut_wait.is_some()
    }

    /// Fades this voice out over [`CUT_SECONDS`], starting `at` frames into the
    /// block, whatever its instrument and release (one-shots included).
    pub fn cut(&mut self, at: u32) {
        if !self.active {
            return;
        }
        let wait = at.saturating_sub(self.delay);
        self.cut_wait = Some(self.cut_wait.map_or(wait, |w| w.min(wait)));
        self.cut_step = self.velocity.max(1e-6) / (CUT_SECONDS * self.sample_rate);
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
        self.env.set_params(self.envelope(params));
        let pitch = f32::from(self.key) + param(params, inst::PITCH) + self.shape.fine;
        match instrument {
            Instrument::Sampler(buffer) => {
                // Slices play at their own pitch: the key picks the slice.
                let pitch = if self.slice_end.is_some() {
                    param(params, inst::ROOT_KEY) + param(params, inst::PITCH)
                } else {
                    pitch
                };
                self.render_sample(buffer, params, pitch, left, right, sample_rate);
            }
            Instrument::Kit(pads) => {
                // Pads play at their own pitch.
                let pitch = param(params, inst::ROOT_KEY) + param(params, inst::PITCH);
                match pads.get(self.pad) {
                    Some(Some(buffer)) => {
                        self.render_sample(buffer, params, pitch, left, right, sample_rate);
                    }
                    _ => self.active = false,
                }
            }
            Instrument::Granular(buffer) => {
                self.render_granular(buffer, params, pitch, left, right, sample_rate);
            }
            Instrument::Fm => self.render_fm(params, pitch, left, right),
            Instrument::Keys => self.render_keys(params, pitch, left, right),
            Instrument::Additive => self.render_additive(params, pitch, left, right),
            // Layers forward notes; they never sound themselves.
            Instrument::Layer(_) => self.active = false,
            // External instruments never get voices; see `engine::trigger`.
            Instrument::External(_) => self.active = false,
            // MIDI Out channels send messages; see `engine::trigger`.
            Instrument::MidiOut(_) => self.active = false,
            Instrument::Synth => self.render_synth(params, pitch, left, right, sample_rate),
            Instrument::DrumSynth => self.render_drum(params, left, right),
            Instrument::Pluck => self.render_pluck(params, left, right),
        }
    }

    /// Plays `buffer` from the current position at `pitch` relative to the
    /// root key (sampler, slices and kit pads).
    fn render_sample(
        &mut self,
        buffer: &musiccraft_dsp::SampleBuffer,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: f32,
    ) {
        let root = param(params, inst::ROOT_KEY);
        let rate = buffer.sample_rate() / f64::from(sample_rate);
        let step_for = |glide: f32| rate * 2.0_f64.powf(f64::from(pitch + glide - root) / 12.0);
        let mut step = step_for(self.glide);
        let end = self
            .slice_end
            .unwrap_or(f64::INFINITY)
            .min(buffer.len() as f64);
        let fade_in = f64::from(param(params, inst::FADE_IN)) * f64::from(sample_rate);
        let fade_out = f64::from(param(params, inst::FADE_OUT)) * f64::from(sample_rate);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            if self.glide != 0.0 {
                // Portamento or a slide moves the pitch offset toward zero.
                self.step_glide();
                step = step_for(self.glide);
            }
            let fade = self.sample_fade(fade_in, fade_out, step, end);
            self.sample_frames += 1.0;
            let level = self.env.next_level() * self.velocity * fade;
            let (sl, sr) = buffer.read(self.sample_pos);
            *l += sl * level * self.pan.0;
            *r += sr * level * self.pan.1;
            let done = if self.reverse {
                self.sample_pos -= step;
                self.sample_pos < 0.0
            } else {
                self.sample_pos += step;
                self.sample_pos >= end
            };
            if done || self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// The sampler's fade gain (0..1) for the current frame: rising over
    /// `fade_in` output frames from the start, falling over the last
    /// `fade_out` frames before the note (or the sample) ends.
    fn sample_fade(&self, fade_in: f64, fade_out: f64, step: f64, end: f64) -> f32 {
        let mut gain = 1.0_f64;
        if fade_in > 0.0 {
            gain = gain.min(self.sample_frames / fade_in);
        }
        if fade_out > 0.0 {
            let source_left = if self.reverse {
                self.sample_pos
            } else {
                end - self.sample_pos
            };
            let mut left = source_left / step.abs().max(1e-9);
            if let (Some(gate), false) = (self.gate, self.one_shot) {
                left = left.min(gate as f64);
            }
            gain = gain.min(left / fade_out);
        }
        gain.clamp(0.0, 1.0) as f32
    }

    /// Writes one mono sample to both sides, with the note's pan.
    fn emit(&self, l: &mut f32, r: &mut f32, sample: f32) {
        *l += sample * self.pan.0;
        *r += sample * self.pan.1;
    }

    /// Four-operator FM.
    fn render_fm(
        &mut self,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let algo = (param(params, inst::FM_ALGO).round().max(0.0) as usize).min(FM_ALGOS.len() - 1);
        let Some(&(modulators, carriers, top)) = FM_ALGOS.get(algo) else {
            return;
        };
        let feedback = param(params, inst::FM_FEEDBACK).clamp(0.0, 1.0);
        let op = |n: usize, field: usize| {
            let base = if n < 4 {
                inst::OP1_RATIO + n * 4
            } else {
                inst::OP5_RATIO + (n - 4) * 4
            };
            param(params, base + field)
        };
        let ratio: [f32; FM_OPS] = std::array::from_fn(|i| op(i, 0));
        let level: [f32; FM_OPS] = std::array::from_fn(|i| op(i, 1));
        let decay: [f32; FM_OPS] = std::array::from_fn(|i| op(i, 2).max(0.005));
        let sustain: [f32; FM_OPS] = std::array::from_fn(|i| op(i, 3));
        let waves: [i32; FM_OPS] =
            std::array::from_fn(|i| param(params, inst::OP1_WAVE + i).round() as i32);
        let carrier_count = carriers.count_ones().max(1) as f32;
        let cutoff = param(params, inst::FM_CUTOFF);
        let filtered = cutoff < 19_999.0;
        let sr = self.sample_rate;
        if filtered {
            let resonance = param(params, inst::FM_RESONANCE).clamp(0.0, 1.0);
            self.filter_l.set(cutoff, resonance, sr);
            self.filter_r.set(cutoff, resonance, sr);
        }
        let mut hz = key_to_hz(pitch + self.glide);
        let tau = std::f32::consts::TAU;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            if self.glide != 0.0 {
                self.step_glide();
                hz = key_to_hz(pitch + self.glide);
            }
            let t = self.elapsed as f32 / sr;
            self.elapsed = self.elapsed.saturating_add(1);
            let amp = self.env.next_level() * self.velocity;
            let mut out_of = [0.0_f32; FM_OPS];
            let env = |i: usize| sustain[i] + (1.0 - sustain[i]) * (-t / decay[i]).exp();
            // Modulators always have higher numbers, so running from the top
            // down has every modulator ready first.
            for i in (0..FM_OPS).rev() {
                let mask = modulators.get(i).copied().unwrap_or(0);
                if level[i] <= 0.0 && carriers & (1 << i) == 0 && mask == 0 {
                    continue;
                }
                let phase = self.phases[i];
                self.phases[i] = (phase + f64::from(hz * ratio[i] / sr)).fract();
                let mut modulation: f32 = (0..FM_OPS)
                    .filter(|m| mask & (1 << m) != 0)
                    .map(|m| out_of[m])
                    .sum();
                if i == top {
                    modulation += self.fm_prev * feedback;
                }
                let x = phase as f32 * tau + modulation * 4.0;
                let value = fm_wave(waves[i], x) * level[i] * env(i);
                if i == top {
                    self.fm_prev = value;
                }
                out_of[i] = value;
            }
            let mix: f32 = (0..FM_OPS)
                .filter(|c| carriers & (1 << c) != 0)
                .map(|c| out_of[c])
                .sum::<f32>()
                / carrier_count;
            let mut sample = mix * amp * SYNTH_GAIN * 1.6;
            if filtered {
                sample = self.filter_l.process(sample, SvfMode::LowPass);
            }
            self.emit(l, r, sample);
            if self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// Piano, electric piano or organ.
    fn render_keys(
        &mut self,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let kind = param(params, inst::KEYS_TYPE).round() as i32;
        let tone = param(params, inst::KEYS_TONE).clamp(0.0, 1.0);
        let ring = param(params, inst::KEYS_DECAY).max(0.05);
        let tremolo = param(params, inst::KEYS_TREMOLO).clamp(0.0, 1.0);
        let hammer = param(params, inst::KEYS_HAMMER).clamp(0.0, 1.0);
        let sr = self.sample_rate;
        let mut hz = key_to_hz(pitch + self.glide);
        let tau = std::f32::consts::TAU;
        let nyquist = sr * 0.45;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            if self.glide != 0.0 {
                self.step_glide();
                hz = key_to_hz(pitch + self.glide);
            }
            let t = self.elapsed as f32 / sr;
            self.elapsed = self.elapsed.saturating_add(1);
            let amp = self.env.next_level() * self.velocity;
            let noise = self.next_noise();
            let trem = 1.0 - tremolo * 0.5 * (1.0 + (t * 5.5 * tau).sin());
            let sample = match kind {
                // Electric piano: a decaying FM tine with a bell overtone.
                1 => {
                    let index = (0.6 + tone * 3.0) * self.velocity * (-t / 0.5).exp();
                    let m = self.phases[1];
                    self.phases[1] = (m + f64::from(hz / sr)).fract();
                    let c = self.phases[0];
                    self.phases[0] = (c + f64::from(hz / sr)).fract();
                    let b = self.phases[2];
                    self.phases[2] = (b + f64::from((hz * 14.0).min(nyquist) / sr)).fract();
                    let tine = (c as f32 * tau + (m as f32 * tau).sin() * index).sin();
                    let bell = (b as f32 * tau).sin() * 0.08 * hammer * (-t / 0.04).exp();
                    (tine + bell) * (-t / ring).exp() * trem
                }
                // Organ: drawbar-like harmonics, a percussion blip, no decay.
                2 => {
                    const DRAWBARS: [f32; 9] = [0.5, 1.0, 1.5, 2.0, 3.0, 4.0, 5.0, 6.0, 8.0];
                    let mut sum = 0.0;
                    for (i, harmonic) in DRAWBARS.iter().enumerate() {
                        let f = hz * harmonic;
                        if f > nyquist {
                            break;
                        }
                        let weight = if i < 3 {
                            1.0
                        } else {
                            tone * 0.8 / (i as f32 - 1.0)
                        };
                        let phase = self.phases[i];
                        self.phases[i] = (phase + f64::from(f / sr)).fract();
                        sum += (phase as f32 * tau).sin() * weight;
                    }
                    let percussion =
                        (self.phases[4] as f32 * tau).sin() * hammer * (-t / 0.2).exp();
                    (sum * 0.3 + percussion * 0.5) * trem
                }
                // Piano: slightly stretched partials, upper ones fading faster.
                _ => {
                    let mut sum = 0.0;
                    for n in 1..=12 {
                        let nf = n as f32;
                        let f = hz * nf * (1.0 + 0.0004 * nf * nf).sqrt();
                        if f > nyquist {
                            break;
                        }
                        let i = n - 1;
                        let phase = self.phases[i];
                        self.phases[i] = (phase + f64::from(f / sr)).fract();
                        let brightness = 1.5 - tone * 0.8 - self.velocity * 0.3;
                        let weight = nf.powf(-brightness.max(0.3));
                        let fade = (-t * (1.0 + 0.5 * (nf - 1.0)) / ring).exp();
                        sum += (phase as f32 * tau).sin() * weight * fade;
                    }
                    let strike = if t < 0.015 {
                        noise * hammer * (1.0 - t / 0.015) * 0.6
                    } else {
                        0.0
                    };
                    (sum * 0.45 + strike) * trem
                }
            };
            self.emit(l, r, sample * amp * SYNTH_GAIN * 1.6);
            if self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// Additive synth: a sum of harmonics shaped by brightness, even level,
    /// stretch and per-harmonic decay.
    fn render_additive(
        &mut self,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
    ) {
        let count = (param(params, inst::ADD_PARTIALS).round() as usize).clamp(1, PARTIALS);
        let spectrum_a = additive_spectrum(
            param(params, inst::ADD_BRIGHT),
            param(params, inst::ADD_EVEN),
            count,
        );
        let spectrum_b = additive_spectrum(
            param(params, inst::ADD_BRIGHT_B),
            param(params, inst::ADD_EVEN_B),
            count,
        );
        let inharm_a = param(params, inst::ADD_INHARM).clamp(0.0, 1.0);
        let inharm_b = param(params, inst::ADD_INHARM_B).clamp(0.0, 1.0);
        let morph = param(params, inst::ADD_MORPH).clamp(0.0, 1.0);
        let morph_time = param(params, inst::ADD_MORPH_TIME).max(0.0);
        let morph_rate = param(params, inst::ADD_MORPH_RATE).max(0.0);
        let morph_depth = param(params, inst::ADD_MORPH_DEPTH).clamp(0.0, 1.0);
        let fade = param(params, inst::ADD_DECAY).clamp(0.0, 1.0);
        let sr = self.sample_rate;
        let mut hz = key_to_hz(pitch + self.glide);
        let tau = std::f32::consts::TAU;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            if self.glide != 0.0 {
                self.step_glide();
                hz = key_to_hz(pitch + self.glide);
            }
            let t = self.elapsed as f32 / sr;
            self.elapsed = self.elapsed.saturating_add(1);
            let amp = self.env.next_level() * self.velocity;
            // Where between the spectra this frame sits.
            let mut m = morph;
            if morph_time > 0.0 {
                m += (1.0 - morph) * (t / morph_time).min(1.0);
            }
            if morph_rate > 0.0 {
                m += (t * morph_rate * tau).sin() * morph_depth * 0.5;
            }
            let m = m.clamp(0.0, 1.0);
            let inharm = inharm_a + (inharm_b - inharm_a) * m;
            let mut sum = 0.0;
            for n in 1..=count {
                let nf = n as f32;
                let f = hz * nf * (1.0 + inharm * 0.02 * (nf - 1.0));
                if f > sr * 0.45 {
                    break;
                }
                let i = n - 1;
                let phase = self.phases[i];
                self.phases[i] = (phase + f64::from(f / sr)).fract();
                let (a, b) = (spectrum_a[i], spectrum_b[i]);
                let weight = a + (b - a) * m;
                let decay = (-t * fade * nf * 1.5).exp();
                sum += (phase as f32 * tau).sin() * weight * decay;
            }
            self.emit(l, r, sum * amp * SYNTH_GAIN * 1.6);
            if self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// Granular sampler: windowed grains from around a scanning position.
    fn render_granular(
        &mut self,
        buffer: &musiccraft_dsp::SampleBuffer,
        params: &ChannelParams,
        pitch: f32,
        left: &mut [f32],
        right: &mut [f32],
        sample_rate: f32,
    ) {
        let len = buffer.len() as f64;
        if len < 2.0 {
            self.active = false;
            return;
        }
        let root = param(params, inst::ROOT_KEY);
        let size = (param(params, inst::GRAN_SIZE).max(1.0) * 0.001 * sample_rate) as u32;
        let interval = sample_rate / param(params, inst::GRAN_DENSITY).max(0.1);
        let spread = f64::from(param(params, inst::GRAN_SPREAD).clamp(0.0, 1.0));
        let jitter = param(params, inst::GRAN_PITCH_RAND).max(0.0);
        let scan = f64::from(param(params, inst::GRAN_SCAN));
        let rate = buffer.sample_rate() / f64::from(sample_rate);
        let step_at = |glide: f32| rate * 2.0_f64.powf(f64::from(pitch + glide - root) / 12.0);
        let mut base_step = step_at(self.glide);
        // Overlapping grains are normalized by how many overlap on average.
        let overlap = (size as f32 / interval).max(1.0);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            if self.glide != 0.0 {
                // New grains follow portamento and slides.
                self.step_glide();
                base_step = step_at(self.glide);
            }
            self.grain_timer -= 1.0;
            if self.grain_timer <= 0.0 {
                self.grain_timer += interval;
                let offset = f64::from(self.next_noise()) * spread * len * 0.5;
                let detune = f64::from(self.next_noise() * jitter);
                if let Some(grain) = self.grains.iter_mut().find(|g| g.length == 0) {
                    *grain = Grain {
                        position: (self.sample_pos + offset).rem_euclid(len),
                        step: base_step * 2.0_f64.powf(detune / 12.0),
                        age: 0,
                        length: size.max(2),
                    };
                }
            }
            self.sample_pos = (self.sample_pos + scan * rate).rem_euclid(len);
            let level = self.env.next_level() * self.velocity / overlap.sqrt();
            let (mut sum_l, mut sum_r) = (0.0, 0.0);
            for grain in self.grains.iter_mut().filter(|g| g.length > 0) {
                let x = grain.age as f32 / grain.length as f32;
                let window = 0.5 - 0.5 * (x * std::f32::consts::TAU).cos();
                let (gl, gr) = buffer.read(grain.position.rem_euclid(len));
                sum_l += gl * window;
                sum_r += gr * window;
                grain.position += grain.step;
                grain.age += 1;
                if grain.age >= grain.length {
                    grain.length = 0;
                }
            }
            *l += sum_l * level * self.pan.0;
            *r += sum_r * level * self.pan.1;
            if self.env.is_idle() {
                self.active = false;
                break;
            }
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
        let count = pluck_strings(params);
        let detune = param(params, inst::PLUCK_DETUNE);
        let coeff = 0.05 + 0.95 * tone;
        let mut noise = self.noise;
        for (k, string) in self.strings.iter_mut().enumerate() {
            if k >= count {
                string.clear();
                continue;
            }
            // Extra strings sit `detune` cents above and below the note.
            let cents = match k {
                1 => detune,
                2 => -detune,
                _ => 0.0,
            };
            let delay = self.sample_rate / (hz * 2.0_f32.powf(cents / 1_200.0));
            let mut smooth = 0.0;
            string.excite(delay, || {
                noise ^= noise << 13;
                noise ^= noise >> 17;
                noise ^= noise << 5;
                let n = noise as f32 / u32::MAX as f32 * 2.0 - 1.0;
                smooth += (n - smooth) * coeff;
                smooth
            });
        }
        self.noise = noise;
        self.bow_noise = 0.0;
        self.string_peak = 1.0;
        self.string_count = 0;
    }

    /// Plucked (or bowed) strings (Karplus-Strong): the burst circulates
    /// through damped delays tuned to the note; bowing keeps feeding them
    /// noise while the note is held.
    fn render_pluck(&mut self, params: &ChannelParams, left: &mut [f32], right: &mut [f32]) {
        let damping = param(params, inst::PLUCK_DAMPING).clamp(0.0, 1.0) * 0.5;
        let sr = self.sample_rate;
        let count = pluck_strings(params);
        let period = self.strings.first().map_or(100.0, |s| s.delay);
        let sustain = 0.001_f32.powf(period / (param(params, inst::PLUCK_DECAY).max(0.05) * sr));
        let release = 0.001_f32.powf(period / (0.08 * sr));
        let bow = param(params, inst::PLUCK_BOW).clamp(0.0, 1.0);
        let bow_tone = 0.05 + 0.95 * param(params, inst::PLUCK_TONE).clamp(0.0, 1.0);
        let norm = 1.0 / (count as f32).sqrt();
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !self.advance_gate() {
                continue;
            }
            let releasing = self.env.is_releasing();
            let feedback = if releasing { release } else { sustain };
            let inject = if bow > 0.0 && !releasing {
                let level = self.env.next_level();
                let n = self.next_noise();
                self.bow_noise += (n - self.bow_noise) * bow_tone;
                self.bow_noise * bow * level * 0.03
            } else {
                0.0
            };
            let mut read = 0.0;
            for string in self.strings.iter_mut().take(count) {
                read += string.step(damping, feedback, inject);
            }
            read *= norm;
            let out = read * self.velocity * PLUCK_GAIN;
            *l += out * self.pan.0;
            *r += out * self.pan.1;
            // Stop once a whole period stays below -80 dB (a held, bowed
            // string keeps going).
            self.string_peak = self.string_peak.max(read.abs());
            self.string_count += 1;
            if self.string_count as f32 >= period {
                if self.string_peak < 1e-4 && (bow == 0.0 || releasing) {
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
        // Mod X and Y of the note offset the filter.
        let cutoff = param(params, inst::CUTOFF) * 2.0_f32.powf(self.shape.cutoff);
        let resonance = (param(params, inst::RESONANCE) + self.shape.resonance).clamp(0.0, 1.0);
        let env_amount = param(params, inst::ENV_AMOUNT);
        let unison = (param(params, inst::UNISON).round() as usize).clamp(1, MAX_UNISON);
        let spread = param(params, inst::SPREAD);
        let sub_level = param(params, inst::SUB).clamp(0.0, 1.0);
        let wave3 = Waveform::from_param(param(params, inst::WAVE3));
        let osc3_level = param(params, inst::OSC3_LEVEL).clamp(0.0, 1.0);
        let osc3_ratio = 2.0_f64.powf(f64::from(param(params, inst::OSC3_COARSE)) / 12.0);
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
        let mode2 = match param(params, inst::FILTER2_TYPE).round() as i32 {
            1 => Some(SvfMode::LowPass),
            2 => Some(SvfMode::BandPass),
            3 => Some(SvfMode::HighPass),
            _ => None,
        };
        let cutoff2 = param(params, inst::CUTOFF2);
        let resonance2 = param(params, inst::RESONANCE2).clamp(0.0, 1.0);
        let routing = param(params, inst::FILTER_ROUTING).round() as i32;
        let ramp = 1.0 / FILTER_UPDATE_INTERVAL as f32;
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
            // Glide moves every frame; the oscillators follow at control rate.
            self.step_glide();
            let level = self.env.next_level();
            if self.filter_countdown == 0 {
                let lfo_raw = (self.lfo_phase * std::f32::consts::TAU).sin();
                lfo = lfo_raw * lfo_depth;
                let sources = ModSources {
                    lfo1: lfo_raw,
                    velocity: self.velocity,
                    key: ((pitch - 60.0) / 24.0).clamp(-1.0, 1.0),
                    amp_env: level,
                };
                let mut noise = self.noise;
                self.mods = self.mod_state.update(
                    params,
                    sources,
                    FILTER_UPDATE_INTERVAL,
                    sample_rate,
                    || {
                        noise ^= noise << 13;
                        noise ^= noise >> 17;
                        noise ^= noise << 5;
                        noise as f32 / u32::MAX as f32 * 2.0 - 1.0
                    },
                );
                self.noise = noise;
                let mods = self.mods;
                let vibrato = if lfo_target == 0 { lfo * 2.0 } else { 0.0 };
                let base = f64::from(key_to_hz(pitch + self.glide + vibrato + mods.pitch));
                let detune = detune * 2.0_f64.powf(f64::from(mods.detune) / 1200.0);
                for i in 0..unison {
                    let cents = f64::from(spread * position(i));
                    let hz = base * 2.0_f64.powf(cents / 1200.0);
                    if let (Some(a), Some(b)) = (self.osc1.get_mut(i), self.osc2.get_mut(i)) {
                        a.set_frequency(hz, sr);
                        b.set_frequency(hz * detune, sr);
                    }
                }
                self.sub.set_frequency(base * 0.5, sr);
                self.osc3.set_frequency(base * osc3_ratio, sr);
                let lfo_octaves = if lfo_target == 1 { lfo * 3.0 } else { 0.0 };
                let modulated =
                    cutoff * 2.0_f32.powf(env_amount * level + lfo_octaves + mods.cutoff);
                let resonance = (resonance + mods.resonance).clamp(0.0, 1.0);
                self.filter_l.set(modulated, resonance, sample_rate);
                self.filter_r.set(modulated, resonance, sample_rate);
                if mode2.is_some() {
                    let f2 = cutoff2 * 2.0_f32.powf(mods.cutoff2);
                    self.filter2_l.set(f2, resonance2, sample_rate);
                    self.filter2_r.set(f2, resonance2, sample_rate);
                }
                self.filter_countdown = FILTER_UPDATE_INTERVAL;
            }
            self.mod_gain += (self.mods.gain - self.mod_gain) * ramp;
            self.mod_pan += (self.mods.pan.clamp(-1.0, 1.0) - self.mod_pan) * ramp;
            let mix = (mix + self.mods.osc_mix).clamp(0.0, 1.0);
            let osc3_level = (osc3_level + self.mods.osc3).clamp(0.0, 1.0);
            self.filter_countdown -= 1;
            self.lfo_phase = (self.lfo_phase + lfo_step).fract();
            // Group A is oscillator 1 and the sub, group B oscillators 2
            // and 3; the split routing filters them separately.
            let (mut a_l, mut a_r, mut b_l, mut b_r) = (0.0, 0.0, 0.0, 0.0);
            for i in 0..unison {
                let (Some(a), Some(b)) = (self.osc1.get_mut(i), self.osc2.get_mut(i)) else {
                    continue;
                };
                let a = a.next_sample(wave1) * (1.0 - mix) * norm;
                let b = b.next_sample(wave2) * mix * norm;
                let p = position(i);
                a_l += a * (1.0 - p);
                a_r += a * (1.0 + p);
                b_l += b * (1.0 - p);
                b_r += b * (1.0 + p);
            }
            let sub = self.sub.next_sample(Waveform::Sine) * sub_level;
            let third = if osc3_level > 0.0 {
                self.osc3.next_sample(wave3) * osc3_level
            } else {
                0.0
            };
            let mut groups = [a_l + sub, a_r + sub, b_l + third, b_r + third];
            if drive > 0.001 {
                for x in &mut groups {
                    *x = (*x * drive_gain).tanh() * drive_norm;
                }
            }
            let [a_l, a_r, b_l, b_r] = groups;
            let tremolo = if lfo_target == 2 {
                1.0 - (0.5 + 0.5 * lfo) * lfo_depth
            } else {
                1.0
            };
            let gain = level * self.velocity * SYNTH_GAIN * tremolo * self.mod_gain;
            let (out_l, out_r) = match mode2 {
                None => (
                    self.filter_l.process(a_l + b_l, mode),
                    self.filter_r.process(a_r + b_r, mode),
                ),
                Some(mode2) => match routing {
                    1 => (
                        (self.filter_l.process(a_l + b_l, mode)
                            + self.filter2_l.process(a_l + b_l, mode2))
                            * 0.5,
                        (self.filter_r.process(a_r + b_r, mode)
                            + self.filter2_r.process(a_r + b_r, mode2))
                            * 0.5,
                    ),
                    2 => (
                        self.filter_l.process(a_l, mode) + self.filter2_l.process(b_l, mode2),
                        self.filter_r.process(a_r, mode) + self.filter2_r.process(b_r, mode2),
                    ),
                    _ => (
                        self.filter2_l
                            .process(self.filter_l.process(a_l + b_l, mode), mode2),
                        self.filter2_r
                            .process(self.filter_r.process(a_r + b_r, mode), mode2),
                    ),
                },
            };
            let (pan_l, pan_r) = if self.mod_pan == 0.0 {
                (1.0, 1.0)
            } else {
                ((1.0 - self.mod_pan).min(1.0), (1.0 + self.mod_pan).min(1.0))
            };
            *l += out_l * gain * self.pan.0 * pan_l;
            *r += out_r * gain * self.pan.1 * pan_r;
            if self.env.is_idle() {
                self.active = false;
                break;
            }
        }
    }

    /// Moves the glide offset one frame toward zero, after any slide wait.
    fn step_glide(&mut self) {
        if self.glide_wait > 0 {
            self.glide_wait -= 1;
        } else if self.glide != 0.0 {
            let change = self.glide_rate.min(self.glide.abs());
            self.glide -= change * self.glide.signum();
        }
    }

    /// Handles the start delay and the gate countdown for one frame. Returns
    /// whether the voice sounds on this frame.
    fn advance_gate(&mut self) -> bool {
        if !self.active {
            return false;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return false;
        }
        match self.cut_wait {
            Some(0) => {
                self.velocity -= self.cut_step;
                if self.velocity <= 0.0 {
                    self.velocity = 0.0;
                    self.active = false;
                    return false;
                }
            }
            Some(wait) => self.cut_wait = Some(wait - 1),
            None => {}
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
            shape: crate::NoteShape::default(),
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
            shape: crate::NoteShape::default(),
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
    fn second_filter_routings_and_matrix_change_the_sound() {
        let base = ChannelParams::default();
        let (reference, _) = render_synth_note(&base);
        let mut with_filter2 = base;
        with_filter2.instrument[inst::FILTER2_TYPE] = 1.0;
        with_filter2.instrument[inst::CUTOFF2] = 300.0;
        let cases: [(&[(usize, f32)], &str); 4] = [
            (&[], "series"),
            (&[(inst::FILTER_ROUTING, 1.0)], "parallel"),
            (
                &[(inst::FILTER_ROUTING, 2.0), (inst::OSC_MIX, 0.5)],
                "split",
            ),
            (
                &[
                    (inst::MOD1_SOURCE, 4.0),
                    (inst::MOD1_DEST, 0.0),
                    (inst::MOD1_AMOUNT, 1.0),
                ],
                "velocity to pitch",
            ),
        ];
        let mut outputs = Vec::new();
        for (changes, name) in cases {
            let mut params = with_filter2;
            for (i, v) in changes {
                params.instrument[*i] = *v;
            }
            let (out, _) = render_synth_note(&params);
            let diff: f32 = out.iter().zip(&reference).map(|(a, b)| (a - b).abs()).sum();
            assert!(diff > 1.0, "{name} made no difference");
            assert!(out.iter().all(|s| s.is_finite() && s.abs() < 2.0), "{name}");
            outputs.push(out);
        }
        // Each routing sounds different from the others.
        for (i, a) in outputs.iter().enumerate() {
            for b in outputs.iter().skip(i + 1) {
                assert!(a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-3));
            }
        }
    }

    #[test]
    fn matrix_pitch_from_velocity_raises_an_octave() {
        let mut params = ChannelParams::default();
        params.instrument[inst::WAVE1] = 0.0;
        params.instrument[inst::OSC_MIX] = 0.0;
        params.instrument[inst::CUTOFF] = 20_000.0;
        params.instrument[inst::SUSTAIN] = 1.0;
        let (plain, _) = render_synth_note(&params);
        params.instrument[inst::MOD1_SOURCE] = 4.0;
        params.instrument[inst::MOD1_DEST] = 0.0;
        params.instrument[inst::MOD1_AMOUNT] = 1.0;
        let (raised, _) = render_synth_note(&params);
        let crossings = |x: &[f32]| x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        let (a, b) = (crossings(&plain[1_000..]), crossings(&raised[1_000..]));
        assert!((b as f32 / a as f32 - 2.0).abs() < 0.1, "{a} vs {b}");
    }

    fn held_start(key: u8) -> VoiceStart {
        VoiceStart {
            shape: crate::NoteShape::default(),
            slide: false,
            channel: 0,
            key,
            velocity: 1.0,
            delay: 0,
            gate: None,
            live: true,
            pan: 0.0,
            glide_from: None,
        }
    }

    fn render_frames(voice: &mut Voice, instrument: &Instrument, params: &ChannelParams, n: usize) {
        let mut l = vec![0.0; n];
        let mut r = vec![0.0; n];
        voice.render(instrument, params, &mut l, &mut r, 1_000.0);
    }

    #[test]
    fn slides_move_every_pitched_instrument_from_their_own_frame() {
        let buffer = std::sync::Arc::new(musiccraft_dsp::SampleBuffer::mono(
            vec![0.25; 48_000],
            1_000.0,
        ));
        let instruments = [
            ("synth", Instrument::Synth),
            ("fm", Instrument::Fm),
            ("keys", Instrument::Keys),
            ("additive", Instrument::Additive),
            ("sampler", Instrument::Sampler(buffer.clone())),
            ("granular", Instrument::Granular(buffer)),
        ];
        let params = ChannelParams::default();
        for (name, instrument) in &instruments {
            let mut voice = Voice::new(1_000.0);
            voice.start(held_start(57), instrument, &params, 1);
            // A slide note 40 frames into the block, gliding over 100 frames.
            voice.slide_by(12, 40, 100);
            render_frames(&mut voice, instrument, &params, 40);
            assert!(
                (voice.current_key() - 57.0).abs() < 1e-4,
                "{name} moved before the slide note: {}",
                voice.current_key()
            );
            render_frames(&mut voice, instrument, &params, 50);
            assert!(
                (voice.current_key() - 63.0).abs() < 0.2,
                "{name} halfway: {}",
                voice.current_key()
            );
            render_frames(&mut voice, instrument, &params, 100);
            assert!(
                (voice.current_key() - 69.0).abs() < 1e-4,
                "{name} arrived: {}",
                voice.current_key()
            );
        }
    }

    #[test]
    fn portamento_notes_glide_on_fm_keys_and_additive() {
        let params = ChannelParams::default();
        for (name, instrument) in [
            ("fm", Instrument::Fm),
            ("keys", Instrument::Keys),
            ("additive", Instrument::Additive),
        ] {
            let mut start = held_start(69);
            start.shape.porta = true;
            start.glide_from = Some(57.0);
            let mut voice = Voice::new(1_000.0);
            voice.start(start, &instrument, &params, 1);
            assert!((voice.current_key() - 57.0).abs() < 1e-4, "{name} start");
            // Portamento without a glide time takes 0.1 s (100 frames here).
            render_frames(&mut voice, &instrument, &params, 400);
            assert!(
                (voice.current_key() - 69.0).abs() < 1e-4,
                "{name} arrived: {}",
                voice.current_key()
            );
        }
    }

    #[test]
    fn sample_slices_are_not_pitched() {
        let buffer = std::sync::Arc::new(musiccraft_dsp::SampleBuffer::mono(
            vec![0.25; 4_000],
            1_000.0,
        ));
        let mut params = ChannelParams::default();
        let instrument = Instrument::Sampler(buffer);
        let mut voice = Voice::new(1_000.0);
        voice.start(held_start(60), &instrument, &params, 1);
        assert!(voice.pitched());
        params.instrument[inst::SLICES] = 4.0;
        voice.start(held_start(60), &instrument, &params, 1);
        assert!(!voice.pitched());
        assert!(glides(&instrument));
        assert!(!glides(&Instrument::DrumSynth));
        assert!(!glides(&Instrument::Pluck));
        assert!(!glides(&Instrument::Kit(Vec::new())));
    }

    #[test]
    fn no_glide_without_a_glide_time() {
        let params = ChannelParams::default();
        let start = VoiceStart {
            shape: crate::NoteShape::default(),
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
            shape: crate::NoteShape::default(),
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

    #[test]
    fn bowed_strings_sustain_while_held_and_extra_strings_detune() {
        let mut params = ChannelParams::default();
        params.instrument[inst::PLUCK_DECAY] = 0.3;
        params.instrument[inst::SUSTAIN] = 1.0;
        let plucked = render(&Instrument::Pluck, &params, 69, 96_000, None);
        params.instrument[inst::PLUCK_BOW] = 1.0;
        let bowed = render(&Instrument::Pluck, &params, 69, 96_000, None);
        let late = 72_000..96_000;
        assert!(
            peak(&plucked[late.clone()]) < 1e-3,
            "a plucked string dies away"
        );
        assert!(
            peak(&bowed[late.clone()]) > 0.02,
            "a bowed one keeps sounding"
        );
        let hz = autocorrelation_hz(&bowed[48_000..57_600]);
        assert!((hz - 440.0).abs() < 10.0, "bowed in tune: {hz}");
        assert!(bowed.iter().all(|s| s.is_finite() && s.abs() < 1.5));
        // Released, the bowed string stops too.
        let released = render(&Instrument::Pluck, &params, 69, 48_000, Some(24_000));
        assert!(peak(&released[40_000..]) < 1e-3);
        params.instrument[inst::PLUCK_BOW] = 0.0;
        params.instrument[inst::PLUCK_STRINGS] = 3.0;
        let three = render(&Instrument::Pluck, &params, 69, 24_000, None);
        assert_ne!(three, plucked[..24_000].to_vec());
        let hz = autocorrelation_hz(&three[2_400..12_000]);
        assert!(
            (hz - 440.0).abs() < 10.0,
            "three strings centre on the note: {hz}"
        );
    }
}

#[cfg(test)]
mod more_instrument_tests {
    use std::sync::Arc;

    use musiccraft_dsp::SampleBuffer;

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
            shape: crate::NoteShape::default(),
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

    fn pitch_hz(data: &[f32]) -> f32 {
        let score = |lag: usize| -> f32 { data.iter().zip(&data[lag..]).map(|(a, b)| a * b).sum() };
        let lag = (40..1_200)
            .max_by(|a, b| score(*a).total_cmp(&score(*b)))
            .unwrap_or(1);
        48_000.0 / lag as f32
    }

    #[test]
    fn fm_keys_and_additive_sound_in_tune_and_release() {
        let mut params = ChannelParams::default();
        params.instrument[inst::SUSTAIN] = 1.0;
        params.instrument[inst::RELEASE] = 0.05;
        for (name, instrument, setup) in [
            ("fm", Instrument::Fm, vec![]),
            ("piano", Instrument::Keys, vec![(inst::KEYS_TYPE, 0.0)]),
            ("epiano", Instrument::Keys, vec![(inst::KEYS_TYPE, 1.0)]),
            ("organ", Instrument::Keys, vec![(inst::KEYS_TYPE, 2.0)]),
            ("additive", Instrument::Additive, vec![]),
        ] {
            let mut p = params;
            for (i, v) in setup {
                p.instrument[i] = v;
            }
            // Key 57 is A4 (220 Hz) in MusicCraft's naming.
            let out = render(&instrument, &p, 57, 24_000, Some(12_000));
            let level = peak(&out[..12_000]);
            assert!(level > 0.05 && level < 1.0, "{name} level {level}");
            assert!(out.iter().all(|s| s.is_finite()), "{name} finite");
            if name != "organ" {
                let hz = pitch_hz(&out[2_400..9_600]);
                assert!((hz - 220.0).abs() < 6.0, "{name} at {hz} Hz");
            }
            assert!(peak(&out[18_000..]) < 1e-3, "{name} releases");
        }
    }

    #[test]
    fn additive_morphs_between_two_spectra() {
        let mut params = ChannelParams::default();
        params.instrument[inst::SUSTAIN] = 1.0;
        params.instrument[inst::ADD_BRIGHT] = 0.0;
        params.instrument[inst::ADD_BRIGHT_B] = 1.0;
        let edges = |x: &[f32]| {
            let level = peak(x).max(1e-6);
            x.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>() / level
        };
        let dark = render(&Instrument::Additive, &params, 57, 9_600, None);
        params.instrument[inst::ADD_MORPH] = 1.0;
        let bright = render(&Instrument::Additive, &params, 57, 9_600, None);
        assert!(edges(&bright[4_800..]) > edges(&dark[4_800..]) * 1.3);
        // A one-second sweep goes from the first spectrum to the second.
        params.instrument[inst::ADD_MORPH] = 0.0;
        params.instrument[inst::ADD_MORPH_TIME] = 1.0;
        let sweep = render(&Instrument::Additive, &params, 57, 52_800, None);
        assert!(edges(&sweep[48_000..52_800]) > edges(&sweep[0..4_800]) * 1.3);
        let hz = pitch_hz(&sweep[2_400..9_600]);
        assert!((hz - 220.0).abs() < 6.0, "stays in tune: {hz}");
    }

    #[test]
    fn six_operator_algorithms_waves_and_filter() {
        let mut params = ChannelParams::default();
        params.instrument[inst::SUSTAIN] = 1.0;
        let four = render(&Instrument::Fm, &params, 57, 9_600, None);
        // The old algorithms ignore operators 5 and 6.
        let mut extra = params;
        extra.instrument[inst::OP5_LEVEL] = 1.0;
        extra.instrument[inst::OP6_LEVEL] = 1.0;
        assert_eq!(render(&Instrument::Fm, &extra, 57, 9_600, None), four);
        let mut outputs = vec![four];
        for algo in 6..=11 {
            let mut p = extra;
            p.instrument[inst::FM_ALGO] = algo as f32;
            for level in [
                inst::OP1_LEVEL,
                inst::OP2_LEVEL,
                inst::OP3_LEVEL,
                inst::OP4_LEVEL,
            ] {
                p.instrument[level] = 0.7;
            }
            let out = render(&Instrument::Fm, &p, 57, 9_600, None);
            let level = peak(&out);
            assert!(level > 0.02 && level < 1.0, "algorithm {algo}: {level}");
            assert!(out.iter().all(|s| s.is_finite()));
            outputs.push(out);
        }
        for (i, a) in outputs.iter().enumerate() {
            for b in outputs.iter().skip(i + 1) {
                assert!(a.iter().zip(b).any(|(x, y)| (x - y).abs() > 1e-3));
            }
        }
        let mut square = params;
        square.instrument[inst::OP1_WAVE] = 2.0;
        assert_ne!(
            render(&Instrument::Fm, &square, 57, 2_400, None),
            outputs[0]
        );
        let mut dark = params;
        dark.instrument[inst::OP1_WAVE] = 3.0;
        let bright = render(&Instrument::Fm, &dark, 57, 9_600, None);
        dark.instrument[inst::FM_CUTOFF] = 300.0;
        let filtered = render(&Instrument::Fm, &dark, 57, 9_600, None);
        let edges = |x: &[f32]| x.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>();
        assert!(
            edges(&filtered) < edges(&bright) * 0.8,
            "the filter smooths it"
        );
    }

    #[test]
    fn kit_pads_and_grains_play_their_samples() {
        let ramp: Vec<f32> = (0..4_800)
            .map(|i| ((i as f32) * 0.05).sin() * 0.5)
            .collect();
        let buffer = Arc::new(SampleBuffer::mono(ramp, 48_000.0));
        let kit = Instrument::Kit(vec![None, Some(Arc::clone(&buffer))]);
        let mut params = ChannelParams::default();
        params.instrument[inst::SUSTAIN] = 1.0;
        let root = params.instrument[inst::ROOT_KEY] as u8;
        assert!(
            peak(&render(&kit, &params, root + 1, 2_000, Some(10))) > 0.4,
            "pad 2 plays"
        );
        assert!(
            peak(&render(&kit, &params, root, 2_000, Some(10))) < 1e-6,
            "pad 1 is empty"
        );
        let granular = Instrument::Granular(buffer);
        let out = render(&granular, &params, root, 24_000, None);
        assert!(peak(&out[4_800..]) > 0.05, "grains sound");
        assert!(out.iter().all(|s| s.is_finite() && s.abs() < 2.0));
    }

    #[test]
    fn third_oscillator_adds_to_the_synth() {
        let mut params = ChannelParams::default();
        let base = render(&Instrument::Synth, &params, 57, 4_800, None);
        params.instrument[inst::OSC3_LEVEL] = 1.0;
        let with = render(&Instrument::Synth, &params, 57, 4_800, None);
        let diff: f32 = base.iter().zip(&with).map(|(a, b)| (a - b).abs()).sum();
        assert!(diff > 1.0);
    }
}
