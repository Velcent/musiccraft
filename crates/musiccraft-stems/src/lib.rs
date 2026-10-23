//! Splits a stereo sound into four stems: vocals, drums, bass and other
//! (`docs/spec/stems.md`). Works on whole sounds and allocates, so it runs on
//! the main thread or a worker thread, never the audio thread.
//!
//! The built-in splitter needs no model: it shares every bin of a
//! short-time spectrum between the stems, by harmonic–percussive median
//! filtering, frequency and how centred the bin is in the stereo image.

use std::collections::VecDeque;
use std::f32::consts::PI;

use musiccraft_dsp::fft::{Complex, Fft};

/// One of the four stems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stem {
    /// Centred voices (and any centred lead).
    Vocals,
    /// Hits and clicks.
    Drums,
    /// Sustained low notes.
    Bass,
    /// Everything else.
    Other,
}

impl Stem {
    /// Every stem, in file and channel order.
    pub const ALL: [Self; 4] = [Self::Vocals, Self::Drums, Self::Bass, Self::Other];

    /// Lower-case name, used for files and channel names.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Vocals => "vocals",
            Self::Drums => "drums",
            Self::Bass => "bass",
            Self::Other => "other",
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Vocals => 0,
            Self::Drums => 1,
            Self::Bass => 2,
            Self::Other => 3,
        }
    }
}

/// A stereo sound.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Audio {
    /// Left channel.
    pub left: Vec<f32>,
    /// Right channel.
    pub right: Vec<f32>,
}

impl Audio {
    fn silent(len: usize) -> Self {
        Self {
            left: vec![0.0; len],
            right: vec![0.0; len],
        }
    }
}

/// The four stems of a sound, each as long as the sound.
#[derive(Debug, Clone, PartialEq)]
pub struct Stems([Audio; 4]);

impl Stems {
    /// Stems given in [`Stem::ALL`] order.
    #[must_use]
    pub fn new(stems: [Audio; 4]) -> Self {
        Self(stems)
    }

    /// One stem.
    #[must_use]
    pub fn get(&self, stem: Stem) -> &Audio {
        &self.0[stem.index()]
    }
}

/// The split was stopped by its progress callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

/// Half the width of both median filters, in frames and in bins.
const MEDIAN: usize = 8;
/// The centre weight's power: only near-centre bins count as vocals.
const CENTRE_POWER: i32 = 6;
/// Below this, a magnitude counts as silence.
const TINY: f32 = 1e-12;

/// Window length for `sample_rate`: about 85 ms, as a power of two.
fn window_len(sample_rate: f64) -> usize {
    ((sample_rate.max(1.0) * 0.085) as usize)
        .next_power_of_two()
        .clamp(256, 16_384)
}

/// 0 below `a`, 1 above `b`, smooth in between.
fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The median of `values` (reordered).
fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    *values.select_nth_unstable_by(mid, f32::total_cmp).1
}

/// The left and right spectra of bin `k` of `z`, the transform of
/// `left + i·right`.
fn sides(z: &[Complex], k: usize) -> (Complex, Complex) {
    let n = z.len();
    let a = z[k];
    let b = z[(n - k) % n];
    let conj = (b.0, -b.1);
    let left = ((a.0 + conj.0) * 0.5, (a.1 + conj.1) * 0.5);
    // (a - conj) / 2i
    let right = ((a.1 - conj.1) * 0.5, -(a.0 - conj.0) * 0.5);
    (left, right)
}

fn norm(c: Complex) -> f32 {
    c.0 * c.0 + c.1 * c.1
}

/// One analysed frame: the transform of both channels packed as
/// `left + i·right`, and the bins' mean magnitudes up to Nyquist.
struct Frame {
    z: Vec<Complex>,
    magnitude: Vec<f32>,
}

/// The short-time transform of a stereo sound, overlap-added back.
struct Stft<'a> {
    left: &'a [f32],
    right: &'a [f32],
    fft: Fft,
    window: Vec<f32>,
    hop: usize,
}

impl Stft<'_> {
    fn len(&self) -> usize {
        self.window.len()
    }

    /// Where frame `k` starts: frames begin before the sound, so every
    /// sample is covered by the same number of frames.
    fn start(&self, k: usize) -> isize {
        (k * self.hop) as isize - (self.len() - self.hop) as isize
    }

    fn frames(&self) -> usize {
        let len = self.left.len().min(self.right.len());
        (len + self.len() - self.hop).div_ceil(self.hop)
    }

    fn analyse(&self, k: usize) -> Frame {
        let start = self.start(k);
        let at = |side: &[f32], i: usize| {
            usize::try_from(start + i as isize)
                .ok()
                .and_then(|j| side.get(j))
                .copied()
                .unwrap_or(0.0)
        };
        let mut z: Vec<Complex> = self
            .window
            .iter()
            .enumerate()
            .map(|(i, w)| (at(self.left, i) * w, at(self.right, i) * w))
            .collect();
        self.fft.forward(&mut z);
        let magnitude = (0..=self.len() / 2)
            .map(|k| {
                let (l, r) = sides(&z, k);
                (norm(l).sqrt() + norm(r).sqrt()) * 0.5
            })
            .collect();
        Frame { z, magnitude }
    }

    /// Transforms `spectrum` back and adds frame `k` of it into `out`.
    fn add(&self, k: usize, spectrum: &mut [Complex], out: &mut Audio) {
        self.fft.inverse(spectrum);
        let start = self.start(k);
        // Root-Hann windows on both sides add up to 2 at quarter hops.
        for (i, (c, w)) in spectrum.iter().zip(&self.window).enumerate() {
            let Ok(j) = usize::try_from(start + i as isize) else {
                continue;
            };
            if let (Some(l), Some(r)) = (out.left.get_mut(j), out.right.get_mut(j)) {
                *l += c.0 * w * 0.5;
                *r += c.1 * w * 0.5;
            }
        }
    }
}

