//! Tone, distortion, filtering and dynamics effects: bass boost, overdrive,
//! soft clipper, waveshaper, DC removal, ring modulator, frequency shifter,
//! formant filter, auto filter, noise gate, transient shaper, multi-tap delay,
//! graphic EQ and seven-band parametric EQ. All allocation happens in `new`.

use std::f32::consts::{PI, TAU};

use crate::modulation::SYNC_RATES;
use crate::{Biquad, ProcessContext, Svf, SvfMode, db_to_gain, flush_denormal};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Bass boost: a low shelf with gentle saturation.
///
/// Parameters: `[freq Hz, boost dB, drive]`.
#[derive(Debug, Clone, Default)]
pub struct BassBoost {
    shelves: [Biquad; 2],
}

impl BassBoost {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let freq = param(p, 0, 80.0);
        let boost = param(p, 1, 6.0);
        let drive = param(p, 2, 0.2).clamp(0.0, 1.0);
        let k = 1.0 + drive * 3.0;
        let norm = 1.0 / k.tanh();
        for (shelf, channel) in self.shelves.iter_mut().zip([left, right]) {
            shelf.set_low_shelf(freq, boost, ctx.sample_rate);
            for s in channel.iter_mut() {
                let x = shelf.process(*s);
                *s = if drive > 0.001 {
                    (x * k).tanh() * norm
                } else {
                    x
                };
            }
        }
    }
}

/// The waveshaping curve `shape` (0 tanh, 1 hard clip, 2 fold, 3 sine,
/// 4 asymmetric, 5 rectify) for an input already multiplied by its drive.
#[must_use]
pub fn shape_curve(shape: f32, x: f32) -> f32 {
    match shape.round() as i32 {
        1 => x.clamp(-1.0, 1.0),
        2 => {
            // Fold back at ±1.
            let y = (x + 1.0).rem_euclid(4.0);
            if y < 2.0 { y - 1.0 } else { 3.0 - y }
        }
        3 => (x * PI * 0.5).sin(),
        4 => {
            if x >= 0.0 {
                x.tanh()
            } else {
                (x * 0.5).tanh() * 0.6
            }
        }
        5 => x.abs().tanh() * 2.0 - 0.76,
        _ => x.tanh(),
    }
}

/// Waveshaper: one of several transfer curves.
///
/// Parameters: `[shape, drive dB, mix, output dB]`.
pub fn waveshaper(left: &mut [f32], right: &mut [f32], p: &[f32]) {
    let shape = param(p, 0, 0.0);
    let drive = db_to_gain(param(p, 1, 12.0));
    let mix = param(p, 2, 1.0).clamp(0.0, 1.0);
    let output = db_to_gain(param(p, 3, -6.0));
    for s in left.iter_mut().chain(right.iter_mut()) {
        let wet = shape_curve(shape, *s * drive);
        *s = (*s + (wet - *s) * mix) * output;
    }
}

/// Overdrive: asymmetric tube-like saturation followed by a tone filter.
///
/// Parameters: `[drive dB, tone Hz, bias, mix, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct Overdrive {
    tone: [Svf; 2],
}

/// The overdrive's transfer curve before the tone filter.
#[must_use]
pub fn overdrive_curve(drive_db: f32, bias: f32, x: f32) -> f32 {
    let g = db_to_gain(drive_db);
    (x * g + bias).tanh() - bias.tanh()
}

impl Overdrive {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let drive = param(p, 0, 18.0);
        let tone = param(p, 1, 3_000.0);
        let bias = param(p, 2, 0.1).clamp(0.0, 0.5);
        let mix = param(p, 3, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(p, 4, -6.0));
        for (filter, channel) in self.tone.iter_mut().zip([left, right]) {
            filter.set(tone, 0.1, ctx.sample_rate);
            for s in channel.iter_mut() {
                let wet = filter.process(overdrive_curve(drive, bias, *s), SvfMode::LowPass);
                *s = (*s + (wet - *s) * mix) * output;
            }
        }
    }
}

