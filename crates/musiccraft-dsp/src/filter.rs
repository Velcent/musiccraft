use std::f32::consts::PI;

/// Response selected from a [`Svf`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SvfMode {
    /// Low-pass.
    #[default]
    LowPass,
    /// Band-pass.
    BandPass,
    /// High-pass.
    HighPass,
}

impl SvfMode {
    /// Maps a `mode` parameter value (0, 1, 2) to a response.
    #[must_use]
    pub fn from_param(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::BandPass,
            2 => Self::HighPass,
            _ => Self::LowPass,
        }
    }
}

/// Topology-preserving-transform state-variable filter (after Andrew Simper's
/// published "linear trapezoidal integrated SVF" paper).
#[derive(Debug, Clone, Default)]
pub struct Svf {
    ic1eq: f32,
    ic2eq: f32,
    a1: f32,
    a2: f32,
    a3: f32,
    k: f32,
}

impl Svf {
    /// Creates a filter with neutral coefficients; call [`Svf::set`] before use.
    #[must_use]
    pub fn new() -> Self {
        let mut svf = Self::default();
        svf.set(1_000.0, 0.0, 44_100.0);
        svf
    }

    /// Sets cutoff (hertz) and resonance (0..=1) for a sample rate.
    pub fn set(&mut self, cutoff: f32, resonance: f32, sample_rate: f32) {
        let nyquist = sample_rate * 0.49;
        let cutoff = cutoff.clamp(10.0, nyquist.max(10.0));
        let g = (PI * cutoff / sample_rate.max(1.0)).tan();
        // Resonance 0 → Q 0.5, resonance 1 → Q ~ 25.
        let q = 0.5 + resonance.clamp(0.0, 1.0).powi(2) * 24.5;
        self.k = 1.0 / q;
        self.a1 = 1.0 / (1.0 + g * (g + self.k));
        self.a2 = g * self.a1;
        self.a3 = g * self.a2;
    }

    /// Clears the filter memory.
    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    /// Filters one sample.
    pub fn process(&mut self, input: f32, mode: SvfMode) -> f32 {
        let v3 = input - self.ic2eq;
        let v1 = self.a1 * self.ic1eq + self.a2 * v3;
        let v2 = self.ic2eq + self.a2 * self.ic1eq + self.a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;
        if !self.ic1eq.is_finite() || !self.ic2eq.is_finite() {
            self.reset();
            return 0.0;
        }
        match mode {
            SvfMode::LowPass => v2,
            SvfMode::BandPass => v1,
            SvfMode::HighPass => input - self.k * v1 - v2,
        }
    }
}

/// A transposed direct-form II biquad, with coefficient formulas from the public
/// "Audio EQ Cookbook" by Robert Bristow-Johnson.
#[derive(Debug, Clone)]
pub struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Default for Biquad {
    fn default() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }
}

impl Biquad {
    fn set_normalized(&mut self, b: [f32; 3], a: [f32; 3]) {
        let a0 = if a[0].abs() < 1e-12 { 1.0 } else { a[0] };
        self.b0 = b[0] / a0;
        self.b1 = b[1] / a0;
        self.b2 = b[2] / a0;
        self.a1 = a[1] / a0;
        self.a2 = a[2] / a0;
    }

    fn omega(freq: f32, sample_rate: f32) -> (f32, f32) {
        let w = 2.0 * PI * freq.clamp(10.0, sample_rate * 0.49) / sample_rate.max(1.0);
        (w.cos(), w.sin())
    }

