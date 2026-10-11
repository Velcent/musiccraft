//! A small iterative radix-2 FFT. Tables are built in `new`; transforms
//! never allocate, so they may run on the audio thread.

use std::f32::consts::TAU;

/// A complex number as (re, im).
pub type Complex = (f32, f32);

fn mul(a: Complex, b: Complex) -> Complex {
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
}

/// Precomputed tables for transforms of one power-of-two size.
#[derive(Debug, Clone)]
pub struct Fft {
    twiddles: Vec<Complex>,
    reversed: Vec<usize>,
}

impl Fft {
    /// Tables for size `n`, rounded up to a power of two (at least 2).
    #[must_use]
    pub fn new(n: usize) -> Self {
        let n = n.max(2).next_power_of_two();
        let bits = n.trailing_zeros();
        let twiddles = (0..n / 2)
            .map(|k| {
                let a = -TAU * k as f32 / n as f32;
                (a.cos(), a.sin())
            })
            .collect();
        let reversed = (0..n)
            .map(|i| i.reverse_bits() >> (usize::BITS - bits))
            .collect();
        Self { twiddles, reversed }
    }

    /// The transform size.
    #[must_use]
    pub fn len(&self) -> usize {
        self.reversed.len()
    }

    /// Always false: sizes are at least 2.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.reversed.is_empty()
    }

    /// Forward transform in place. `buf` must have [`Fft::len`] entries;
    /// other lengths are left untouched.
    pub fn forward(&self, buf: &mut [Complex]) {
        let n = self.len();
        if buf.len() != n {
            return;
        }
        for (i, &j) in self.reversed.iter().enumerate() {
            if i < j {
                buf.swap(i, j);
            }
        }
        let mut size = 2;
        while size <= n {
            let half = size / 2;
            let stride = n / size;
            for chunk in buf.chunks_exact_mut(size) {
                let (lo, hi) = chunk.split_at_mut(half);
                for (k, (a, b)) in lo.iter_mut().zip(hi.iter_mut()).enumerate() {
                    let w = self.twiddles.get(k * stride).copied().unwrap_or((1.0, 0.0));
                    let t = mul(*b, w);
                    *b = (a.0 - t.0, a.1 - t.1);
                    *a = (a.0 + t.0, a.1 + t.1);
                }
            }
            size *= 2;
        }
    }

    /// Inverse transform in place, scaled so `inverse(forward(x)) == x`.
    pub fn inverse(&self, buf: &mut [Complex]) {
        for c in buf.iter_mut() {
            c.1 = -c.1;
        }
        self.forward(buf);
        let scale = 1.0 / buf.len().max(1) as f32;
        for c in buf.iter_mut() {
            *c = (c.0 * scale, -c.1 * scale);
        }
    }
}

/// Magnitudes in dB of `samples` (Hann-windowed), one per bin up to Nyquist.
/// Main thread only (allocates).
#[must_use]
pub fn spectrum_db(samples: &[f32]) -> Vec<f32> {
    let fft = Fft::new(samples.len());
    let n = fft.len();
    let mut buf: Vec<Complex> = (0..n)
        .map(|i| {
            let w = 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos();
            (samples.get(i).copied().unwrap_or(0.0) * w, 0.0)
        })
        .collect();
    fft.forward(&mut buf);
    // A full-scale sine reads 0 dB (Hann window has gain 0.5).
    let scale = 4.0 / n as f32;
    buf.iter()
        .take(n / 2)
        .map(|(re, im)| crate::gain_to_db((re * re + im * im).sqrt() * scale))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_a_known_bin() {
        let fft = Fft::new(64);
        let input: Vec<Complex> = (0..64).map(|i| ((i as f32 * 0.3).sin(), 0.0)).collect();
        let mut buf = input.clone();
        fft.forward(&mut buf);
        fft.inverse(&mut buf);
        for (a, b) in buf.iter().zip(&input) {
            assert!((a.0 - b.0).abs() < 1e-4 && a.1.abs() < 1e-4);
        }
        let tone: Vec<f32> = (0..1024)
            .map(|i| (TAU * 64.0 * i as f32 / 1024.0).sin())
            .collect();
        let db = spectrum_db(&tone);
        let peak = db
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i);
        assert_eq!(peak, Some(64));
        assert!(db[64].abs() < 0.5, "full scale reads 0 dB: {}", db[64]);
    }
}