/// The soft clipper's curve: linear below the threshold, then bending
/// smoothly toward it.
#[must_use]
pub fn soft_clip_curve(threshold_db: f32, knee: f32, x: f32) -> f32 {
    let t = db_to_gain(threshold_db).max(1e-3);
    let start = t * (1.0 - knee.clamp(0.0, 1.0) * 0.9);
    let a = x.abs();
    if a <= start {
        x
    } else {
        let room = (t - start).max(1e-6);
        (start + room * ((a - start) / room).tanh()).copysign(x)
    }
}

/// Soft clipper. Parameters: `[threshold dB, knee, output dB]`.
pub fn soft_clipper(left: &mut [f32], right: &mut [f32], p: &[f32]) {
    let threshold = param(p, 0, -3.0);
    let knee = param(p, 1, 0.5);
    let output = db_to_gain(param(p, 2, 0.0));
    for s in left.iter_mut().chain(right.iter_mut()) {
        *s = soft_clip_curve(threshold, knee, *s) * output;
    }
}

/// DC removal: a gentle high-pass. Parameters: `[freq Hz]`.
#[derive(Debug, Clone, Default)]
pub struct DcBlock {
    state: [(f32, f32); 2],
}

impl DcBlock {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let freq = param(p, 0, 10.0).max(1.0);
        let r = (-TAU * freq / ctx.sample_rate.max(1.0)).exp();
        for (state, channel) in self.state.iter_mut().zip([left, right]) {
            for s in channel.iter_mut() {
                let (x1, y1) = *state;
                let y = *s - x1 + r * y1;
                *state = (*s, if y.is_finite() { y } else { 0.0 });
                *s = state.1;
            }
        }
    }
}

/// Ring modulator. Parameters: `[freq Hz, mix]`.
#[derive(Debug, Clone, Default)]
pub struct RingMod {
    phase: f32,
}

impl RingMod {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let step = param(p, 0, 440.0).max(0.0) / ctx.sample_rate.max(1.0);
        let mix = param(p, 1, 1.0).clamp(0.0, 1.0);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let carrier = (self.phase * TAU).sin();
            *l += (*l * carrier - *l) * mix;
            *r += (*r * carrier - *r) * mix;
            self.phase = (self.phase + step).fract();
        }
    }
}

/// Two all-pass chains whose outputs are 90° apart over most of the audio
/// band (a Hilbert transformer), from the public-domain coefficients by
/// Olli Niemitalo.
#[derive(Debug, Clone, Default)]
struct Hilbert {
    a: [[f32; 4]; 4],
    b: [[f32; 4]; 4],
    delayed: f32,
}

const HILBERT_A: [f32; 4] = [0.479_400_86, 0.876_218_5, 0.976_597_6, 0.997_499_9];
const HILBERT_B: [f32; 4] = [0.161_758_5, 0.733_015_4, 0.945_349_8, 0.990_599_2];

impl Hilbert {
    /// Runs `input` through four all-pass sections in z^-2; each state is
    /// `[x1, x2, y1, y2]`.
    fn chain(states: &mut [[f32; 4]; 4], coeffs: &[f32; 4], input: f32) -> f32 {
        let mut x = input;
        for (st, c) in states.iter_mut().zip(coeffs) {
            let c2 = c * c;
            let y = c2 * (x + st[3]) - st[1];
            *st = [x, st[0], y, st[2]];
            if !y.is_finite() {
                *st = [0.0; 4];
            }
            x = st[2];
        }
        x
    }

    /// The in-phase and quadrature parts of `input`.
    fn process(&mut self, input: f32) -> (f32, f32) {
        let i = Self::chain(&mut self.a, &HILBERT_A, input);
        let q = Self::chain(&mut self.b, &HILBERT_B, input);
        let delayed = self.delayed;
        self.delayed = i;
        (delayed, q)
    }
}

/// Frequency shifter: moves every frequency by the same number of hertz.
///
/// Parameters: `[shift Hz, mix]`.
#[derive(Debug, Clone, Default)]
pub struct FreqShift {
    hilbert: [Hilbert; 2],
    phase: f32,
}