    /// Configures a low-shelf with slope 1.
    pub fn set_low_shelf(&mut self, freq: f32, gain_db: f32, sample_rate: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, sample_rate);
        let alpha = sin / 2.0 * 2.0_f32.sqrt();
        let sq = 2.0 * a.sqrt() * alpha;
        self.set_normalized(
            [
                a * ((a + 1.0) - (a - 1.0) * cos + sq),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cos),
                a * ((a + 1.0) - (a - 1.0) * cos - sq),
            ],
            [
                (a + 1.0) + (a - 1.0) * cos + sq,
                -2.0 * ((a - 1.0) + (a + 1.0) * cos),
                (a + 1.0) + (a - 1.0) * cos - sq,
            ],
        );
    }

    /// Configures a high-shelf with slope 1.
    pub fn set_high_shelf(&mut self, freq: f32, gain_db: f32, sample_rate: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, sample_rate);
        let alpha = sin / 2.0 * 2.0_f32.sqrt();
        let sq = 2.0 * a.sqrt() * alpha;
        self.set_normalized(
            [
                a * ((a + 1.0) + (a - 1.0) * cos + sq),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cos),
                a * ((a + 1.0) + (a - 1.0) * cos - sq),
            ],
            [
                (a + 1.0) - (a - 1.0) * cos + sq,
                2.0 * ((a - 1.0) - (a + 1.0) * cos),
                (a + 1.0) - (a - 1.0) * cos - sq,
            ],
        );
    }

    /// Configures a peaking band.
    pub fn set_peak(&mut self, freq: f32, gain_db: f32, q: f32, sample_rate: f32) {
        let a = 10.0_f32.powf(gain_db / 40.0);
        let (cos, sin) = Self::omega(freq, sample_rate);
        let alpha = sin / (2.0 * q.max(0.01));
        self.set_normalized(
            [1.0 + alpha * a, -2.0 * cos, 1.0 - alpha * a],
            [1.0 + alpha / a, -2.0 * cos, 1.0 - alpha / a],
        );
    }

    /// Clears the filter memory.
    pub fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }

    /// Filters one sample.
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        if !y.is_finite() {
            self.reset();
            return 0.0;
        }
        y
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Oscillator, Waveform};

    fn rms_through(mut f: impl FnMut(f32) -> f32, hz: f64) -> f32 {
        let mut osc = Oscillator::new();
        osc.set_frequency(hz, 44_100.0);
        let mut sum = 0.0;
        for i in 0..44_100 {
            let y = f(osc.next_sample(Waveform::Sine));
            if i > 4_410 {
                sum += y * y;
            }
        }
        (sum / (44_100.0 - 4_410.0)).sqrt()
    }

    #[test]
    fn lowpass_passes_lows_and_cuts_highs() {
        let mut svf = Svf::new();
        svf.set(500.0, 0.0, 44_100.0);
        let low = rms_through(|x| svf.process(x, SvfMode::LowPass), 100.0);
        svf.reset();
        let high = rms_through(|x| svf.process(x, SvfMode::LowPass), 8_000.0);
        assert!(low > 0.6, "low rms {low}");
        assert!(high < 0.05, "high rms {high}");
    }

    #[test]
    fn highpass_cuts_lows() {
        let mut svf = Svf::new();
        svf.set(2_000.0, 0.0, 44_100.0);
        let low = rms_through(|x| svf.process(x, SvfMode::HighPass), 50.0);
        assert!(low < 0.01, "low rms {low}");
    }

    #[test]
    fn peak_boost_raises_level_at_centre() {
        let mut bq = Biquad::default();
        bq.set_peak(1_000.0, 12.0, 1.0, 44_100.0);
        let rms = rms_through(|x| bq.process(x), 1_000.0);
        // +12 dB on a sine of rms 0.707 → ~2.81
        assert!((rms - 2.81).abs() < 0.1, "rms {rms}");
    }

    #[test]
    fn flat_shelves_are_transparent() {
        let mut low = Biquad::default();
        low.set_low_shelf(200.0, 0.0, 44_100.0);
        let mut high = Biquad::default();
        high.set_high_shelf(5_000.0, 0.0, 44_100.0);
        let rms = rms_through(|x| high.process(low.process(x)), 1_000.0);
        assert!(
            (rms - std::f32::consts::FRAC_1_SQRT_2).abs() < 0.01,
            "rms {rms}"
        );
    }
}
