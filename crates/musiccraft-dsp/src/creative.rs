//! Multiband compressor, vocoder, pitch shifter and stutter. All allocation
//! happens in `new`.

use std::f32::consts::PI;

use crate::modulation::SYNC_RATES;
use crate::{ProcessContext, Svf, SvfMode, db_to_gain, gain_to_db};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

fn coefficient(ms: f32, sample_rate: f32) -> f32 {
    1.0 - (-1.0 / (ms.max(0.01) * 0.001 * sample_rate.max(1.0))).exp()
}

/// Static gain (dB) of a compressor band for an input level.
#[must_use]
pub fn band_gain_db(input_db: f32, threshold_db: f32, ratio: f32) -> f32 {
    let over = (input_db - threshold_db).max(0.0);
    over / ratio.max(1.0) - over
}

/// A Linkwitz–Riley (4th order) crossover: two cascaded Butterworth
/// stages; low and high add back up to an all-pass of the input.
#[derive(Debug, Clone, Default)]
struct Crossover {
    first: Svf,
    low: Svf,
    high: Svf,
}

impl Crossover {
    fn set(&mut self, freq: f32, sample_rate: f32) {
        for f in [&mut self.first, &mut self.low, &mut self.high] {
            f.set_q(freq, std::f32::consts::FRAC_1_SQRT_2, sample_rate);
        }
    }

    fn process(&mut self, x: f32) -> (f32, f32) {
        let (lp, hp) = self.first.process_split(x);
        (self.low.process_split(lp).0, self.high.process_split(hp).1)
    }
}

/// One channel's three-way split. The low band also passes through the upper
/// crossover's all-pass so all three stay in phase and sum flat.
#[derive(Debug, Clone, Default)]
pub(crate) struct Split {
    lower: Crossover,
    upper: Crossover,
    align: Crossover,
}

impl Split {
    pub(crate) fn set(&mut self, f1: f32, f2: f32, sample_rate: f32) {
        let f2 = f2.max(f1 * 1.1);
        self.lower.set(f1, sample_rate);
        self.upper.set(f2, sample_rate);
        self.align.set(f2, sample_rate);
    }

    pub(crate) fn process(&mut self, x: f32) -> [f32; 3] {
        let (low, rest) = self.lower.process(x);
        let (mid, high) = self.upper.process(rest);
        let (a, b) = self.align.process(low);
        [a + b, mid, high]
    }
}

/// Three-band compressor.
///
/// Parameters: `[low/mid Hz, mid/high Hz, (threshold dB, ratio, gain dB) × 3,
/// attack ms, release ms, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct MultibandCompressor {
    split: [Split; 2],
    envelope: [f32; 3],
}

impl MultibandCompressor {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let sr = ctx.sample_rate;
        for split in &mut self.split {
            split.set(param(p, 0, 200.0), param(p, 1, 2_500.0), sr);
        }
        let bands: [(f32, f32, f32); 3] = std::array::from_fn(|b| {
            (
                param(p, 2 + b * 3, -18.0),
                param(p, 3 + b * 3, 3.0),
                db_to_gain(param(p, 4 + b * 3, 0.0)),
            )
        });
        let attack = coefficient(param(p, 11, 10.0), sr);
        let release = coefficient(param(p, 12, 150.0), sr);
        let output = db_to_gain(param(p, 13, 0.0));
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let [sl, sr_] = &mut self.split;
            let lb = sl.process(*l);
            let rb = sr_.process(*r);
            let mut out = (0.0, 0.0);
            for (b, ((threshold, ratio, makeup), env)) in
                bands.iter().zip(self.envelope.iter_mut()).enumerate()
            {
                let (x, y) = (
                    lb.get(b).copied().unwrap_or(0.0),
                    rb.get(b).copied().unwrap_or(0.0),
                );
                let level = x.abs().max(y.abs());
                let coeff = if level > *env { attack } else { release };
                *env += (level - *env) * coeff;
                let gain = db_to_gain(band_gain_db(gain_to_db(*env), *threshold, *ratio)) * makeup;
                out.0 += x * gain;
                out.1 += y * gain;
            }
            *l = out.0 * output;
            *r = out.1 * output;
        }
    }
}

/// Number of vocoder bands.
pub const VOCODER_BANDS: usize = 16;

/// Most vocoder bands.
pub const MAX_VOCODER_BANDS: usize = 64;

/// The vocoder's band count and range from its parameters: `(bands, low Hz,
/// high Hz)`.
#[must_use]
pub fn vocoder_layout(p: &[f32]) -> (usize, f32, f32) {
    let bands =
        (param(p, 8, VOCODER_BANDS as f32).round().max(2.0) as usize).min(MAX_VOCODER_BANDS);
    let low = param(p, 9, 120.0).clamp(20.0, 2_000.0);
    let high = param(p, 10, 7_500.0).clamp(low * 2.0, 20_000.0);
    (bands, low, high)
}

