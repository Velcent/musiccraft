//! Wavetables, band-limited per octave so an oscillator never aliases.

use musiccraft_dsp::fft::{Complex, Fft};

/// Samples in one frame of a source table.
pub const FRAME_LEN: usize = 2048;
/// Most frames a table holds.
pub const MAX_FRAMES: usize = 256;
/// Band-limited levels, one per octave: 1024 harmonics down to 1.
pub const LEVELS: usize = 11;

/// One band-limited copy of every frame.
#[derive(Debug, Clone)]
struct Level {
    /// Highest harmonic kept.
    harmonics: usize,
    /// Samples per frame (a power of two).
    len: usize,
    /// Frames one after another.
    data: Vec<f32>,
}

/// A wavetable: 1 to 256 single-cycle frames, stored at [`LEVELS`] bandwidths.
/// Built on the main thread; reading never allocates.
#[derive(Debug, Clone)]
pub struct Wavetable {
    frames: usize,
    levels: Vec<Level>,
}

impl Wavetable {
    /// Builds a table from frames of [`FRAME_LEN`] samples laid end to end.
    /// A partial last frame and frames past [`MAX_FRAMES`] are ignored; `None`
    /// if there is no whole frame.
    #[must_use]
    pub fn new(samples: &[f32]) -> Option<Self> {
        let frames = (samples.len() / FRAME_LEN).min(MAX_FRAMES);
        if frames == 0 {
            return None;
        }
        let fft = Fft::new(FRAME_LEN);
        let spectra: Vec<Vec<Complex>> = samples
            .chunks_exact(FRAME_LEN)
            .take(frames)
            .map(|frame| {
                let mut buf: Vec<Complex> = frame
                    .iter()
                    .map(|&x| (if x.is_finite() { x } else { 0.0 }, 0.0))
                    .collect();
                fft.forward(&mut buf);
                buf
            })
            .collect();
        let levels = (0..LEVELS)
            .map(|level| build_level(&spectra, (FRAME_LEN / 2) >> level))
            .collect();
        Some(Self { frames, levels })
    }

    /// Builds a table from a function of (frame position 0..1, phase 0..1).
    #[must_use]
    pub fn from_fn(frames: usize, f: impl Fn(f32, f32) -> f32) -> Self {
        let frames = frames.clamp(1, MAX_FRAMES);
        let mut samples = Vec::with_capacity(frames * FRAME_LEN);
        for frame in 0..frames {
            let y = if frames > 1 {
                frame as f32 / (frames - 1) as f32
            } else {
                0.0
            };
            samples.extend((0..FRAME_LEN).map(|i| f(y, i as f32 / FRAME_LEN as f32)));
        }
        // Whole frames by construction.
        Self::new(&samples).unwrap_or_else(Self::silent)
    }

    /// A single silent frame.
    #[must_use]
    pub fn silent() -> Self {
        let levels = (0..LEVELS)
            .map(|level| {
                let harmonics = (FRAME_LEN / 2) >> level;
                let len = level_len(harmonics);
                Level {
                    harmonics,
                    len,
                    data: vec![0.0; len],
                }
            })
            .collect();
        Self { frames: 1, levels }
    }

    /// The default table: 64 frames morphing sine → triangle → saw → square.
    #[must_use]
    pub fn basic() -> Self {
        Self::from_fn(64, |y, p| {
            let sine = (p * std::f32::consts::TAU).sin();
            let triangle = 1.0 - 4.0 * ((p + 0.25).rem_euclid(1.0) - 0.5).abs();
            let saw = 2.0 * (p + 0.5).rem_euclid(1.0) - 1.0;
            let square = if p < 0.5 { 1.0 } else { -1.0 };
            let shapes = [sine, triangle, saw, square];
            let x = y * 3.0;
            let i = (x.floor() as usize).min(2);
            let t = x - i as f32;
            shapes[i] * (1.0 - t) + shapes[i + 1] * t
        })
    }

