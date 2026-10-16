//! Tap tempo: turns the times of taps into a tempo.

use musiccraft_project::{MAX_TEMPO_BPM, MIN_TEMPO_BPM};

/// Taps further apart than this start a new count.
pub const RESET_SECONDS: f64 = 2.0;
/// Taps averaged.
pub const MAX_TAPS: usize = 8;

/// Adds a tap at `now` (seconds) to `taps` and returns the tempo they give:
/// the average interval of up to the last [`MAX_TAPS`] taps, clamped to the
/// project's tempo range. A pause over [`RESET_SECONDS`] starts again; one
/// tap gives no tempo yet.
pub fn tap(taps: &mut Vec<f64>, now: f64) -> Option<f64> {
    if taps
        .last()
        .is_some_and(|last| now - last > RESET_SECONDS || now < *last)
    {
        taps.clear();
    }
    taps.push(now);
    if taps.len() > MAX_TAPS {
        taps.remove(0);
    }
    let (first, last) = (taps.first()?, taps.last()?);
    let intervals = taps.len().checked_sub(1).filter(|n| *n > 0)?;
    let average = (last - first) / intervals as f64;
    (average > 0.0).then(|| (60.0 / average).clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_taps_give_their_tempo_and_a_pause_resets() {
        let mut taps = Vec::new();
        assert_eq!(tap(&mut taps, 10.0), None, "one tap is not a tempo");
        let bpm = tap(&mut taps, 10.5).unwrap();
        assert!((bpm - 120.0).abs() < 1e-9);
        for t in [11.0, 11.5, 12.0] {
            tap(&mut taps, t);
        }
        assert!((tap(&mut taps, 12.5).unwrap() - 120.0).abs() < 1e-9);
        // Only the last eight taps count.
        for i in 0..20 {
            tap(&mut taps, 13.0 + f64::from(i) * 0.4);
        }
        assert_eq!(taps.len(), MAX_TAPS);
        assert!((tap(&mut taps, 13.0 + 20.0 * 0.4).unwrap() - 150.0).abs() < 1e-6);
        // A long pause starts over.
        assert_eq!(tap(&mut taps, 100.0), None);
        // Slow taps give a slow tempo.
        assert!((tap(&mut taps, 101.9).unwrap() - 31.578_947_368_421_05).abs() < 1e-6);
    }
}