/// Centre frequency of band `i` of `bands`, log-spaced from `low` to `high`.
#[must_use]
pub fn vocoder_freq(i: usize, bands: usize, low: f32, high: f32) -> f32 {
    low * (high / low).powf(i as f32 / (bands.max(2) - 1) as f32)
}

/// Channel vocoder: this track is the carrier (usually a synth); the
/// sidechain is the modulator (usually a voice) whose band levels shape it.
///
/// Parameters: `[sidechain, bandwidth, attack ms, release ms, shift st, noise,
/// mix, output dB, bands, low Hz, high Hz]`.
#[derive(Debug, Clone)]
pub struct Vocoder {
    analysis: [Svf; MAX_VOCODER_BANDS],
    synthesis: [[Svf; MAX_VOCODER_BANDS]; 2],
    envelopes: [f32; MAX_VOCODER_BANDS],
    seed: u32,
}

impl Default for Vocoder {
    fn default() -> Self {
        Self {
            analysis: std::array::from_fn(|_| Svf::default()),
            synthesis: std::array::from_fn(|_| std::array::from_fn(|_| Svf::default())),
            envelopes: [0.0; MAX_VOCODER_BANDS],
            seed: 0,
        }
    }
}

impl Vocoder {
    /// Processes a block in place; with no sidechain the output is silent
    /// apart from the dry mix.
    pub fn process_keyed(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        p: &[f32],
        ctx: ProcessContext,
        key: Option<(&[f32], &[f32])>,
    ) {
        let sr = ctx.sample_rate;
        let q = 0.5 + param(p, 1, 0.5).clamp(0.0, 1.0) * 0.45;
        let attack = coefficient(param(p, 2, 5.0), sr);
        let release = coefficient(param(p, 3, 60.0), sr);
        let shift = 2.0_f32.powf(param(p, 4, 0.0) / 12.0);
        let noise = param(p, 5, 0.1).clamp(0.0, 1.0);
        let mix = param(p, 6, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(p, 7, 0.0));
        let (bands, low, high) = vocoder_layout(p);
        // More bands are narrower, so they overlap about as much as 16 do.
        let q = (1.0 - (1.0 - q) * (VOCODER_BANDS as f32 / bands as f32).sqrt()).min(0.99);
        for (i, band) in self.analysis.iter_mut().take(bands).enumerate() {
            band.set(vocoder_freq(i, bands, low, high), q, sr);
        }
        for bank in &mut self.synthesis {
            for (i, band) in bank.iter_mut().take(bands).enumerate() {
                band.set(vocoder_freq(i, bands, low, high) * shift, q, sr);
            }
        }
        if self.seed == 0 {
            self.seed = 0x9e37_79b9;
        }
        // Band-pass gain shrinks as the bands narrow; make up for it.
        let makeup = (4.0 + q * 10.0) * (VOCODER_BANDS as f32 / bands as f32).sqrt();
        for (i, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            let modulator = key
                .map(|(kl, kr)| {
                    (kl.get(i).copied().unwrap_or(0.0) + kr.get(i).copied().unwrap_or(0.0)) * 0.5
                })
                .unwrap_or(0.0);
            self.seed ^= self.seed << 13;
            self.seed ^= self.seed >> 17;
            self.seed ^= self.seed << 5;
            let hiss = (self.seed as f32 / u32::MAX as f32 * 2.0 - 1.0) * noise * 0.5;
            let mut wet = [0.0_f32; 2];
            for (b, (band, env)) in self
                .analysis
                .iter_mut()
                .zip(&mut self.envelopes)
                .take(bands)
                .enumerate()
            {
                let level = band.process(modulator, SvfMode::BandPass).abs();
                let coeff = if level > *env { attack } else { release };
                *env += (level - *env) * coeff;
                for (ch, (bank, x)) in self.synthesis.iter_mut().zip([*l, *r]).enumerate() {
                    if let (Some(s), Some(w)) = (bank.get_mut(b), wet.get_mut(ch)) {
                        *w += s.process(x + hiss, SvfMode::BandPass) * *env;
                    }
                }
            }
            *l = (*l * (1.0 - mix) + wet[0] * makeup * mix) * output;
            *r = (*r * (1.0 - mix) + wet[1] * makeup * mix) * output;
        }
    }
}

/// Pitch shifter: two read heads sweep a short delay at a different speed
/// than it is written, crossfading so each jump back is hidden.
///
/// Parameters: `[pitch st, fine cents, window ms, mix]`.
#[derive(Debug, Clone)]
pub struct PitchShifter {
    lines: [Vec<f32>; 2],
    write: usize,
    phase: f32,
}