    /// Number of frames.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.frames
    }

    /// The level to read for a phase increment (cycles per sample): the one
    /// with the most harmonics that all stay below Nyquist.
    #[must_use]
    pub fn level_for(&self, increment: f32) -> usize {
        let limit = 0.5 / increment.abs().max(1e-9);
        self.levels
            .iter()
            .position(|level| level.harmonics as f32 <= limit)
            .unwrap_or(LEVELS - 1)
    }

    /// Frame pair and blend for a position 0..1.
    #[must_use]
    pub fn frame_pos(&self, position: f32) -> (usize, usize, f32) {
        let x = position.clamp(0.0, 1.0) * (self.frames - 1) as f32;
        let a = (x.floor() as usize).min(self.frames - 1);
        let b = (a + 1).min(self.frames - 1);
        (a, b, x - a as f32)
    }

    /// Frames `a` and `b` of `level`, blended by `mix`, ready to read.
    #[must_use]
    pub fn frames_at(&self, level: usize, a: usize, b: usize, mix: f32) -> Option<FramePair<'_>> {
        let level = self.levels.get(level)?;
        let frame = |f: usize| level.data.get(f * level.len..(f + 1) * level.len);
        let a_data = frame(a)?;
        let b_data = if mix > 0.0 && a != b { frame(b) } else { None };
        Some(FramePair {
            a: a_data,
            b: b_data,
            mix,
            len: level.len as f32,
            mask: level.len - 1,
        })
    }

    /// One sample at `phase` (0..1) from `level`, blending frames `a` and `b`
    /// by `mix`, interpolated linearly.
    #[must_use]
    pub fn read(&self, level: usize, a: usize, b: usize, mix: f32, phase: f32) -> f32 {
        self.frames_at(level, a, b, mix)
            .map_or(0.0, |pair| pair.read(phase.rem_euclid(1.0)))
    }
}

/// Two frames of one level, blended: what an oscillator reads for a block.
#[derive(Debug, Clone, Copy)]
pub struct FramePair<'a> {
    a: &'a [f32],
    b: Option<&'a [f32]>,
    mix: f32,
    len: f32,
    mask: usize,
}

impl FramePair<'_> {
    /// The sample at `phase` (0..1; values a little outside wrap).
    #[inline]
    #[must_use]
    pub fn read(&self, phase: f32) -> f32 {
        let x = phase * self.len;
        let floor = x.floor();
        let i = (floor as i64 as usize) & self.mask;
        let j = (i + 1) & self.mask;
        let t = x - floor;
        let lerp = |data: &[f32]| {
            let s0 = data.get(i).copied().unwrap_or(0.0);
            let s1 = data.get(j).copied().unwrap_or(0.0);
            s0 + (s1 - s0) * t
        };
        let sa = lerp(self.a);
        match self.b {
            Some(b) => sa + (lerp(b) - sa) * self.mix,
            None => sa,
        }
    }
}

/// Samples per frame for a level keeping `harmonics`: four per harmonic,
/// between 64 and [`FRAME_LEN`].
fn level_len(harmonics: usize) -> usize {
    (harmonics * 4).clamp(64, FRAME_LEN)
}

fn build_level(spectra: &[Vec<Complex>], harmonics: usize) -> Level {
    let len = level_len(harmonics);
    let fft = Fft::new(len);
    let mut data = Vec::with_capacity(spectra.len() * len);
    let mut buf = vec![(0.0, 0.0); len];
    let scale = len as f32 / FRAME_LEN as f32;
    for spectrum in spectra {
        buf.fill((0.0, 0.0));
        // Bin 0 (DC) stays zero; positive and mirrored negative harmonics.
        for h in 1..=harmonics.min(len / 2 - 1) {
            let (re, im) = spectrum.get(h).copied().unwrap_or((0.0, 0.0));
            buf[h] = (re * scale, im * scale);
            buf[len - h] = (re * scale, -im * scale);
        }
        fft.inverse(&mut buf);
        data.extend(buf.iter().map(|c| c.0));
    }
    Level {
        harmonics,
        len,
        data,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_keep_the_harmonics_below_nyquist() {
        let table = Wavetable::from_fn(1, |_, p| 1.0 - 2.0 * p);
        assert_eq!(table.level_for(0.0001), 0);
        // 1 kHz at 48 kHz: at most 24 harmonics, so the 16-harmonic level.
        let level = table.level_for(1_000.0 / 48_000.0);
        assert_eq!(table.levels[level].harmonics, 16);
        assert_eq!(table.level_for(0.49), LEVELS - 1);
    }

    #[test]
    fn a_sine_survives_every_level() {
        let table = Wavetable::from_fn(1, |_, p| (p * std::f32::consts::TAU).sin());
        for level in 0..LEVELS {
            let peak = (0..64)
                .map(|i| table.read(level, 0, 0, 0.0, i as f32 / 64.0))
                .fold(0.0_f32, |m, x| m.max(x));
            assert!((peak - 1.0).abs() < 0.01, "level {level}: {peak}");
            let quarter = table.read(level, 0, 0, 0.0, 0.25);
            assert!((quarter - 1.0).abs() < 0.01, "level {level}: {quarter}");
        }
    }

    #[test]
    fn dc_is_removed_and_frames_blend() {
        let table = Wavetable::from_fn(2, |y, _| y + 0.5);
        assert!(table.read(0, 0, 0, 0.0, 0.3).abs() < 1e-4);
        let basic = Wavetable::basic();
        assert_eq!(basic.frames(), 64);
        let (a, b, mix) = basic.frame_pos(0.5);
        assert_eq!((a, b), (31, 32));
        assert!((mix - 0.5).abs() < 1e-3);
        assert!(Wavetable::new(&[0.0; 100]).is_none());
    }
}
