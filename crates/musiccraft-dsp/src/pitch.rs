//! Pitch detection, the pitch corrector and the tuner. All allocation
//! happens in `new`.
//!
//! The detector is the YIN method (de Cheveigné and Kawahara, "YIN, a
//! fundamental frequency estimator for speech and music", JASA 2002): the
//! cumulative mean normalized difference function of a short frame, its
//! first dip below a threshold, refined by parabolic interpolation. It runs
//! on a low-passed, decimated copy of the input so its cost stays small.

use musiccraft_params::CORRECTION_SCALES;

use crate::creative::PitchShifter;
use crate::{ProcessContext, Svf, SvfMode};

/// Rate (Hz) the detector aims to analyse at; the input is decimated by a
/// whole factor to get near it.
const ANALYSIS_RATE: f32 = 12_000.0;
/// Integration window of the difference function, in analysis samples.
const WINDOW: usize = 256;
/// Analysis samples between two estimates.
const HOP: usize = 64;
/// Lowest and highest pitch the detector reports.
pub const MIN_HZ: f32 = 60.0;
/// See [`MIN_HZ`].
pub const MAX_HZ: f32 = 1_000.0;
/// Dips of the normalized difference below this count as periodic.
const THRESHOLD: f32 = 0.15;
/// Frames quieter than this (RMS) are treated as silence.
const SILENCE: f32 = 1e-3;
/// Samples processed between updates of the corrector's shift.
const CHUNK: usize = 32;

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Fractional MIDI note of a frequency, with A4 (note 69) at `reference` Hz.
#[must_use]
pub fn hz_to_note(hz: f32, reference: f32) -> f32 {
    69.0 + 12.0 * (hz.max(1e-3) / reference.max(1e-3)).log2()
}

/// The note of the scale (`mask` bit `n` = `n` semitones above `key`)
/// nearest to the fractional note `note`. An empty mask keeps `note`.
#[must_use]
pub fn nearest_in_scale(note: f32, key: u8, mask: u16) -> f32 {
    if mask & 0x0FFF == 0 || !note.is_finite() {
        return note;
    }
    let base = note.round() as i32;
    let mut best = note;
    let mut distance = f32::INFINITY;
    for candidate in base - 6..=base + 6 {
        let degree = (candidate - i32::from(key)).rem_euclid(12);
        if mask & (1 << degree) == 0 {
            continue;
        }
        let d = (candidate as f32 - note).abs();
        if d < distance {
            distance = d;
            best = candidate as f32;
        }
    }
    best
}

/// Note name and cents away from it, e.g. `("A4", 3.2)`, for a pitch.
/// Octaves are numbered so that A4 is note 69.
#[must_use]
pub fn note_and_cents(hz: f32, reference: f32) -> (String, f32) {
    let note = hz_to_note(hz, reference);
    let nearest = note.round();
    let index = (nearest as i32).rem_euclid(12) as usize;
    let octave = (nearest as i32).div_euclid(12) - 1;
    let name = musiccraft_params::NOTE_NAMES
        .get(index)
        .copied()
        .unwrap_or("?");
    (format!("{name}{octave}"), (note - nearest) * 100.0)
}

/// Monophonic pitch detector (YIN) fed one sample at a time.
#[derive(Debug, Clone)]
pub struct PitchDetector {
    decimation: usize,
    rate: f32,
    antialias: [Svf; 2],
    phase: usize,
    history: Vec<f32>,
    write: usize,
    since_hop: usize,
    frame: Vec<f32>,
    diff: Vec<f32>,
    min_tau: usize,
    max_tau: usize,
    pitch: f32,
}

