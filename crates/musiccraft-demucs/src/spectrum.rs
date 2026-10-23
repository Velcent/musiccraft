//! The short-time transforms on both sides of the network, exactly as
//! HTDemucs computes them (its `_spec` and `_ispec`, on PyTorch's `stft` and
//! `istft` with `normalized=True`, `center=True` and reflection padding).

use std::f32::consts::PI;

use musiccraft_dsp::fft::{Complex, Fft};

/// Transforms of one size and hop.
pub(crate) struct Transform {
    fft: Fft,
    window: Vec<f32>,
    hop: usize,
}

/// `x` padded by mirroring at both ends, the end samples not repeated
/// (`left` and `right` must be shorter than `x`).
fn reflect(x: &[f32], left: usize, right: usize) -> Vec<f32> {
    let n = x.len();
    let at = |i: isize| -> f32 {
        let j = if i < 0 {
            (-i) as usize
        } else if i as usize >= n {
            2 * (n - 1) - i as usize
        } else {
            i as usize
        };
        x.get(j).copied().unwrap_or(0.0)
    };
    (-(left as isize)..(n + right) as isize).map(at).collect()
}

impl Transform {
    /// Transforms of `n` bins with a hop of `n / 4` (HTDemucs's only choice).
    pub(crate) fn new(n: usize) -> Self {
        let window = (0..n)
            .map(|i| 0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos())
            .collect();
        Self {
            fft: Fft::new(n),
            window,
            hop: n / 4,
        }
    }

    fn len(&self) -> usize {
        self.window.len()
    }

    /// Frames of a segment of `length` samples.
    pub(crate) fn frames(&self, length: usize) -> usize {
        length.div_ceil(self.hop)
    }

    /// Bins kept per frame (Nyquist is dropped).
    pub(crate) fn bins(&self) -> usize {
        self.len() / 2
    }

    /// The spectrogram of one channel, `bins × frames`, bin-major.
    pub(crate) fn forward(&self, x: &[f32]) -> Vec<Complex> {
        let (n, hop) = (self.len(), self.hop);
        let frames = self.frames(x.len());
        let pad = hop / 2 * 3;
        let padded = reflect(x, pad, pad + frames * hop - x.len());
        // PyTorch's own centring, then frames 2.. of it.
        let padded = reflect(&padded, n / 2, n / 2);
        let scale = 1.0 / (n as f32).sqrt();
        let bins = self.bins();
        let mut out = vec![(0.0, 0.0); bins * frames];
        let mut buf = vec![(0.0, 0.0); n];
        for frame in 0..frames {
            let start = (frame + 2) * hop;
            for (i, (b, w)) in buf.iter_mut().zip(&self.window).enumerate() {
                *b = (padded.get(start + i).copied().unwrap_or(0.0) * w, 0.0);
            }
            self.fft.forward(&mut buf);
            for (bin, c) in buf.iter().take(bins).enumerate() {
                out[bin * frames + frame] = (c.0 * scale, c.1 * scale);
            }
        }
        out
    }

    /// One channel back from a `bins × frames` spectrogram, `length` samples.
    pub(crate) fn inverse(&self, z: &[Complex], frames: usize, length: usize) -> Vec<f32> {
        let (n, hop) = (self.len(), self.hop);
        let bins = self.bins();
        let pad = hop / 2 * 3;
        // Two silent frames each side, as `_ispec` adds.
        let total = frames + 4;
        let span = n + hop * (total - 1);
        let mut sum = vec![0.0_f32; span];
        let mut envelope = vec![0.0_f32; span];
        let scale = (n as f32).sqrt();
        let mut buf = vec![(0.0, 0.0); n];
        for k in 0..total {
            let frame = k.checked_sub(2).filter(|f| *f < frames);
            buf.iter_mut().for_each(|b| *b = (0.0, 0.0));
            if let Some(frame) = frame {
                for bin in 0..bins {
                    let mut c = z[bin * frames + frame];
                    if bin == 0 {
                        // A real transform ignores the imaginary part here.
                        c.1 = 0.0;
                    }
                    buf[bin] = c;
                    if bin > 0 {
                        buf[n - bin] = (c.0, -c.1);
                    }
                }
                self.fft.inverse(&mut buf);
            }
            let start = k * hop;
            for (i, w) in self.window.iter().enumerate() {
                sum[start + i] += buf[i].0 * scale * w;
                envelope[start + i] += w * w;
            }
        }
        (0..length)
            .map(|i| {
                let j = i + pad + n / 2;
                let e = envelope.get(j).copied().unwrap_or(0.0);
                if e > 1e-11 { sum[j] / e } else { 0.0 }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reflection_skips_the_edge_sample() {
        assert_eq!(
            reflect(&[1.0, 2.0, 3.0, 4.0], 2, 2),
            [3.0, 2.0, 1.0, 2.0, 3.0, 4.0, 3.0, 2.0]
        );
    }

    #[test]
    fn the_inverse_undoes_the_transform() {
        let t = Transform::new(256);
        let x: Vec<f32> = (0..3_000)
            .map(|i| (i as f32 * 0.05).sin() + 0.3 * (i as f32 * 0.31).cos())
            .collect();
        let z = t.forward(&x);
        let frames = t.frames(x.len());
        assert_eq!(z.len(), t.bins() * frames);
        let y = t.inverse(&z, frames, x.len());
        // HTDemucs drops the first and last two frames, so only the
        // middle comes back exactly.
        for (a, b) in x.iter().zip(&y).take(x.len() - 256).skip(256) {
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }
}