impl PitchShifter {
    /// Allocates 120 ms of delay. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * 0.12) as usize + 4;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            phase: 0.0,
        }
    }

    /// Clears the delay.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
    }

    fn read(line: &[f32], position: f32) -> f32 {
        let len = line.len();
        if len == 0 {
            return 0.0;
        }
        let i = position.floor();
        let t = position - i;
        let a = line.get(i as usize % len).copied().unwrap_or(0.0);
        let b = line.get((i as usize + 1) % len).copied().unwrap_or(0.0);
        a + (b - a) * t
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let ratio = 2.0_f32.powf((param(p, 0, 0.0) + param(p, 1, 0.0) / 100.0) / 12.0);
        let window = param(p, 2, 50.0) * 0.001 * ctx.sample_rate;
        let mix = param(p, 3, 1.0).clamp(0.0, 1.0);
        self.shift(left, right, ratio, window, mix);
    }

    /// Shifts a block by a frequency `ratio`, with read heads sweeping
    /// `window` samples (clamped to what the delay holds), mixed with the dry
    /// signal by `mix`.
    pub fn shift(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        ratio: f32,
        window: f32,
        mix: f32,
    ) {
        let len = self.lines[0].len();
        if len < 4 {
            return;
        }
        let window = if window.is_finite() { window } else { 8.0 };
        let window = window.clamp(8.0, (len - 3) as f32);
        let ratio = if ratio.is_finite() { ratio } else { 1.0 };
        // The delay shrinks by (ratio − 1) samples per sample.
        let step = (1.0 - ratio) / window;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            for (line, x) in self.lines.iter_mut().zip([*l, *r]) {
                if let Some(slot) = line.get_mut(self.write) {
                    *slot = x;
                }
            }
            let mut wet = [0.0_f32; 2];
            for head in [0.0, 0.5] {
                let phase = (self.phase + head).fract();
                let delay = phase * window + 1.0;
                let position = (self.write as f32 - delay).rem_euclid(len as f32);
                let gain = (phase * PI).sin().powi(2);
                for (w, line) in wet.iter_mut().zip(&self.lines) {
                    *w += Self::read(line, position) * gain;
                }
            }
            *l += (wet[0] - *l) * mix;
            *r += (wet[1] - *r) * mix;
            self.phase = (self.phase + step).rem_euclid(1.0);
            self.write = (self.write + 1) % len;
        }
    }
}

/// Stutter: near the end of every period it repeats a short slice of what
/// just played, in time with the song.
///
/// Parameters: `[slice (sync rate), every (sync rate), portion, decay, mix]`.
#[derive(Debug, Clone)]
pub struct Stutter {
    lines: [Vec<f32>; 2],
    write: usize,
}