impl PitchDetector {
    /// Allocates the analysis buffers for `sample_rate`. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let sample_rate = if sample_rate.is_finite() {
            sample_rate.max(1_000.0)
        } else {
            48_000.0
        };
        let decimation = ((sample_rate / ANALYSIS_RATE).round() as usize).max(1);
        let rate = sample_rate / decimation as f32;
        let min_tau = ((rate / MAX_HZ).floor() as usize).max(2);
        let max_tau = ((rate / MIN_HZ).ceil() as usize).max(min_tau + 2);
        let mut antialias = [Svf::new(), Svf::new()];
        // A fourth-order Butterworth low-pass below the analysis Nyquist.
        for (filter, q) in antialias.iter_mut().zip([0.541_f32, 1.307]) {
            filter.set_q(rate * 0.4, q, sample_rate);
        }
        let len = WINDOW + max_tau + 1;
        Self {
            decimation,
            rate,
            antialias,
            phase: 0,
            history: vec![0.0; len],
            write: 0,
            since_hop: 0,
            frame: vec![0.0; len],
            diff: vec![0.0; max_tau + 2],
            min_tau,
            max_tau,
            pitch: 0.0,
        }
    }

    /// Clears the history and the estimate.
    pub fn reset(&mut self) {
        self.history.fill(0.0);
        for filter in &mut self.antialias {
            filter.reset();
        }
        self.phase = 0;
        self.since_hop = 0;
        self.pitch = 0.0;
    }

    /// The latest estimate in hertz, or `None` for silence or noise.
    #[must_use]
    pub fn pitch(&self) -> Option<f32> {
        (self.pitch > 0.0).then_some(self.pitch)
    }

    /// Feeds one input sample; returns true when a new estimate was made.
    pub fn push(&mut self, sample: f32) -> bool {
        let sample = if sample.is_finite() { sample } else { 0.0 };
        let mut x = sample;
        for filter in &mut self.antialias {
            x = filter.process(x, SvfMode::LowPass);
        }
        self.phase += 1;
        if self.phase < self.decimation {
            return false;
        }
        self.phase = 0;
        let len = self.history.len();
        if let Some(slot) = self.history.get_mut(self.write) {
            *slot = x;
        }
        self.write = (self.write + 1) % len.max(1);
        self.since_hop += 1;
        if self.since_hop < HOP {
            return false;
        }
        self.since_hop = 0;
        self.estimate();
        true
    }

    /// Runs YIN over the newest frame.
    fn estimate(&mut self) {
        let len = self.history.len();
        // Unroll the ring, oldest sample first.
        for (i, slot) in self.frame.iter_mut().enumerate() {
            *slot = self
                .history
                .get((self.write + i) % len)
                .copied()
                .unwrap_or(0.0);
        }
        let frame = &self.frame;
        let energy = frame.iter().take(WINDOW).map(|s| s * s).sum::<f32>() / WINDOW as f32;
        if energy.sqrt() < SILENCE {
            self.pitch = 0.0;
            return;
        }
        // Difference function d(τ), then its cumulative mean normalization.
        let mut running = 0.0_f32;
        if let Some(first) = self.diff.first_mut() {
            *first = 1.0;
        }
        for tau in 1..=self.max_tau {
            let d: f32 = frame
                .iter()
                .zip(frame.iter().skip(tau))
                .take(WINDOW)
                .map(|(a, b)| (a - b) * (a - b))
                .sum();
            running += d;
            if let Some(slot) = self.diff.get_mut(tau) {
                *slot = if running > 0.0 {
                    d * tau as f32 / running
                } else {
                    1.0
                };
            }
        }
        self.pitch = self.pick().map_or(0.0, |tau| self.rate / tau);
    }

    /// The first dip below the threshold, walked down to its minimum and
    /// refined by a parabola through its neighbours.
    fn pick(&self) -> Option<f32> {
        let at = |tau: usize| self.diff.get(tau).copied().unwrap_or(1.0);
        let mut tau = self.min_tau;
        while tau < self.max_tau && at(tau) >= THRESHOLD {
            tau += 1;
        }
        if tau >= self.max_tau {
            return None;
        }
        while tau + 1 < self.max_tau && at(tau + 1) < at(tau) {
            tau += 1;
        }
        let (a, b, c) = (at(tau - 1), at(tau), at(tau + 1));
        let curve = a - 2.0 * b + c;
        let offset = if curve.abs() > 1e-9 {
            (0.5 * (a - c) / curve).clamp(-0.5, 0.5)
        } else {
            0.0
        };
        let refined = tau as f32 + offset;
        (refined > 0.0).then_some(refined)
    }
}

/// Pitch corrector: detects the pitch of a monophonic voice or instrument
/// and pulls it toward the nearest note of a scale.
///
/// Parameters: `[key, scale, speed ms, amount, reference Hz, mix]`.
#[derive(Debug, Clone)]
pub struct PitchCorrector {
    detector: PitchDetector,
    shifter: PitchShifter,
    /// Current correction in semitones.
    shift: f32,
    /// Correction the current estimate asks for, in semitones.
    target: f32,
    /// Read-head window in samples, smoothed.
    window: f32,
}

