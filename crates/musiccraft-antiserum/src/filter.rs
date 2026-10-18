//! Antiserum's multimode filter (one channel). Coefficients are set at the
//! control rate; processing is per sample and allocation-free.

use std::f32::consts::PI;

use musiccraft_dsp::flush_denormal;

/// Delay line length for the comb and flanger types (down to 20 Hz at
/// 96 kHz; higher rates bottom out a little above).
const DELAY_LEN: usize = 8_192;

/// A filter type, by `filterN_type` value (see `params::FILTER_TYPES`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterType {
    LowPass12,
    #[default]
    LowPass24,
    HighPass12,
    HighPass24,
    BandPass12,
    BandPass24,
    Notch12,
    Notch24,
    Peak12,
    Peak24,
    Ladder,
    Diode,
    CombPlus,
    CombMinus,
    Formant,
    Vowel,
    Phaser,
    Flanger,
}

const ALL: [FilterType; 18] = [
    FilterType::LowPass12,
    FilterType::LowPass24,
    FilterType::HighPass12,
    FilterType::HighPass24,
    FilterType::BandPass12,
    FilterType::BandPass24,
    FilterType::Notch12,
    FilterType::Notch24,
    FilterType::Peak12,
    FilterType::Peak24,
    FilterType::Ladder,
    FilterType::Diode,
    FilterType::CombPlus,
    FilterType::CombMinus,
    FilterType::Formant,
    FilterType::Vowel,
    FilterType::Phaser,
    FilterType::Flanger,
];

impl FilterType {
    /// The type for a parameter value.
    #[must_use]
    pub fn from_param(value: f32) -> Self {
        ALL.get(value.max(0.0).round() as usize)
            .copied()
            .unwrap_or_default()
    }

    fn uses_delay(self) -> bool {
        matches!(self, Self::CombPlus | Self::CombMinus | Self::Flanger)
    }
}

/// Vowel formants (Hz) for a, e, i, o, u: textbook averages.
const VOWELS: [[f32; 3]; 5] = [
    [800.0, 1_150.0, 2_900.0],
    [400.0, 1_600.0, 2_700.0],
    [300.0, 2_300.0, 3_000.0],
    [450.0, 800.0, 2_830.0],
    [325.0, 700.0, 2_530.0],
];