/// The shares of vocals, drums and bass for one bin (other takes the rest).
fn shares(
    harmonic: f32,
    percussive: f32,
    frequency: f32,
    left: Complex,
    right: Complex,
) -> [f32; 3] {
    let (h2, p2) = (harmonic * harmonic, percussive * percussive);
    let drums = if h2 + p2 > TINY { p2 / (h2 + p2) } else { 0.0 };
    let tonal = 1.0 - drums;
    let low = 1.0 - smoothstep(120.0, 250.0, frequency);
    let band =
        smoothstep(150.0, 300.0, frequency) * (1.0 - smoothstep(7_000.0, 12_000.0, frequency));
    let energy = norm(left) + norm(right);
    let centre = if energy > TINY {
        (2.0 * (left.0 * right.0 + left.1 * right.1) / energy).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let vocals = tonal * (1.0 - low) * band * centre.powi(CENTRE_POWER);
    [vocals, drums, tonal * low]
}

/// Splits a stereo sound (pass the same slice twice for mono) into stems.
/// `progress` hears the fraction done now and then; returning `false` stops
/// the split.
///
/// # Errors
///
/// [`Cancelled`] when `progress` stopped it.
pub fn split(
    left: &[f32],
    right: &[f32],
    sample_rate: f64,
    progress: &mut dyn FnMut(f32) -> bool,
) -> Result<Stems, Cancelled> {
    let len = left.len().min(right.len());
    let n = window_len(sample_rate);
    let window: Vec<f32> = (0..n)
        .map(|i| (0.5 - 0.5 * (2.0 * PI * i as f32 / n as f32).cos()).sqrt())
        .collect();
    let stft = Stft {
        left: &left[..len],
        right: &right[..len],
        fft: Fft::new(n),
        window,
        hop: n / 4,
    };
    let mut out: [Audio; 4] = std::array::from_fn(|_| Audio::silent(len));
    let frames = stft.frames();
    let bins = n / 2 + 1;
    let bin_hz = (sample_rate / n as f64) as f32;
    // Frames `first..first + window.len()` are analysed and kept.
    let mut kept: VecDeque<Frame> = VecDeque::new();
    let mut first = 0;
    let mut scratch = Vec::with_capacity(2 * MEDIAN + 1);
    let mut spectra: [Vec<Complex>; 3] = std::array::from_fn(|_| vec![(0.0, 0.0); n]);
    for k in 0..frames {
        while first + kept.len() < frames && first + kept.len() <= k + MEDIAN {
            kept.push_back(stft.analyse(first + kept.len()));
        }
        while first + MEDIAN < k {
            kept.pop_front();
            first += 1;
        }
        let Some(frame) = kept.get(k - first) else {
            break;
        };
        for bin in 0..bins {
            scratch.clear();
            scratch.extend(kept.iter().filter_map(|f| f.magnitude.get(bin)));
            let harmonic = median(&mut scratch);
            scratch.clear();
            let low = bin.saturating_sub(MEDIAN);
            let high = (bin + MEDIAN + 1).min(bins);
            scratch.extend_from_slice(&frame.magnitude[low..high]);
            let percussive = median(&mut scratch);
            let (l, r) = sides(&frame.z, bin);
            let shares = shares(harmonic, percussive, bin as f32 * bin_hz, l, r);
            for (spectrum, share) in spectra.iter_mut().zip(shares) {
                let a = frame.z[bin];
                spectrum[bin] = (a.0 * share, a.1 * share);
                let mirror = (n - bin) % n;
                let b = frame.z[mirror];
                spectrum[mirror] = (b.0 * share, b.1 * share);
            }
        }
        for (spectrum, stem) in spectra.iter_mut().zip(&mut out) {
            stft.add(k, spectrum, stem);
        }
        if k % 32 == 0 && !progress(k as f32 / frames as f32) {
            return Err(Cancelled);
        }
    }
    let [vocals, drums, bass, other] = &mut out;
    for i in 0..len {
        other.left[i] = left[i] - vocals.left[i] - drums.left[i] - bass.left[i];
        other.right[i] = right[i] - vocals.right[i] - drums.right[i] - bass.right[i];
    }
    progress(1.0);
    Ok(Stems(out))
}

#[cfg(test)]
mod tests;
