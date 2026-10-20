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

/// An oscillator's sample, its analysis and its zones.
#[derive(Debug, Clone)]
pub struct Source {
    /// The sample, if any.
    pub sample: Option<Arc<SampleBuffer>>,
    /// The sample's harmonics over time (silent without a sample).
    pub harmonics: Arc<HarmonicTable>,
    /// Multisample zones, in the order notes look for a match.
    pub zones: Vec<Zone>,
}

impl Default for Source {
    fn default() -> Self {
        Self {
            sample: None,
            harmonics: Arc::new(HarmonicTable::silent()),
            zones: Vec::new(),
        }
    }
}

impl Source {
    /// A source from a sample (analysed here, which is slow) and zones.
    /// Main thread.
    #[must_use]
    pub fn new(sample: Option<Arc<SampleBuffer>>, zones: Vec<Zone>) -> Self {
        let harmonics = sample
            .as_deref()
            .map_or_else(HarmonicTable::silent, HarmonicTable::analyze);
        Self::with_analysis(sample, Arc::new(harmonics), zones)
    }

    /// A source with an analysis made (and perhaps cached) elsewhere.
    #[must_use]
    pub fn with_analysis(
        sample: Option<Arc<SampleBuffer>>,
        harmonics: Arc<HarmonicTable>,
        zones: Vec<Zone>,
    ) -> Self {
        Self {
            sample: sample.filter(|s| s.len() > 1),
            harmonics,
            zones: zones.into_iter().filter(|z| z.buffer.len() > 1).collect(),
        }
    }
}
