//! What sample-based oscillators play (`docs/spec/antiserum-oscillators.md`):
//! one sample with its harmonic analysis, and multisample zones. Built on the
//! main thread; read without allocating.

use std::sync::Arc;

use musiccraft_dsp::SampleBuffer;
use musiccraft_dsp::resynth::HarmonicTable;

/// One multisample zone, as in the zone sampler.
#[derive(Debug, Clone)]
pub struct Zone {
    /// The zone's sample.
    pub buffer: Arc<SampleBuffer>,
    /// Lowest and highest key it plays.
    pub low: u8,
    pub high: u8,
    /// The key at which it plays at its own speed.
    pub root: u8,
    /// Lowest and highest velocity it plays (1–127).
    pub vel_low: u8,
    pub vel_high: u8,
}

impl Zone {
    /// Whether the zone plays `key` at `velocity` (0..1).
    #[must_use]
    pub fn plays(&self, key: u8, velocity: f32) -> bool {
        let v = (velocity.clamp(0.0, 1.0) * 127.0).round().max(1.0) as u8;
        (self.low..=self.high).contains(&key) && (self.vel_low..=self.vel_high).contains(&v)
    }
}

/// A sample's harmonics over time, for its left and right channels (the same
/// table twice for a mono sample).
pub type Analysis = [Arc<HarmonicTable>; 2];

/// The analysis of nothing: silence on both sides.
#[must_use]
pub fn silent_analysis() -> Analysis {
    let silent = Arc::new(HarmonicTable::silent());
    [Arc::clone(&silent), silent]
}

/// Analyses each channel of `sample` (once when they are the same). Slow;
/// main thread.
#[must_use]
pub fn analyze(sample: &SampleBuffer) -> Analysis {
    if sample.left() == sample.right() {
        let table = Arc::new(HarmonicTable::analyze(sample));
        return [Arc::clone(&table), table];
    }
    let rate = sample.sample_rate();
    let side = |data: &[f32]| {
        Arc::new(HarmonicTable::analyze(&SampleBuffer::mono(
            data.to_vec(),
            rate,
        )))
    };
    [side(sample.left()), side(sample.right())]
}

/// An oscillator's sample, its analysis and its zones.
#[derive(Debug, Clone)]
pub struct Source {
    /// The sample, if any.
    pub sample: Option<Arc<SampleBuffer>>,
    /// The sample's harmonics over time, left and right (silent without a
    /// sample).
    pub harmonics: Analysis,
    /// Multisample zones, in the order notes look for a match.
    pub zones: Vec<Zone>,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            sample: None,
            harmonics: silent_analysis(),
            zones: Vec::new(),
        }
    }
}

impl Source {
    /// A source from a sample (analysed here, which is slow) and zones.
    /// Main thread.
    #[must_use]
    pub fn new(sample: Option<Arc<SampleBuffer>>, zones: Vec<Zone>) -> Self {
        let harmonics = sample.as_deref().map_or_else(silent_analysis, analyze);
        Self::with_analysis(sample, harmonics, zones)
    }

    /// A source with an analysis made (and perhaps cached) elsewhere.
    #[must_use]
    pub fn with_analysis(
        sample: Option<Arc<SampleBuffer>>,
        harmonics: Analysis,
        zones: Vec<Zone>,
    ) -> Self {
        Self {
            sample: sample.filter(|s| s.len() > 1),
            harmonics,
            zones: zones.into_iter().filter(|z| z.buffer.len() > 1).collect(),
        }
    }
}