impl PitchCorrector {
    /// Allocates the detector and shifter. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            detector: PitchDetector::new(sample_rate),
            shifter: PitchShifter::new(sample_rate),
            shift: 0.0,
            target: 0.0,
            window: 0.0,
        }
    }

    /// Clears detector, delay and correction.
    pub fn reset(&mut self) {
        self.detector.reset();
        self.shifter.reset();
        self.shift = 0.0;
        self.target = 0.0;
        self.window = 0.0;
    }

    /// The pitch of the input, before correction.
    #[must_use]
    pub fn detected(&self) -> Option<f32> {
        self.detector.pitch()
    }

    /// The correction currently applied, in semitones.
    #[must_use]
    pub fn correction(&self) -> f32 {
        self.shift
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let key = param(p, 0, 0.0).round().clamp(0.0, 11.0) as u8;
        let mask = CORRECTION_SCALES
            .get(param(p, 1, 0.0).round().max(0.0) as usize)
            .map_or(0x0FFF, |(_, mask)| *mask);
        let speed_ms = param(p, 2, 20.0).max(0.0);
        let amount = param(p, 3, 1.0).clamp(0.0, 1.0);
        let reference = param(p, 4, 440.0);
        let mix = param(p, 5, 1.0).clamp(0.0, 1.0);
        let sample_rate = ctx.sample_rate.max(1.0);
        // One-pole glide per chunk; a speed of 0 snaps at once.
        let chunk_ms = CHUNK as f32 * 1_000.0 / sample_rate;
        let glide = if speed_ms <= chunk_ms {
            1.0
        } else {
            1.0 - (-chunk_ms / speed_ms).exp()
        };
        for (l, r) in left.chunks_mut(CHUNK).zip(right.chunks_mut(CHUNK)) {
            let mut updated = false;
            for (a, b) in l.iter().zip(r.iter()) {
                updated |= self.detector.push((a + b) * 0.5);
            }
            // Unvoiced stretches keep the last correction, so consonants
            // between two notes do not wobble.
            if let Some(hz) = self.detector.pitch().filter(|_| updated) {
                let note = hz_to_note(hz, reference);
                self.target = (nearest_in_scale(note, key, mask) - note) * amount;
            }
            self.shift += (self.target - self.shift) * glide;
            // Two periods of the voice make a smooth window.
            let wanted = self
                .detector
                .pitch()
                .map_or(0.03 * sample_rate, |hz| 2.0 * sample_rate / hz)
                .clamp(0.01 * sample_rate, 0.05 * sample_rate);
            if self.window <= 0.0 {
                self.window = wanted;
            }
            self.window += (wanted - self.window) * 0.1;
            let ratio = 2.0_f32.powf(self.shift / 12.0);
            // The shifted copy fades in over the first 10 cents of correction,
            // so notes already in tune pass through untouched.
            let wet = mix * (self.shift.abs() / 0.1).min(1.0);
            self.shifter.shift(l, r, ratio, self.window, wet);
        }
    }
}

/// Tuner: shows the pitch of what passes through without changing it.
///
/// Parameters: `[reference Hz]` (used by the display).
#[derive(Debug, Clone)]
pub struct Tuner {
    detector: PitchDetector,
}