impl Stutter {
    /// Allocates eight seconds of history. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * 8.0) as usize + 2;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
        }
    }

    /// Clears the history.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.lines[0].len();
        if len < 2 {
            return;
        }
        let rate = |i: usize, default: f64| {
            SYNC_RATES
                .get(param(p, i, 0.0).round().max(0.0) as usize)
                .copied()
                .unwrap_or(default)
        };
        let slice_beats = rate(0, 0.25);
        let every = rate(1, 4.0).max(slice_beats);
        let portion = f64::from(param(p, 2, 0.25).clamp(0.0, 1.0));
        let decay = param(p, 3, 1.0).clamp(0.0, 1.0);
        let mix = param(p, 4, 1.0).clamp(0.0, 1.0);
        let samples_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) * f64::from(ctx.sample_rate);
        let slice = ((slice_beats * samples_per_beat) as usize).clamp(1, len - 1);
        let start = every * (1.0 - portion);
        let mut beats = ctx.beats;
        let step = 1.0 / samples_per_beat;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            for (line, x) in self.lines.iter_mut().zip([*l, *r]) {
                if let Some(slot) = line.get_mut(self.write) {
                    *slot = x;
                }
            }
            let into = beats.rem_euclid(every);
            if portion > 0.0 && into >= start {
                let elapsed = (((into - start) * samples_per_beat) as usize).min(len - 1);
                let repeats = elapsed / slice;
                // Replay from where the stutter began, one slice at a time.
                let back = (repeats * slice).min(len - 1);
                let read = (self.write + len - back) % len;
                let gain = decay.powi(repeats.min(64) as i32);
                let wet = [
                    self.lines[0].get(read).copied().unwrap_or(0.0) * gain,
                    self.lines[1].get(read).copied().unwrap_or(0.0) * gain,
                ];
                *l += (wet[0] - *l) * mix;
                *r += (wet[1] - *r) * mix;
            }
            beats += step;
            self.write = (self.write + 1) % len;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

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

    fn pitch(x: &[f32]) -> f32 {
        // Autocorrelation peak between 50 and 2000 Hz.
        let best = (24..960)
            .max_by(|a, b| {
                let c = |lag: usize| x.iter().zip(&x[lag..]).map(|(p, q)| p * q).sum::<f32>();
                c(*a).total_cmp(&c(*b))
            })
            .unwrap_or(1);
        48_000.0 / best as f32
    }

    #[test]
    fn multiband_is_transparent_until_it_compresses() {
        let mut mb = MultibandCompressor::default();
        let input = sine(440.0, 24_000, 0.1);
        let (mut l, mut r) = (input.clone(), input.clone());
        let mut p = [0.0; 14];
        p[0] = 200.0;
        p[1] = 2_500.0;
        for b in 0..3 {
            p[2 + b * 3] = 0.0; // threshold
            p[3 + b * 3] = 4.0;
        }
        p[11] = 10.0;
        p[12] = 100.0;
        mb.process(&mut l, &mut r, &p, ctx());
        assert!((rms(&l[12_000..]) - rms(&input[12_000..])).abs() < 0.005);
        // Now squash the mids hard.
        p[5] = -40.0;
        p[6] = 20.0;
        let (mut l, mut r) = (sine(1_000.0, 24_000, 0.5), sine(1_000.0, 24_000, 0.5));
        mb.process(&mut l, &mut r, &p, ctx());
        assert!(rms(&l[12_000..]) < 0.5 * 0.71 * 0.3, "mids compressed");
    }

    #[test]
    fn pitch_shifter_moves_up_an_octave() {
        let mut shifter = PitchShifter::new(48_000.0);
        let (mut l, mut r) = (sine(220.0, 24_000, 0.5), sine(220.0, 24_000, 0.5));
        shifter.process(&mut l, &mut r, &[12.0, 0.0, 40.0, 1.0], ctx());
        let hz = pitch(&l[12_000..16_000]);
        assert!((hz - 440.0).abs() < 15.0, "shifted to {hz}");
    }

    #[test]
    fn vocoder_needs_a_modulator() {
        let mut vocoder = Vocoder::default();
        let carrier: Vec<f32> = (0..24_000)
            .map(|i| ((i % 200) as f32 / 100.0) - 1.0)
            .collect();
        let voice = sine(500.0, 24_000, 0.5);
        let (mut l, mut r) = (carrier.clone(), carrier.clone());
        let p = [1.0, 0.5, 5.0, 60.0, 0.0, 0.0, 1.0, 0.0];
        vocoder.process_keyed(&mut l, &mut r, &p, ctx(), Some((&voice, &voice)));
        let loud = rms(&l[12_000..]);
        let (mut l, mut r) = (carrier.clone(), carrier);
        Vocoder::default().process_keyed(&mut l, &mut r, &p, ctx(), None);
        assert!(loud > 0.05, "vocoded: {loud}");
        assert!(rms(&l[12_000..]) < 0.01, "silent without a modulator");
    }

    #[test]
    fn vocoder_band_count_keeps_a_similar_level() {
        let carrier: Vec<f32> = (0..24_000)
            .map(|i| ((i % 200) as f32 / 100.0) - 1.0)
            .collect();
        let voice = sine(500.0, 24_000, 0.5);
        let level = |bands: f32| {
            let mut vocoder = Vocoder::default();
            let p = [
                1.0, 0.5, 5.0, 60.0, 0.0, 0.0, 1.0, 0.0, bands, 120.0, 7_500.0,
            ];
            let (mut l, mut r) = (carrier.clone(), carrier.clone());
            vocoder.process_keyed(&mut l, &mut r, &p, ctx(), Some((&voice, &voice)));
            assert!(l.iter().all(|s| s.is_finite()));
            rms(&l[12_000..])
        };
        let (few, standard, many) = (level(4.0), level(16.0), level(64.0));
        for got in [few, many] {
            assert!(
                got > standard * 0.25 && got < standard * 4.0,
                "{few} {standard} {many}"
            );
        }
    }

    #[test]
    fn stutter_repeats_the_slice() {
        let mut stutter = Stutter::new(48_000.0);
        // A ramp makes repeats visible: during the stutter the output jumps
        // back every slice.
        let n = 96_000; // one bar at 120 bpm
        let mut l: Vec<f32> = (0..n).map(|i| i as f32 / n as f32).collect();
        let mut r = l.clone();
        // slice 1/16 (index 1), every bar (5), last quarter, no decay.
        stutter.process(&mut l, &mut r, &[1.0, 5.0, 0.25, 1.0, 1.0], ctx());
        let slice = 6_000; // a sixteenth at 120 bpm
        let start = 72_000;
        assert!(
            (l[start + 10] - l[start + slice + 10]).abs() < 1e-6,
            "repeats"
        );
        assert!((l[1_000] - 1_000.0 / n as f32).abs() < 1e-6, "dry before");
    }
}