impl FreqShift {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let step = param(p, 0, 100.0) / ctx.sample_rate.max(1.0);
        let mix = param(p, 1, 1.0).clamp(0.0, 1.0);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (c, s) = ((self.phase * TAU).cos(), (self.phase * TAU).sin());
            for (h, x) in self.hilbert.iter_mut().zip([l, r]) {
                let (i, q) = h.process(*x);
                let shifted = i * c + q * s;
                *x += (shifted - *x) * mix;
            }
            self.phase = (self.phase + step).rem_euclid(1.0);
        }
    }
}

/// Formant frequencies (Hz) of five vowels: a, e, i, o, u.
pub const VOWELS: [[f32; 3]; 5] = [
    [800.0, 1_150.0, 2_900.0],
    [400.0, 1_600.0, 2_700.0],
    [350.0, 1_700.0, 2_700.0],
    [450.0, 800.0, 2_830.0],
    [325.0, 700.0, 2_530.0],
];

/// Formant frequencies for a vowel position 0..4 (blending neighbours).
#[must_use]
pub fn formants(vowel: f32, shift_semitones: f32) -> [f32; 3] {
    let v = vowel.clamp(0.0, 4.0);
    let i = (v.floor() as usize).min(3);
    let t = v - i as f32;
    let scale = 2.0_f32.powf(shift_semitones / 12.0);
    let mut out = [0.0; 3];
    for (k, f) in out.iter_mut().enumerate() {
        *f = (VOWELS[i][k] * (1.0 - t) + VOWELS[i + 1][k] * t) * scale;
    }
    out
}

/// Formant (vowel) filter: three band-passes at a vowel's formants.
///
/// Parameters: `[vowel 0–4, shift st, resonance, mix]`.
#[derive(Debug, Clone, Default)]
pub struct Formant {
    bands: [[Svf; 3]; 2],
}

impl Formant {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let freqs = formants(param(p, 0, 0.0), param(p, 1, 0.0));
        let resonance = 0.4 + param(p, 2, 0.6).clamp(0.0, 1.0) * 0.5;
        let mix = param(p, 3, 1.0).clamp(0.0, 1.0);
        for (bands, channel) in self.bands.iter_mut().zip([left, right]) {
            for (band, f) in bands.iter_mut().zip(freqs) {
                band.set(f, resonance, ctx.sample_rate);
            }
            for s in channel.iter_mut() {
                let gains = [1.0, 0.7, 0.4];
                let wet: f32 = bands
                    .iter_mut()
                    .zip(gains)
                    .map(|(b, g)| b.process(*s, SvfMode::BandPass) * g)
                    .sum();
                *s += (wet * 0.7 - *s) * mix;
            }
        }
    }
}

/// Auto filter: a resonant filter swept by a tempo-synced LFO and by the
/// input's own level.
///
/// Parameters: `[mode, cutoff Hz, resonance, rate, lfo depth oct, envelope oct, mix]`.
#[derive(Debug, Clone, Default)]
pub struct AutoFilter {
    filters: [Svf; 2],
    follower: f32,
}

impl AutoFilter {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let mode = SvfMode::from_param(param(p, 0, 0.0));
        let cutoff = param(p, 1, 1_000.0);
        let resonance = param(p, 2, 0.5);
        let rate = SYNC_RATES
            .get(param(p, 3, 3.0).round().max(0.0) as usize)
            .copied()
            .unwrap_or(1.0);
        let depth = param(p, 4, 2.0);
        let envelope = param(p, 5, 0.0);
        let mix = param(p, 6, 1.0).clamp(0.0, 1.0);
        let sr = ctx.sample_rate.max(1.0);
        let beats_per_sample = ctx.tempo_bpm / 60.0 / f64::from(sr);
        let release = (-1.0 / (0.08 * sr)).exp();
        let mut beats = ctx.beats;
        for (i, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let level = (*l).abs().max((*r).abs());
            self.follower = level.max(self.follower * release);
            // Coefficients change every 16 samples; that is plenty for a sweep.
            if i % 16 == 0 {
                let lfo = ((beats / rate).fract() as f32 * TAU).sin();
                let octaves = lfo * depth + envelope * self.follower.min(1.0) * 4.0;
                let f = cutoff * 2.0_f32.powf(octaves);
                for filter in &mut self.filters {
                    filter.set(f, resonance, sr);
                }
            }
            beats += beats_per_sample;
            let wl = self.filters[0].process(*l, mode);
            let wr = self.filters[1].process(*r, mode);
            *l += (wl - *l) * mix;
            *r += (wr - *r) * mix;
        }
    }
}