/// A TPT state-variable filter section returning (low, band, high).
#[derive(Debug, Clone, Copy, Default)]
struct Svf {
    ic1: f32,
    ic2: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    fn set(&mut self, cutoff: f32, q: f32, sample_rate: f32) {
        let cutoff = cutoff.clamp(10.0, sample_rate * 0.49);
        let g = (PI * cutoff / sample_rate).tan();
        self.k = 1.0 / q.max(0.05);
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    fn process(&mut self, x: f32) -> (f32, f32, f32) {
        let v3 = x - self.ic2;
        let v1 = self.a1 * self.ic1 + self.a2 * v3;
        let v2 = self.ic2 + self.a2 * self.ic1 + self.a3 * v3;
        self.ic1 = flush_denormal(2.0 * v1 - self.ic1);
        self.ic2 = flush_denormal(2.0 * v2 - self.ic2);
        (v2, v1, x - self.k * v1 - v2)
    }
}

/// One channel of a filter.
#[derive(Debug, Clone)]
pub struct Filter {
    kind: FilterType,
    sample_rate: f32,
    svf: [Svf; 3],
    /// Cutoff coefficient for the one-pole cascades and all-passes.
    g: f32,
    resonance: f32,
    q: f32,
    drive: f32,
    gain: [f32; 3],
    poles: [f32; 4],
    feedback: f32,
    delay: Box<[f32; DELAY_LEN]>,
    write: usize,
    delay_samples: f32,
}

impl Filter {
    /// A cleared low-pass filter. Allocates its delay line.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            kind: FilterType::default(),
            sample_rate: sample_rate.max(1.0),
            svf: [Svf::default(); 3],
            g: 0.0,
            resonance: 0.0,
            q: 0.5,
            drive: 1.0,
            gain: [1.0; 3],
            poles: [0.0; 4],
            feedback: 0.0,
            delay: Box::new([0.0; DELAY_LEN]),
            write: 0,
            delay_samples: 1.0,
        }
    }

    /// Clears the filter's memory.
    pub fn reset(&mut self) {
        for svf in &mut self.svf {
            svf.ic1 = 0.0;
            svf.ic2 = 0.0;
        }
        self.poles = [0.0; 4];
        self.feedback = 0.0;
        if self.kind.uses_delay() {
            self.delay.fill(0.0);
        }
    }

    /// Sets the type, cutoff (Hz), resonance and drive (0..1). Control rate.
    pub fn set(&mut self, kind: FilterType, cutoff: f32, resonance: f32, drive: f32) {
        if kind != self.kind {
            self.kind = kind;
            self.reset();
        }
        let sr = self.sample_rate;
        let cutoff = cutoff.clamp(20.0, sr * 0.45);
        let res = resonance.clamp(0.0, 1.0);
        self.resonance = res;
        self.drive = 1.0 + drive.clamp(0.0, 1.0) * 9.0;
        self.q = 0.5 + res * res * 19.5;
        self.g = (PI * cutoff / sr).tan();
        match kind {
            FilterType::LowPass12
            | FilterType::HighPass12
            | FilterType::BandPass12
            | FilterType::Notch12
            | FilterType::Peak12 => self.svf[0].set(cutoff, self.q, sr),
            FilterType::LowPass24
            | FilterType::HighPass24
            | FilterType::BandPass24
            | FilterType::Notch24
            | FilterType::Peak24 => {
                // Two sections: resonance on the second only.
                self.svf[0].set(cutoff, std::f32::consts::FRAC_1_SQRT_2, sr);
                self.svf[1].set(cutoff, self.q, sr);
            }
            FilterType::Formant => {
                self.svf[0].set(cutoff, 2.0 + res * 18.0, sr);
                self.svf[1].set(cutoff * 2.7, 2.0 + res * 18.0, sr);
                self.gain = [1.0, 0.6, 0.0];
            }
            FilterType::Vowel => {
                // Cutoff position 20 Hz..20 kHz (log) morphs a–e–i–o–u.
                let t = ((cutoff / 20.0).ln() / 1_000.0_f32.ln()).clamp(0.0, 1.0) * 4.0;
                let i = (t.floor() as usize).min(3);
                let mix = t - i as f32;
                let (lo, hi) = (VOWELS[i], VOWELS[i + 1]);
                for (n, svf) in self.svf.iter_mut().enumerate() {
                    let freq = lo[n] + (hi[n] - lo[n]) * mix;
                    svf.set(freq, 4.0 + res * 16.0, sr);
                }
                self.gain = [1.0, 0.7, 0.4];
            }
            FilterType::CombPlus | FilterType::CombMinus | FilterType::Flanger => {
                self.delay_samples = (sr / cutoff).clamp(1.0, (DELAY_LEN - 2) as f32);
            }
            FilterType::Ladder | FilterType::Diode | FilterType::Phaser => {}
        }
    }

    /// Filters one sample.
    pub fn process(&mut self, x: f32) -> f32 {
        let x = if self.drive > 1.0 {
            (x * self.drive).tanh() / self.drive.sqrt()
        } else {
            x
        };
        let y = match self.kind {
            FilterType::LowPass12 => self.svf[0].process(x).0,
            FilterType::HighPass12 => self.svf[0].process(x).2,
            FilterType::BandPass12 => self.svf[0].process(x).1,
            FilterType::Notch12 => {
                let (lo, _, hi) = self.svf[0].process(x);
                lo + hi
            }
            FilterType::Peak12 => {
                let (lo, _, hi) = self.svf[0].process(x);
                lo - hi
            }
            FilterType::LowPass24 => {
                let a = self.svf[0].process(x).0;
                self.svf[1].process(a).0
            }
            FilterType::HighPass24 => {
                let a = self.svf[0].process(x).2;
                self.svf[1].process(a).2
            }
            FilterType::BandPass24 => {
                let a = self.svf[0].process(x).1;
                self.svf[1].process(a).1
            }
            FilterType::Notch24 => {
                let (lo, _, hi) = self.svf[0].process(x);
                let (lo2, _, hi2) = self.svf[1].process(lo + hi);
                lo2 + hi2
            }
            FilterType::Peak24 => {
                let (lo, _, hi) = self.svf[0].process(x);
                let (lo2, _, hi2) = self.svf[1].process(lo - hi);
                lo2 - hi2
            }
            FilterType::Ladder => self.ladder(x, 4, 4.0),
            FilterType::Diode => self.ladder(x, 3, 3.2),
            FilterType::CombPlus => self.comb(x, 1.0, false),
            FilterType::CombMinus => self.comb(x, -1.0, false),
            FilterType::Flanger => self.comb(x, 1.0, true),
            FilterType::Formant | FilterType::Vowel => {
                let mut sum = 0.0;
                for (svf, gain) in self.svf.iter_mut().zip(self.gain) {
                    if gain > 0.0 {
                        sum += svf.process(x).1 * gain;
                    }
                }
                sum * 1.5
            }
            FilterType::Phaser => self.phaser(x),
        };
        if y.is_finite() {
            y
        } else {
            self.reset();
            0.0
        }
    }

    /// A cascade of `poles` nonlinear one-pole low-passes with resonant
    /// feedback (`max_k` at full resonance). The diode type saturates
    /// asymmetrically.
    fn ladder(&mut self, x: f32, poles: usize, max_k: f32) -> f32 {
        let k = self.resonance * max_k;
        let g = self.g / (1.0 + self.g);
        let diode = poles == 3;
        // Feedback from the last pole of the previous sample.
        let last = self.poles.get(poles - 1).copied().unwrap_or(0.0);
        let mut input = x * (1.0 + k * 0.5) - k * last;
        input = if !diode {
            input.tanh()
        } else if input > 0.0 {
            input / (1.0 + input)
        } else {
            input / (1.0 - 0.6 * input)
        };
        for pole in self.poles.iter_mut().take(poles) {
            let v = (input - *pole) * g;
            let y = v + *pole;
            *pole = flush_denormal(y + v);
            input = y;
        }
        input
    }

    /// Feedback comb (or, with `flanger`, input plus the comb) tuned to the
    /// cutoff; `sign` picks comb+ or comb−.
    fn comb(&mut self, x: f32, sign: f32, flanger: bool) -> f32 {
        let read = self.write as f32 + DELAY_LEN as f32 - self.delay_samples;
        let i = read as usize % DELAY_LEN;
        let t = read - read.floor();
        let a = self.delay.get(i).copied().unwrap_or(0.0);
        let b = self.delay.get((i + 1) % DELAY_LEN).copied().unwrap_or(0.0);
        let delayed = a + (b - a) * t;
        let fb = sign * self.resonance * 0.97;
        let y = x + fb * delayed;
        if let Some(slot) = self.delay.get_mut(self.write) {
            *slot = flush_denormal(y);
        }
        self.write = (self.write + 1) % DELAY_LEN;
        // Unity average power whatever the feedback.
        let norm = (1.0 - fb * fb).sqrt();
        if flanger {
            0.5 * (x + delayed * norm)
        } else {
            y * norm
        }
    }

    /// Four first-order all-passes at the cutoff with feedback, mixed with
    /// the input (notches).
    fn phaser(&mut self, x: f32) -> f32 {
        let a = (self.g - 1.0) / (self.g + 1.0);
        let mut y = x + self.feedback * self.resonance * 0.9;
        for state in &mut self.poles {
            let out = a * y + *state;
            *state = flush_denormal(y - a * out);
            y = out;
        }
        self.feedback = y;
        0.5 * (x + y)
    }
}

