use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

use musiccraft_params::MAX_MIXER_TRACKS;

/// Values the engine publishes for display: playhead, transport and peak levels.
/// Shared between threads through atomics only.
#[derive(Debug)]
pub struct Meters {
    position_bits: AtomicU64,
    playing: AtomicBool,
    active_voices: AtomicU32,
    peaks: Vec<AtomicU32>,
}

impl Default for Meters {
    fn default() -> Self {
        Self::new()
    }
}

impl Meters {
    /// Allocates meters for [`MAX_MIXER_TRACKS`] stereo tracks.
    #[must_use]
    pub fn new() -> Self {
        Self {
            position_bits: AtomicU64::new(0.0_f64.to_bits()),
            playing: AtomicBool::new(false),
            active_voices: AtomicU32::new(0),
            peaks: (0..MAX_MIXER_TRACKS * 2)
                .map(|_| AtomicU32::new(0))
                .collect(),
        }
    }

    /// Playhead position in ticks.
    #[must_use]
    pub fn position_ticks(&self) -> f64 {
        f64::from_bits(self.position_bits.load(Ordering::Relaxed))
    }

    /// Whether the engine is playing.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    /// Number of sounding voices.
    #[must_use]
    pub fn active_voices(&self) -> u32 {
        self.active_voices.load(Ordering::Relaxed)
    }

    /// Returns the highest peak (left, right) since the last call and resets it.
    #[must_use]
    pub fn take_peak(&self, track: usize) -> (f32, f32) {
        let take = |i: usize| {
            self.peaks
                .get(i)
                .map_or(0.0, |p| f32::from_bits(p.swap(0, Ordering::Relaxed)))
        };
        (take(track * 2), take(track * 2 + 1))
    }

    pub(crate) fn publish(&self, position: f64, playing: bool, voices: u32) {
        self.position_bits
            .store(position.to_bits(), Ordering::Relaxed);
        self.playing.store(playing, Ordering::Relaxed);
        self.active_voices.store(voices, Ordering::Relaxed);
    }

    pub(crate) fn record_peak(&self, track: usize, left: f32, right: f32) {
        // For non-negative floats the bit patterns order the same as the values.
        for (i, value) in [(track * 2, left), (track * 2 + 1, right)] {
            if let Some(peak) = self.peaks.get(i) {
                let value = if value.is_finite() { value.abs() } else { 0.0 };
                peak.fetch_max(value.to_bits(), Ordering::Relaxed);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peaks_hold_until_taken() {
        let meters = Meters::new();
        meters.record_peak(3, 0.5, 0.25);
        meters.record_peak(3, 0.2, 0.75);
        assert_eq!(meters.take_peak(3), (0.5, 0.75));
        assert_eq!(meters.take_peak(3), (0.0, 0.0));
        meters.record_peak(MAX_MIXER_TRACKS + 5, 1.0, 1.0);
        assert_eq!(meters.take_peak(MAX_MIXER_TRACKS + 5), (0.0, 0.0));
    }
}