/// Noise gate: silences the signal while it stays under a threshold.
///
/// Parameters: `[threshold dB, attack ms, hold ms, release ms, range dB]`.
#[derive(Debug, Clone, Default)]
pub struct NoiseGate {
    gain: f32,
    held: u32,
}

impl NoiseGate {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let threshold = db_to_gain(param(p, 0, -40.0));
        let sr = ctx.sample_rate.max(1.0);
        let attack = 1.0 - (-1.0 / (param(p, 1, 1.0).max(0.01) * 0.001 * sr)).exp();
        let hold = (param(p, 2, 20.0).max(0.0) * 0.001 * sr) as u32;
        let release = 1.0 - (-1.0 / (param(p, 3, 100.0).max(1.0) * 0.001 * sr)).exp();
        let floor = db_to_gain(param(p, 4, -80.0));
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let open = (*l).abs().max((*r).abs()) >= threshold;
            if open {
                self.held = hold;
            } else {
                self.held = self.held.saturating_sub(1);
            }
            let target = if open || self.held > 0 { 1.0 } else { floor };
            let coeff = if target > self.gain { attack } else { release };
            self.gain += (target - self.gain) * coeff;
            *l *= self.gain;
            *r *= self.gain;
        }
    }
}

/// Transient shaper: boosts or softens attacks and sustain separately, from
/// the difference between a fast and a slow envelope.
///
/// Parameters: `[attack −1–1, sustain −1–1, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct Transient {
    fast: f32,
    slow: f32,
}

impl Transient {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let attack = param(p, 0, 0.0).clamp(-1.0, 1.0);
        let sustain = param(p, 1, 0.0).clamp(-1.0, 1.0);
        let output = db_to_gain(param(p, 2, 0.0));
        let sr = ctx.sample_rate.max(1.0);
        let fast = 1.0 - (-1.0 / (0.001 * sr)).exp();
        let slow = 1.0 - (-1.0 / (0.05 * sr)).exp();
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let level = (*l).abs().max((*r).abs());
            self.fast += (level - self.fast) * fast;
            self.slow += (level - self.slow) * slow;
            let punch = ((self.fast - self.slow) / (self.slow + 1e-4)).clamp(0.0, 2.0);
            let body = (self.slow / (self.fast + 1e-4)).clamp(0.0, 1.0);
            let gain = (1.0 + attack * punch) * (1.0 + sustain * body * 0.8);
            let gain = gain.clamp(0.0, 8.0) * output;
            *l *= gain;
            *r *= gain;
        }
    }
}

/// Four-tap tempo-synced delay with feedback, damping, and a pan and
/// low/high cut per tap.
///
/// Parameters: `[tap1 beats, tap1 level, …, tap4 beats, tap4 level,
/// feedback, mix, damping, tap1–4 pan, tap1–4 low cut Hz, tap1–4 high cut
/// Hz]`.
#[derive(Debug, Clone)]
pub struct MultiTap {
    lines: [Vec<f32>; 2],
    write: usize,
    damp: [f32; 2],
    /// Per tap: the low-cut filter's low-pass state and the high-cut state.
    tone: [[f32; 2]; 4],
}

/// A multi-tap delay tap's settings for one block.
#[derive(Debug, Clone, Copy)]
struct Tap {
    delay: usize,
    gains: [f32; 2],
    /// One-pole coefficients; `None` when the filter is off.
    low_cut: Option<f32>,
    high_cut: Option<f32>,
}