impl Tuner {
    /// Allocates the detector. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            detector: PitchDetector::new(sample_rate),
        }
    }

    /// Clears the detector.
    pub fn reset(&mut self) {
        self.detector.reset();
    }

    /// The detected pitch.
    #[must_use]
    pub fn detected(&self) -> Option<f32> {
        self.detector.pitch()
    }

    /// Listens to a block; the audio is left untouched.
    pub fn process(&mut self, left: &[f32], right: &[f32]) {
        for (a, b) in left.iter().zip(right) {
            self.detector.push((a + b) * 0.5);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::TAU;

    const SR: f32 = 48_000.0;

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: SR,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    /// A tone with a few harmonics, like a voice or instrument.
    fn tone(hz: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| {
                let t = i as f32 * hz * TAU / SR;
                0.4 * t.sin() + 0.2 * (2.0 * t).sin() + 0.1 * (3.0 * t).sin()
            })
            .collect()
    }

    fn detect(signal: &[f32], sample_rate: f32) -> Option<f32> {
        let mut detector = PitchDetector::new(sample_rate);
        for s in signal {
            detector.push(*s);
        }
        detector.pitch()
    }

    fn cents(a: f32, b: f32) -> f32 {
        1_200.0 * (a / b).log2()
    }

    /// Period length estimated by counting rising zero crossings.
    fn zero_crossing_hz(x: &[f32]) -> f32 {
        let crossings: Vec<usize> = x
            .windows(2)
            .enumerate()
            .filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0)
            .map(|(i, _)| i)
            .collect();
        let (Some(first), Some(last)) = (crossings.first(), crossings.last()) else {
            return 0.0;
        };
        (crossings.len() - 1) as f32 * SR / (last - first) as f32
    }

    #[test]
    fn detects_tones_across_the_range() {
        for hz in [65.0, 110.0, 196.0, 261.63, 440.0, 523.25, 880.0] {
            let found = detect(&tone(hz, 9_600), SR).unwrap_or(0.0);
            assert!(cents(found, hz).abs() < 10.0, "{hz} Hz detected as {found}");
        }
    }

    #[test]
    fn works_at_other_sample_rates() {
        for rate in [44_100.0_f32, 96_000.0] {
            let signal: Vec<f32> = (0..rate as usize / 4)
                .map(|i| (i as f32 * 330.0 * TAU / rate).sin() * 0.5)
                .collect();
            let found = detect(&signal, rate).unwrap_or(0.0);
            assert!(cents(found, 330.0).abs() < 10.0, "{rate}: {found}");
        }
    }

    #[test]
    fn silence_and_noise_have_no_pitch() {
        assert_eq!(detect(&[0.0; 9_600], SR), None);
        let mut seed = 1_u32;
        let noise: Vec<f32> = (0..9_600)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
            })
            .collect();
        assert_eq!(detect(&noise, SR), None);
    }

    #[test]
    fn scale_snapping() {
        let c_major = CORRECTION_SCALES[1].1;
        assert_eq!(nearest_in_scale(61.4, 0, c_major), 62.0, "C#+40 → D");
        assert_eq!(nearest_in_scale(60.8, 0, c_major), 60.0, "C+80 → C");
        assert_eq!(nearest_in_scale(65.9, 0, c_major), 65.0, "F#-10 → F");
        assert_eq!(nearest_in_scale(66.1, 2, c_major), 66.0, "F# is in D major");
        assert_eq!(nearest_in_scale(60.3, 0, CORRECTION_SCALES[0].1), 60.0);
        assert_eq!(nearest_in_scale(60.3, 0, 0), 60.3, "no scale, no change");
    }

    #[test]
    fn names_notes_and_cents() {
        let (name, cents) = note_and_cents(440.0, 440.0);
        assert_eq!(name, "A4");
        assert!(cents.abs() < 0.01);
        let (name, cents) = note_and_cents(261.63 * 2.0_f32.powf(0.1 / 12.0), 440.0);
        assert_eq!(name, "C4");
        assert!((cents - 10.0).abs() < 0.5, "{cents}");
        let (name, _) = note_and_cents(432.0, 432.0);
        assert_eq!(name, "A4", "reference moves the grid");
    }

    #[test]
    fn corrects_a_flat_note_to_the_scale() {
        // A4 sung 40 cents flat; chromatic, hard tune.
        let sung = 440.0 * 2.0_f32.powf(-0.4 / 12.0);
        let mut corrector = PitchCorrector::new(SR);
        let input = tone(sung, 48_000);
        let (mut l, mut r) = (input.clone(), input);
        corrector.process(&mut l, &mut r, &[0.0, 0.0, 0.0, 1.0, 440.0, 1.0], ctx());
        let out = zero_crossing_hz(&l[24_000..]);
        assert!(cents(out, 440.0).abs() < 8.0, "corrected to {out} Hz");
        assert!((corrector.correction() - 0.4).abs() < 0.05);
        let detected = corrector.detected().unwrap_or(0.0);
        assert!(
            cents(detected, sung).abs() < 10.0,
            "reports the input pitch"
        );
    }

    #[test]
    fn amount_zero_and_in_tune_input_pass_through() {
        let input = tone(440.0, 24_000);
        let mut corrector = PitchCorrector::new(SR);
        let (mut l, mut r) = (input.clone(), input.clone());
        corrector.process(&mut l, &mut r, &[0.0, 0.0, 0.0, 1.0, 440.0, 1.0], ctx());
        let out = zero_crossing_hz(&l[12_000..]);
        assert!(cents(out, 440.0).abs() < 5.0, "in tune stays: {out}");

        let flat = tone(430.0, 24_000);
        let mut corrector = PitchCorrector::new(SR);
        let (mut l, mut r) = (flat.clone(), flat.clone());
        corrector.process(&mut l, &mut r, &[0.0, 0.0, 0.0, 0.0, 440.0, 1.0], ctx());
        assert_eq!(l, flat, "amount 0 leaves audio alone");
    }

    #[test]
    fn slow_speed_glides() {
        let sung = 440.0 * 2.0_f32.powf(-0.4 / 12.0);
        let input = tone(sung, 9_600);
        let mut corrector = PitchCorrector::new(SR);
        let (mut l, mut r) = (input.clone(), input);
        corrector.process(&mut l, &mut r, &[0.0, 0.0, 400.0, 1.0, 440.0, 1.0], ctx());
        let partial = corrector.correction();
        assert!(partial > 0.0 && partial < 0.3, "still gliding: {partial}");
    }

    #[test]
    fn tuner_reports_without_changing_audio() {
        let input = tone(196.0, 9_600);
        let mut tuner = Tuner::new(SR);
        tuner.process(&input, &input);
        let found = tuner.detected().unwrap_or(0.0);
        assert!(cents(found, 196.0).abs() < 10.0, "{found}");
        tuner.reset();
        assert_eq!(tuner.detected(), None);
    }
}