/// Phase-free magnitude helper for tests: RMS of a filtered sine.
#[cfg(test)]
fn response(kind: FilterType, cutoff: f32, freq: f32) -> f32 {
    let sr = 48_000.0;
    let mut filter = Filter::new(sr);
    filter.set(kind, cutoff, 0.2, 0.0);
    let mut sum = 0.0;
    let n = 9_600;
    for i in 0..n {
        let y = filter.process((std::f32::consts::TAU * freq * i as f32 / sr).sin());
        if i >= n / 2 {
            sum += y * y;
        }
    }
    (sum / (n / 2) as f32 * 2.0).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_types_pass_and_stop_the_right_bands() {
        use FilterType::*;
        for kind in [LowPass12, LowPass24, Ladder, Diode] {
            assert!(response(kind, 1_000.0, 100.0) > 0.5, "{kind:?} passes lows");
            assert!(
                response(kind, 1_000.0, 8_000.0) < 0.1,
                "{kind:?} stops highs"
            );
        }
        for kind in [HighPass12, HighPass24] {
            assert!(response(kind, 1_000.0, 8_000.0) > 0.7, "{kind:?}");
            assert!(response(kind, 1_000.0, 100.0) < 0.1, "{kind:?}");
        }
        for kind in [BandPass12, BandPass24] {
            assert!(response(kind, 1_000.0, 1_000.0) > 2.0 * response(kind, 1_000.0, 100.0));
        }
        for kind in [Notch12, Notch24] {
            assert!(response(kind, 1_000.0, 1_000.0) < 0.2, "{kind:?}");
            assert!(response(kind, 1_000.0, 100.0) > 0.8, "{kind:?}");
        }
    }

    #[test]
    fn every_type_is_stable_and_bounded() {
        for kind in ALL {
            let mut filter = Filter::new(48_000.0);
            filter.set(kind, 300.0, 1.0, 1.0);
            let mut peak = 0.0_f32;
            for i in 0..48_000 {
                let x = if i % 200 < 100 { 1.0 } else { -1.0 };
                let y = filter.process(x);
                assert!(y.is_finite(), "{kind:?}");
                peak = peak.max(y.abs());
            }
            assert!(peak < 50.0, "{kind:?} peaks at {peak}");
        }
    }

    #[test]
    fn combs_and_phaser_change_the_sound() {
        use FilterType::*;
        for kind in [CombPlus, CombMinus, Flanger, Phaser, Formant, Vowel] {
            let a = response(kind, 500.0, 500.0);
            let b = response(kind, 500.0, 750.0);
            assert!((a - b).abs() > 0.05, "{kind:?}: {a} vs {b}");
        }
        assert_eq!(FilterType::from_param(17.0), Flanger);
    }
}