/// Lowest low cut and highest high cut: at these the filters are off.
const TAP_LOW_CUT_OFF: f32 = 20.0;
const TAP_HIGH_CUT_OFF: f32 = 20_000.0;

fn one_pole(freq: f32, sample_rate: f32) -> f32 {
    1.0 - (-TAU * freq / sample_rate.max(1.0)).exp()
}

impl MultiTap {
    /// Allocates four seconds of delay. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * 4.0) as usize + 2;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            damp: [0.0; 2],
            tone: [[0.0; 2]; 4],
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
        self.damp = [0.0; 2];
        self.tone = [[0.0; 2]; 4];
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.lines[0].len();
        if len < 4 {
            return;
        }
        let samples_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) as f32 * ctx.sample_rate;
        let sr = ctx.sample_rate;
        let taps: [Tap; 4] = std::array::from_fn(|t| {
            let beats = param(p, t * 2, [0.5, 0.75, 1.0, 1.5][t]);
            let level = param(p, t * 2 + 1, [0.6, 0.45, 0.3, 0.2][t]).clamp(0.0, 1.0);
            let pan = param(p, 11 + t, if t % 2 == 0 { -1.0 } else { 1.0 }).clamp(-1.0, 1.0);
            let angle = (pan + 1.0) * PI * 0.25;
            let low = param(p, 15 + t, TAP_LOW_CUT_OFF);
            let high = param(p, 19 + t, TAP_HIGH_CUT_OFF);
            Tap {
                delay: ((beats * samples_per_beat) as usize).clamp(1, len - 1),
                gains: [angle.cos() * level, angle.sin() * level],
                low_cut: (low > TAP_LOW_CUT_OFF).then(|| one_pole(low, sr)),
                high_cut: (high < TAP_HIGH_CUT_OFF).then(|| one_pole(high, sr)),
            }
        });
        let feedback = param(p, 8, 0.3).clamp(0.0, 0.9);
        let mix = param(p, 9, 0.35).clamp(0.0, 1.0);
        let damping = param(p, 10, 0.3).clamp(0.0, 0.95);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut wet = [0.0_f32; 2];
            for (tap, tone) in taps.iter().zip(self.tone.iter_mut()) {
                let read = (self.write + len - tap.delay) % len;
                let mut mono = (self.lines[0].get(read).copied().unwrap_or(0.0)
                    + self.lines[1].get(read).copied().unwrap_or(0.0))
                    * 0.5;
                if let Some(a) = tap.low_cut {
                    tone[0] = flush_denormal(tone[0] + (mono - tone[0]) * a);
                    mono -= tone[0];
                }
                if let Some(a) = tap.high_cut {
                    tone[1] = flush_denormal(tone[1] + (mono - tone[1]) * a);
                    mono = tone[1];
                }
                wet[0] += mono * tap.gains[0];
                wet[1] += mono * tap.gains[1];
            }
            for (ch, (input, w)) in [*l, *r].into_iter().zip(wet).enumerate() {
                self.damp[ch] =
                    flush_denormal(self.damp[ch] + (w - self.damp[ch]) * (1.0 - damping));
                let fed = input + self.damp[ch] * feedback;
                if let Some(slot) = self.lines[ch].get_mut(self.write) {
                    *slot = if fed.is_finite() {
                        flush_denormal(fed)
                    } else {
                        0.0
                    };
                }
            }
            *l += wet[0] * mix;
            *r += wet[1] * mix;
            self.write = (self.write + 1) % len;
        }
    }
}

/// Centre frequencies of the graphic EQ's seven bands.
pub const GRAPHIC_BANDS: [f32; 7] = [63.0, 160.0, 400.0, 1_000.0, 2_500.0, 6_300.0, 16_000.0];

/// The biquads of the graphic EQ for its gains.
fn graphic_biquads(p: &[f32], sample_rate: f32) -> [Biquad; 7] {
    std::array::from_fn(|i| {
        let mut b = Biquad::default();
        b.set_peak(GRAPHIC_BANDS[i], param(p, i, 0.0), 1.1, sample_rate);
        b
    })
}

