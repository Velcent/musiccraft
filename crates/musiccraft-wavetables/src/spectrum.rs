//! Partials of a frame: magnitude relative to a full-scale sine, phase in
//! turns, where `magnitude * sin(2π (h x + phase))` is harmonic `h`.

use musiccraft_dsp::fft::{Complex, Fft};

use crate::FRAME_LEN;

/// Highest harmonic a frame holds with a phase (the Nyquist bin has none).
pub const HARMONICS: usize = FRAME_LEN / 2 - 1;

/// One harmonic of a frame.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Partial {
    /// 1 is a sine of peak 1.
    pub magnitude: f32,
    /// In turns, 0..1; 0 is a sine starting upwards from zero.
    pub phase: f32,
}

/// The transform of a frame: DC at 0, harmonics at 1..=[`HARMONICS`].
pub(crate) fn transform(frame: &[f32]) -> Vec<Complex> {
    let mut buf: Vec<Complex> = (0..FRAME_LEN)
        .map(|i| (frame.get(i).copied().unwrap_or(0.0), 0.0))
        .collect();
    Fft::new(FRAME_LEN).forward(&mut buf);
    buf
}

/// Back to samples; keeps the bins' conjugate symmetry from the positive half.
pub(crate) fn inverse(mut bins: Vec<Complex>) -> Vec<f32> {
    for h in 1..=HARMONICS {
        let (re, im) = bins.get(h).copied().unwrap_or_default();
        if let Some(mirror) = bins.get_mut(FRAME_LEN - h) {
            *mirror = (re, -im);
        }
    }
    Fft::new(FRAME_LEN).inverse(&mut bins);
    bins.iter().map(|c| c.0).collect()
}

/// The partial in bin `bin` of a transform.
pub(crate) fn partial_of(bin: Complex) -> Partial {
    let scale = 2.0 / FRAME_LEN as f32;
    let magnitude = (bin.0 * bin.0 + bin.1 * bin.1).sqrt() * scale;
    // i·X has the argument 2π·phase for a sine of that phase.
    let phase = if magnitude > 1e-9 {
        (bin.0.atan2(-bin.1) / std::f32::consts::TAU).rem_euclid(1.0)
    } else {
        0.0
    };
    Partial { magnitude, phase }
}

/// The bin holding `partial`.
pub(crate) fn bin_of(partial: Partial) -> Complex {
    let half = FRAME_LEN as f32 / 2.0 * partial.magnitude;
    let angle = partial.phase * std::f32::consts::TAU;
    (half * angle.sin(), -half * angle.cos())
}

/// Harmonics 1..=[`HARMONICS`] of `frame` (index 0 is harmonic 1).
#[must_use]
pub fn partials(frame: &[f32]) -> Vec<Partial> {
    let bins = transform(frame);
    (1..=HARMONICS)
        .map(|h| partial_of(bins.get(h).copied().unwrap_or_default()))
        .collect()
}

/// A frame from harmonics 1..=[`HARMONICS`] (index 0 is harmonic 1) and no DC.
#[must_use]
pub fn from_partials(partials: &[Partial]) -> Vec<f32> {
    let mut bins = vec![(0.0, 0.0); FRAME_LEN];
    for (h, partial) in partials.iter().enumerate().take(HARMONICS) {
        if let Some(bin) = bins.get_mut(h + 1) {
            *bin = bin_of(*partial);
        }
    }
    inverse(bins)
}
