//! Resynthesis: a sample's harmonic levels over time, measured once on the
//! main thread, for the resynth instrument to play back additively at any
//! pitch and speed.

use crate::SampleBuffer;
use crate::fft::{Complex, Fft};
use crate::pitch::PitchDetector;

/// Harmonics measured per frame.
pub const RESYNTH_PARTIALS: usize = 32;
/// Analysis window, in samples (a power of two).
const WINDOW: usize = 4_096;
/// Most frames a table keeps.
const MAX_FRAMES: usize = 512;
/// Fundamental assumed when none can be found.
const FALLBACK_HZ: f32 = 220.0;

/// Harmonic levels over time.
#[derive(Debug, Clone, PartialEq)]
pub struct HarmonicTable {
    /// Per frame, the level of harmonics 1..=32, the loudest at 1.
    pub frames: Vec<[f32; RESYNTH_PARTIALS]>,
    /// Seconds between frames.
    pub frame_seconds: f32,
    /// The fundamental the harmonics were measured against, in hertz.
    pub fundamental: f32,
    /// The largest sum of one frame's levels, for normalizing playback.
    pub peak_sum: f32,
}

impl HarmonicTable {
    /// A table of one silent frame.
    #[must_use]
    pub fn silent() -> Self {
        Self {
            frames: vec![[0.0; RESYNTH_PARTIALS]],
            frame_seconds: 1.0,
            fundamental: FALLBACK_HZ,
            peak_sum: 1.0,
        }
    }

    /// Measures `buffer`. Main thread only (it allocates and is slow).
    #[must_use]
    pub fn analyze(buffer: &SampleBuffer) -> Self {
        let mono: Vec<f32> = buffer
            .left()
            .iter()
            .zip(buffer.right())
            .map(|(l, r)| (l + r) * 0.5)
            .collect();
        let rate = buffer.sample_rate() as f32;
        if mono.is_empty() || !rate.is_finite() || rate <= 0.0 {
            return Self::silent();
        }
        let fundamental = fundamental(&mono, rate);
        let hop = (mono.len() / MAX_FRAMES).max(WINDOW / 4);
        let fft = Fft::new(WINDOW);
        let hann: Vec<f32> = (0..WINDOW)
            .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / WINDOW as f32).cos())
            .collect();
        let mut frames = Vec::new();
        let mut spectrum: Vec<Complex> = vec![(0.0, 0.0); WINDOW];
        let mut centre = 0;
        while centre < mono.len() && frames.len() < MAX_FRAMES {
            for (i, (slot, w)) in spectrum.iter_mut().zip(&hann).enumerate() {
                let at = (centre + i).checked_sub(WINDOW / 2);
                let x = at.and_then(|a| mono.get(a)).copied().unwrap_or(0.0);
                *slot = (x * w, 0.0);
            }
            fft.forward(&mut spectrum);
            let magnitude = |bin: usize| {
                spectrum
                    .get(bin)
                    .map_or(0.0, |(re, im)| (re * re + im * im).sqrt())
            };
            let bin_hz = rate / WINDOW as f32;
            let frame: [f32; RESYNTH_PARTIALS] = std::array::from_fn(|k| {
                let hz = fundamental * (k + 1) as f32;
                if hz >= rate * 0.5 {
                    return 0.0;
                }
                // The strongest bin near the harmonic, to allow for slight
                // detuning and vibrato.
                let bin = (hz / bin_hz).round() as usize;
                (bin.saturating_sub(2)..=bin + 2)
                    .map(magnitude)
                    .fold(0.0_f32, f32::max)
            });
            frames.push(frame);
            centre += hop;
        }
        let loudest = frames
            .iter()
            .flat_map(|f| f.iter())
            .fold(0.0_f32, |m, v| m.max(*v));
        if loudest > 0.0 {
            for frame in &mut frames {
                for v in frame.iter_mut() {
                    *v /= loudest;
                }
            }
        }
        if frames.is_empty() {
            return Self::silent();
        }
        let peak_sum = frames
            .iter()
            .map(|f| f.iter().sum::<f32>())
            .fold(0.0_f32, f32::max)
            .max(1e-3);
        Self {
            frames,
            frame_seconds: hop as f32 / rate,
            fundamental,
            peak_sum,
        }
    }

    /// Length in seconds.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.frames.len() as f32 * self.frame_seconds
    }

    /// Harmonic levels `seconds` in, interpolated between frames. Past the
    /// end it wraps when `looped`, else holds the last frame.
    #[must_use]
    pub fn at(&self, seconds: f32, looped: bool) -> [f32; RESYNTH_PARTIALS] {
        let count = self.frames.len();
        if count == 0 {
            return [0.0; RESYNTH_PARTIALS];
        }
        let mut pos = (seconds / self.frame_seconds.max(1e-6)).max(0.0);
        if looped {
            pos = pos.rem_euclid(count as f32);
        }
        let i = (pos.floor() as usize).min(count - 1);
        let frac = if i + 1 < count || looped {
            pos - pos.floor()
        } else {
            0.0
        };
        let a = self
            .frames
            .get(i)
            .copied()
            .unwrap_or([0.0; RESYNTH_PARTIALS]);
        let b = self.frames.get((i + 1) % count).copied().unwrap_or(a);
        std::array::from_fn(|k| a[k] + (b[k] - a[k]) * frac)
    }
}

