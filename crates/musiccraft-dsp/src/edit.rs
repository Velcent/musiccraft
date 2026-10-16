//! Offline sample edits for the sample editor: time-stretching without
//! changing pitch (WSOLA), pitch-shifting without changing length, and
//! linear resampling. Main thread only: these allocate.

/// WSOLA frame length in samples (at 48 kHz, about 43 ms).
const FRAME: usize = 2_048;
/// How far each frame may move to line up with the last, in samples.
const SEEK: usize = 512;

/// Resamples `input` by `factor` (output length ≈ input length × `factor`)
/// with linear interpolation.
#[must_use]
pub fn resample(input: &[f32], factor: f64) -> Vec<f32> {
    if input.is_empty() || !factor.is_finite() || factor <= 0.0 {
        return Vec::new();
    }
    let len = ((input.len() as f64) * factor).round().max(1.0) as usize;
    (0..len)
        .map(|i| {
            let pos = i as f64 / factor;
            let whole = pos.floor() as usize;
            let frac = (pos - whole as f64) as f32;
            let a = input.get(whole).copied().unwrap_or(0.0);
            let b = input.get(whole + 1).copied().unwrap_or(a);
            a + (b - a) * frac
        })
        .collect()
}

/// The offset in `-SEEK..=SEEK` at which `input[nominal + offset..]` best
/// matches `target` (highest cross-correlation).
fn best_offset(input: &[f32], nominal: usize, target: &[f32]) -> isize {
    let mut best = (0_isize, f32::MIN);
    let seek = SEEK as isize;
    for offset in -seek..=seek {
        let Some(start) = nominal.checked_add_signed(offset) else {
            continue;
        };
        let Some(segment) = input.get(start..start + target.len()) else {
            continue;
        };
        let score: f32 = segment.iter().zip(target).map(|(a, b)| a * b).sum();
        if score > best.1 {
            best = (offset, score);
        }
    }
    best.0
}

/// Stretches a stereo pair to `ratio` times its length, keeping its pitch
/// (waveform-similarity overlap-add: Hann-windowed frames at half-frame
/// hops, each moved by up to `SEEK` samples to line up with the last frame's
/// natural continuation). Both sides use the same alignment, measured on
/// their mix, so the stereo image holds.
#[must_use]
pub fn stretch(left: &[f32], right: &[f32], ratio: f64) -> (Vec<f32>, Vec<f32>) {
    let len = left.len().min(right.len());
    if len == 0 || !ratio.is_finite() || ratio <= 0.0 {
        return (Vec::new(), Vec::new());
    }
    let mix: Vec<f32> = left.iter().zip(right).map(|(l, r)| (l + r) * 0.5).collect();
    let hop_out = FRAME / 2;
    let hop_in = hop_out as f64 / ratio;
    let out_len = ((len as f64) * ratio).round() as usize;
    let mut out = (
        vec![0.0_f32; out_len + FRAME],
        vec![0.0_f32; out_len + FRAME],
    );
    let mut weight = vec![0.0_f32; out_len + FRAME];
    let window: Vec<f32> = (0..FRAME)
        .map(|i| 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / FRAME as f32).cos())
        .collect();
    // Pad so frames near the ends can be read whole.
    let pad = |x: &[f32]| {
        let mut v = vec![0.0; SEEK + FRAME];
        v.extend_from_slice(&x[..len]);
        v.extend(std::iter::repeat_n(0.0, SEEK + FRAME * 2));
        v
    };
    let (pl, pr, pm) = (pad(left), pad(right), pad(&mix));
    let base = SEEK + FRAME;
    let mut previous: Option<usize> = None;
    let mut k = 0;
    while k * hop_out < out_len {
        let nominal = base + (k as f64 * hop_in).round() as usize;
        let start = match previous {
            // Line this frame up with how the last one would have continued.
            Some(prev) => {
                let target = pm
                    .get(prev + hop_out..prev + hop_out + FRAME / 2)
                    .unwrap_or(&[]);
                nominal.saturating_add_signed(best_offset(&pm, nominal, target))
            }
            None => nominal,
        };
        let at = k * hop_out;
        for (i, w) in window.iter().enumerate() {
            let (Some(l), Some(r)) = (pl.get(start + i), pr.get(start + i)) else {
                break;
            };
            if let (Some(ol), Some(or), Some(ow)) = (
                out.0.get_mut(at + i),
                out.1.get_mut(at + i),
                weight.get_mut(at + i),
            ) {
                *ol += l * w;
                *or += r * w;
                *ow += w;
            }
        }
        previous = Some(start);
        k += 1;
    }
    for ((l, r), w) in out.0.iter_mut().zip(out.1.iter_mut()).zip(&weight) {
        if *w > 1e-3 {
            *l /= w;
            *r /= w;
        }
    }
    out.0.truncate(out_len);
    out.1.truncate(out_len);
    out
}

/// Shifts a stereo pair by `semitones`, keeping its length: stretch by the
/// pitch ratio, then resample back.
#[must_use]
pub fn pitch_shift(left: &[f32], right: &[f32], semitones: f64) -> (Vec<f32>, Vec<f32>) {
    let ratio = 2.0_f64.powf(semitones / 12.0);
    let (l, r) = stretch(left, right, ratio);
    let mut l = resample(&l, 1.0 / ratio);
    let mut r = resample(&r, 1.0 / ratio);
    let len = left.len().min(right.len());
    l.resize(len, 0.0);
    r.resize(len, 0.0);
    (l, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(hz: f32, n: usize) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * 0.5)
            .collect()
    }

    fn crossings_hz(x: &[f32]) -> f32 {
        let c = x.windows(2).filter(|w| w[0] <= 0.0 && w[1] > 0.0).count();
        c as f32 * 48_000.0 / x.len() as f32
    }

    #[test]
    fn resampling_changes_length_and_pitch() {
        let x = sine(440.0, 48_000);
        let y = resample(&x, 0.5);
        assert_eq!(y.len(), 24_000);
        assert!((crossings_hz(&y) - 880.0).abs() < 10.0);
    }

    #[test]
    fn stretching_keeps_the_pitch() {
        let x = sine(330.0, 48_000);
        for ratio in [0.5, 1.5, 2.0] {
            let (l, r) = stretch(&x, &x, ratio);
            assert_eq!(l.len(), (48_000.0 * ratio) as usize);
            assert_eq!(l, r);
            let middle = &l[l.len() / 4..l.len() * 3 / 4];
            assert!(
                (crossings_hz(middle) - 330.0).abs() < 6.0,
                "ratio {ratio}: {}",
                crossings_hz(middle)
            );
            let peak = middle.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
            assert!(peak > 0.4 && peak < 0.6, "level kept: {peak}");
        }
    }

    #[test]
    fn pitch_shift_keeps_the_length() {
        let x = sine(220.0, 48_000);
        let (l, _) = pitch_shift(&x, &x, 12.0);
        assert_eq!(l.len(), x.len());
        let middle = &l[12_000..36_000];
        assert!(
            (crossings_hz(middle) - 440.0).abs() < 10.0,
            "{}",
            crossings_hz(middle)
        );
    }

    #[test]
    fn empty_and_bad_input_give_nothing() {
        assert!(resample(&[], 2.0).is_empty());
        assert!(stretch(&[1.0], &[1.0], f64::NAN).0.is_empty());
    }
}
