use std::f64::consts::TAU;

/// A phase-accumulator sine oscillator.
#[derive(Debug, Clone)]
pub struct Sine {
    sample_rate: f64,
    phase: f64,
    increment: f64,
}

impl Sine {
    /// Creates a silent-frequency oscillator (0 Hz) for the given sample rate.
    #[must_use]
    pub fn new(sample_rate: f64) -> Self {
        Self {
            sample_rate,
            phase: 0.0,
            increment: 0.0,
        }
    }

    /// Sets the oscillator frequency in hertz. Phase is preserved.
    pub fn set_frequency(&mut self, hz: f64) {
        self.increment = hz / self.sample_rate;
    }

    /// Produces the next sample in the range -1.0..=1.0.
    pub fn next_sample(&mut self) -> f32 {
        let sample = (self.phase * TAU).sin() as f32;
        self.phase = (self.phase + self.increment).rem_euclid(1.0);
        sample
    }
}

/// Oscillator wave shapes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Waveform {
    /// Pure sine.
    #[default]
    Sine,
    /// Band-limited (PolyBLEP) sawtooth.
    Saw,
    /// Band-limited (PolyBLEP) square.
    Square,
    /// Triangle.
    Triangle,
    /// White noise.
    Noise,
}

impl Waveform {
    /// Maps a parameter value (0..=4, see `musiccraft_params::WAVEFORMS`) to a shape.
    #[must_use]
    pub fn from_param(value: f32) -> Self {
        match value.round() as i32 {
            1 => Self::Saw,
            2 => Self::Square,
            3 => Self::Triangle,
            4 => Self::Noise,
            _ => Self::Sine,
        }
    }
}

/// Correction term that removes the discontinuity aliasing of a naive wave.
fn poly_blep(t: f64, dt: f64) -> f64 {
    if dt <= 0.0 {
        0.0
    } else if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

/// A multi-waveform oscillator with band-limited saw and square.
#[derive(Debug, Clone)]
pub struct Oscillator {
    phase: f64,
    increment: f64,
    noise_state: u32,
}

impl Default for Oscillator {
    fn default() -> Self {
        Self::new()
    }
}

impl Oscillator {
    /// Creates an oscillator at phase zero and 0 Hz.
    #[must_use]
    pub fn new() -> Self {
        Self {
            phase: 0.0,
            increment: 0.0,
            noise_state: 0x1234_5678,
        }
    }

    /// Resets the phase to `phase` (0..1).
    pub fn reset(&mut self, phase: f64) {
        self.phase = phase.rem_euclid(1.0);
    }

    /// Sets the frequency in hertz for the given sample rate.
    pub fn set_frequency(&mut self, hz: f64, sample_rate: f64) {
        self.increment = if sample_rate > 0.0 {
            (hz / sample_rate).clamp(0.0, 0.5)
        } else {
            0.0
        };
    }

    /// Produces the next sample of `wave` in roughly -1..=1.
    pub fn next_sample(&mut self, wave: Waveform) -> f32 {
        let t = self.phase;
        let dt = self.increment;
        let value = match wave {
            Waveform::Sine => (t * TAU).sin(),
            Waveform::Saw => 2.0 * t - 1.0 - poly_blep(t, dt),
            Waveform::Square => {
                let naive = if t < 0.5 { 1.0 } else { -1.0 };
                naive + poly_blep(t, dt) - poly_blep((t + 0.5).rem_euclid(1.0), dt)
            }
            Waveform::Triangle => 1.0 - 4.0 * (t - 0.5).abs(),
            Waveform::Noise => {
                // xorshift32
                let mut x = self.noise_state;
                x ^= x << 13;
                x ^= x >> 17;
                x ^= x << 5;
                self.noise_state = x;
                f64::from(x) / f64::from(u32::MAX) * 2.0 - 1.0
            }
        };
        self.phase = (t + dt).rem_euclid(1.0);
        value as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarter_rate_sine_hits_known_points() {
        let mut osc = Sine::new(4.0);
        osc.set_frequency(1.0);
        let samples: Vec<f32> = (0..4).map(|_| osc.next_sample()).collect();
        let expected = [0.0, 1.0, 0.0, -1.0];
        for (got, want) in samples.iter().zip(expected) {
            assert!((got - want).abs() < 1e-6, "got {got}, want {want}");
        }
    }

    #[test]
    fn zero_frequency_is_silent() {
        let mut osc = Sine::new(48_000.0);
        for _ in 0..16 {
            assert!(osc.next_sample().abs() < f32::EPSILON);
        }
    }

    #[test]
    fn every_waveform_stays_bounded() {
        for wave in [
            Waveform::Sine,
            Waveform::Saw,
            Waveform::Square,
            Waveform::Triangle,
            Waveform::Noise,
        ] {
            let mut osc = Oscillator::new();
            osc.set_frequency(3_000.0, 44_100.0);
            for _ in 0..10_000 {
                let s = osc.next_sample(wave);
                assert!(s.abs() <= 1.01, "{wave:?} produced {s}");
            }
        }
    }

    #[test]
    fn saw_has_no_dc_offset() {
        let mut osc = Oscillator::new();
        osc.set_frequency(441.0, 44_100.0);
        let sum: f32 = (0..44_100).map(|_| osc.next_sample(Waveform::Saw)).sum();
        assert!((sum / 44_100.0).abs() < 0.01);
    }

    #[test]
    fn waveform_from_param_maps_indices() {
        assert_eq!(Waveform::from_param(0.0), Waveform::Sine);
        assert_eq!(Waveform::from_param(1.2), Waveform::Saw);
        assert_eq!(Waveform::from_param(4.0), Waveform::Noise);
        assert_eq!(Waveform::from_param(f32::NAN), Waveform::Sine);
    }
}