/// Seven-band graphic EQ. Parameters: `[seven gains dB, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct GraphicEq {
    bands: [[Biquad; 7]; 2],
}

impl GraphicEq {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let output = db_to_gain(param(p, 7, 0.0));
        for (bands, channel) in self.bands.iter_mut().zip([left, right]) {
            let fresh = graphic_biquads(p, ctx.sample_rate);
            for (b, f) in bands.iter_mut().zip(fresh) {
                b.copy_coefficients(&f);
            }
            for s in channel.iter_mut() {
                let mut x = *s;
                for b in bands.iter_mut() {
                    x = b.process(x);
                }
                *s = x * output;
            }
        }
    }
}

/// Magnitude in dB of the graphic EQ.
#[must_use]
pub fn graphic_eq_db(p: &[f32], freq: f32, sample_rate: f32) -> f32 {
    graphic_biquads(p, sample_rate)
        .iter()
        .map(|b| b.magnitude_db(freq, sample_rate))
        .sum::<f32>()
        + param(p, 7, 0.0)
}

/// The biquads of the parametric EQ: band 1 a low shelf, 2–6 peaks, 7 a
/// high shelf. Parameters come in (freq, gain, q) triples.
fn parametric_biquads(p: &[f32], sample_rate: f32) -> [Biquad; 7] {
    std::array::from_fn(|i| {
        let (f, g, q) = (
            param(p, i * 3, 1_000.0),
            param(p, i * 3 + 1, 0.0),
            param(p, i * 3 + 2, 0.7),
        );
        let mut b = Biquad::default();
        match i {
            0 => b.set_low_shelf(f, g, sample_rate),
            6 => b.set_high_shelf(f, g, sample_rate),
            _ => b.set_peak(f, g, q, sample_rate),
        }
        b
    })
}

/// Seven-band parametric EQ. Parameters: `[(freq, gain, q) × 7, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct ParametricEq {
    bands: [[Biquad; 7]; 2],
}

impl ParametricEq {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let output = db_to_gain(param(p, 21, 0.0));
        for (bands, channel) in self.bands.iter_mut().zip([left, right]) {
            let fresh = parametric_biquads(p, ctx.sample_rate);
            for (b, f) in bands.iter_mut().zip(fresh) {
                b.copy_coefficients(&f);
            }
            for s in channel.iter_mut() {
                let mut x = *s;
                for b in bands.iter_mut() {
                    x = b.process(x);
                }
                *s = x * output;
            }
        }
    }
}

