//! DSP building blocks. Everything here runs on the audio thread: no allocation,
//! no locks, no I/O, no panics.

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
    fn output_stays_in_range() {
        let mut osc = Sine::new(44_100.0);
        osc.set_frequency(440.0);
        for _ in 0..10_000 {
            assert!(osc.next_sample().abs() <= 1.0);
        }
    }
}