/// The median pitch the detector hears over the sample, or the strongest
/// low spectral peak when it hears none.
fn fundamental(mono: &[f32], rate: f32) -> f32 {
    let mut detector = PitchDetector::new(rate);
    let mut heard = Vec::new();
    for x in mono {
        if detector.push(*x) {
            if let Some(hz) = detector.pitch() {
                heard.push(hz);
            }
        }
    }
    if heard.is_empty() {
        return FALLBACK_HZ;
    }
    heard.sort_by(f32::total_cmp);
    heard.get(heard.len() / 2).copied().unwrap_or(FALLBACK_HZ)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tone with harmonics 1, 2 and 3 at levels 1, 0.5, 0.25 whose second
    /// harmonic fades out over the second half.
    fn tone(hz: f32, seconds: f32) -> SampleBuffer {
        let rate = 48_000.0;
        let n = (seconds * rate) as usize;
        let data: Vec<f32> = (0..n)
            .map(|i| {
                let t = i as f32 / rate;
                let fade = (2.0 - 2.0 * i as f32 / n as f32).min(1.0);
                let w = std::f32::consts::TAU * hz * t;
                0.4 * (w.sin() + 0.5 * fade * (2.0 * w).sin() + 0.25 * (3.0 * w).sin())
            })
            .collect();
        SampleBuffer::mono(data, f64::from(rate))
    }

    #[test]
    fn analysis_finds_the_fundamental_and_harmonic_levels() {
        let table = HarmonicTable::analyze(&tone(220.0, 2.0));
        assert!(
            (table.fundamental - 220.0).abs() < 3.0,
            "{}",
            table.fundamental
        );
        assert!((table.duration() - 2.0).abs() < 0.2, "{}", table.duration());
        let early = table.at(0.3, false);
        assert!((early[0] - 1.0).abs() < 0.05, "{early:?}");
        assert!((early[1] / early[0] - 0.5).abs() < 0.1);
        assert!((early[2] / early[0] - 0.25).abs() < 0.08);
        assert!(early[4] < 0.05, "nothing at the fifth harmonic");
        let late = table.at(1.9, false);
        assert!(late[1] < early[1] * 0.3, "the second harmonic faded");
    }

    #[test]
    fn looping_wraps_and_holding_keeps_the_last_frame() {
        let table = HarmonicTable::analyze(&tone(220.0, 1.0));
        let d = table.duration();
        assert_eq!(table.at(d * 3.0, false), table.at(d * 10.0, false));
        let wrapped = table.at(d + 0.1, true);
        let start = table.at(0.1, true);
        for (a, b) in wrapped.iter().zip(&start) {
            assert!((a - b).abs() < 1e-4);
        }
        assert_eq!(
            HarmonicTable::analyze(&SampleBuffer::mono(vec![], 48_000.0)),
            HarmonicTable::silent()
        );
    }
}