/// Magnitude in dB of the parametric EQ.
#[must_use]
pub fn parametric_eq_db(p: &[f32], freq: f32, sample_rate: f32) -> f32 {
    parametric_biquads(p, sample_rate)
        .iter()
        .map(|b| b.magnitude_db(freq, sample_rate))
        .sum::<f32>()
        + param(p, 21, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vowel_from_a_to_u_is_reachable() {
        for (i, vowel) in VOWELS.iter().enumerate() {
            let got = formants(i as f32, 0.0);
            for (g, w) in got.iter().zip(vowel) {
                assert!((g - w).abs() < 1e-3, "vowel {i}: {got:?} vs {vowel:?}");
            }
        }
        // Halfway between o and u blends the two.
        let mid = formants(3.5, 0.0);
        assert!((mid[0] - (VOWELS[3][0] + VOWELS[4][0]) / 2.0).abs() < 1e-3);
        // Out-of-range values stay at the ends.
        assert!((formants(9.0, 0.0)[0] - VOWELS[4][0]).abs() < 1e-3);
    }

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 48_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    fn sine(hz: f32, n: usize, level: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f32 * hz * TAU / 48_000.0).sin() * level)
            .collect()
    }

    fn rms(x: &[f32]) -> f32 {
        (x.iter().map(|s| s * s).sum::<f32>() / x.len().max(1) as f32).sqrt()
    }

    #[test]
    fn curves_are_bounded_and_shaped() {
        for shape in 0..6 {
            for i in -40..=40 {
                let y = shape_curve(shape as f32, i as f32 / 10.0);
                assert!(y.is_finite() && y.abs() <= 1.3, "shape {shape}: {y}");
            }
        }
        assert!(
            (soft_clip_curve(-6.0, 0.5, 0.1) - 0.1).abs() < 1e-6,
            "linear below"
        );
        assert!(soft_clip_curve(-6.0, 0.5, 10.0) <= db_to_gain(-6.0) + 1e-4);
        assert!(
            overdrive_curve(18.0, 0.2, 0.0).abs() < 1e-6,
            "no offset at rest"
        );
    }

    #[test]
    fn eqs_shape_the_response() {
        let mut graphic = [0.0; 8];
        graphic[3] = 12.0;
        assert!((graphic_eq_db(&graphic, 1_000.0, 48_000.0) - 12.0).abs() < 1.5);
        assert!(graphic_eq_db(&graphic, 63.0, 48_000.0).abs() < 1.0);
        let mut param_eq = [0.0; 24];
        for band in 0..7 {
            param_eq[band * 3] = [80.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0][band];
            param_eq[band * 3 + 2] = 1.0;
        }
        param_eq[4] = -9.0; // band 2 (200 Hz) cut
        assert!((parametric_eq_db(&param_eq, 200.0, 48_000.0) + 9.0).abs() < 1.0);
        let mut eq = ParametricEq::default();
        let mut l = sine(200.0, 9_600, 0.5);
        let mut r = l.clone();
        eq.process(&mut l, &mut r, &param_eq, ctx());
        assert!(rms(&l[4_800..]) < 0.5 * 0.71 * 0.5, "cut by ~9 dB");
    }

    #[test]
    fn gate_and_transient_shaper() {
        let mut gate = NoiseGate::default();
        let mut l = sine(200.0, 9_600, 0.001);
        l.extend(sine(200.0, 9_600, 0.5));
        let mut r = l.clone();
        gate.process(&mut l, &mut r, &[-40.0, 1.0, 10.0, 50.0, -80.0], ctx());
        assert!(rms(&l[4_800..9_600]) < 1e-5, "quiet part gated");
        assert!(rms(&l[14_400..]) > 0.3, "loud part passes");
        let mut shaper = Transient::default();
        let mut l: Vec<f32> = (0..9_600)
            .map(|i| if i < 4_800 { 0.0 } else { 0.5 })
            .collect();
        let mut r = l.clone();
        shaper.process(&mut l, &mut r, &[1.0, 0.0, 0.0], ctx());
        assert!(l[4_850] > 0.6, "attack boosted: {}", l[4_850]);
        assert!((l[9_500] - 0.5).abs() < 0.1, "sustain unchanged");
    }

    #[test]
    fn frequency_shifter_moves_a_tone() {
        let mut shifter = FreqShift::default();
        let mut l = sine(1_000.0, 48_000, 0.5);
        let mut r = l.clone();
        shifter.process(&mut l, &mut r, &[200.0, 1.0], ctx());
        let tail = &l[4_800..];
        let crossings = tail
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        let hz = crossings as f32 * 48_000.0 / tail.len() as f32;
        assert!((hz - 1_200.0).abs() < 25.0, "shifted to {hz}");
    }

    #[test]
    fn the_rest_stay_finite_and_change_the_sound() {
        let input = sine(220.0, 24_000, 0.5);
        let check = |name: &str, out: &[f32]| {
            assert!(out.iter().all(|s| s.is_finite() && s.abs() < 4.0), "{name}");
            let diff: f32 = out.iter().zip(&input).map(|(a, b)| (a - b).abs()).sum();
            assert!(diff > 1.0, "{name} changes the sound");
        };
        let run = |f: &mut dyn FnMut(&mut [f32], &mut [f32])| {
            let (mut l, mut r) = (input.clone(), input.clone());
            f(&mut l, &mut r);
            l
        };
        check(
            "bass boost",
            &run(&mut |l, r| BassBoost::default().process(l, r, &[100.0, 12.0, 0.5], ctx())),
        );
        check(
            "overdrive",
            &run(&mut |l, r| {
                Overdrive::default().process(l, r, &[24.0, 2_000.0, 0.2, 1.0, -6.0], ctx())
            }),
        );
        check(
            "waveshaper",
            &run(&mut |l, r| waveshaper(l, r, &[2.0, 12.0, 1.0, 0.0])),
        );
        check(
            "ring",
            &run(&mut |l, r| RingMod::default().process(l, r, &[300.0, 1.0], ctx())),
        );
        check(
            "formant",
            &run(&mut |l, r| Formant::default().process(l, r, &[1.5, 0.0, 0.6, 1.0], ctx())),
        );
        check(
            "auto filter",
            &run(&mut |l, r| {
                AutoFilter::default().process(l, r, &[0.0, 400.0, 0.6, 3.0, 2.0, 1.0, 1.0], ctx())
            }),
        );
        let mut taps = MultiTap::new(48_000.0);
        check(
            "multi-tap",
            &run(&mut |l, r| {
                taps.process(
                    l,
                    r,
                    &[0.25, 0.8, 0.5, 0.6, 0.75, 0.4, 1.0, 0.3, 0.5, 0.5, 0.2],
                    ctx(),
                )
            }),
        );
        let mut dc = DcBlock::default();
        let (mut l, mut r) = (vec![0.5; 48_000], vec![0.5; 48_000]);
        dc.process(&mut l, &mut r, &[20.0], ctx());
        assert!(l[47_000].abs() < 0.01, "DC removed");
    }

    fn tap_params(extra: &[(usize, f32)]) -> Vec<f32> {
        let mut p = musiccraft_params::EffectKind::MultiTap.defaults().to_vec();
        // One tap at a quarter beat (125 ms at 120 BPM), no feedback, all wet.
        for (i, v) in [
            (1, 1.0),
            (3, 0.0),
            (5, 0.0),
            (7, 0.0),
            (0, 0.25),
            (8, 0.0),
            (9, 1.0),
        ] {
            p[i] = v;
        }
        for (i, v) in extra {
            p[*i] = *v;
        }
        p
    }

    #[test]
    fn multi_tap_pan_places_each_tap() {
        // Default pan: tap 1 hard left, as before per-tap pan existed.
        let mut taps = MultiTap::new(48_000.0);
        let (mut l, mut r) = (vec![0.0; 7_000], vec![0.0; 7_000]);
        l[0] = 1.0;
        r[0] = 1.0;
        taps.process(&mut l, &mut r, &tap_params(&[]), ctx());
        assert!((l[6_000] - 1.0).abs() < 1e-6 && r[6_000].abs() < 1e-6);
        // Centred: equal power on both sides.
        let mut taps = MultiTap::new(48_000.0);
        let (mut l, mut r) = (vec![0.0; 7_000], vec![0.0; 7_000]);
        l[0] = 1.0;
        r[0] = 1.0;
        taps.process(&mut l, &mut r, &tap_params(&[(11, 0.0)]), ctx());
        assert!((l[6_000] - r[6_000]).abs() < 1e-6);
        assert!((l[6_000] - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-4);
    }

    #[test]
    fn multi_tap_filters_shape_only_their_tap() {
        let echo_level = |extra: &[(usize, f32)], hz: f32| {
            let mut taps = MultiTap::new(48_000.0);
            let mut l = sine(hz, 24_000, 0.5);
            let mut r = l.clone();
            let dry = l.clone();
            taps.process(&mut l, &mut r, &tap_params(extra), ctx());
            let wet: Vec<f32> = l.iter().zip(&dry).map(|(o, d)| o - d).collect();
            wet[12_000..].iter().fold(0.0_f32, |m, s| m.max(s.abs()))
        };
        let open = echo_level(&[], 8_000.0);
        let cut = echo_level(&[(19, 1_000.0)], 8_000.0);
        assert!(open > 0.4 && cut < open * 0.2, "high cut: {open} vs {cut}");
        let low_open = echo_level(&[], 60.0);
        let low_cut = echo_level(&[(15, 1_500.0)], 60.0);
        assert!(low_cut < low_open * 0.1, "low cut: {low_open} vs {low_cut}");
    }
}
