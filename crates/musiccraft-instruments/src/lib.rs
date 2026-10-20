//! MusicCraft's instruments: one [`Voice`] per sounding note, for every
//! built-in instrument (synths, samplers, drums, plucked strings, FM, keys,
//! additive, granular, resynthesis).
//!
//! Nothing here knows about the engine, the mixer or the project. A host (the
//! engine, or a plugin wrapper) allocates voices, starts them with a
//! [`VoiceStart`] and an [`Instrument`], and asks each to add its output to a
//! block with the channel's instrument parameters ([`Params`]).
//!
//! Like the engine, everything reachable from [`Voice::render`] is free of
//! allocation, locks, I/O and panics; voices are built on the main thread.

use std::sync::Arc;

use musiccraft_dsp::SampleBuffer;
use musiccraft_params::MAX_INSTRUMENT_PARAMS;

pub mod arp;
mod kind;
pub mod live;
mod modmatrix;
mod strings;
mod voice;

pub use kind::{KINDS, Kind, NOTE_FUNCTIONS};
pub use voice::{CUT_SECONDS, Voice, VoiceStart, glides};

/// A channel's instrument parameters, indexed by `musiccraft_params::inst`.
pub type Params = [f32; MAX_INSTRUMENT_PARAMS];

/// Every instrument parameter at its default.
#[must_use]
pub fn default_params() -> Params {
    musiccraft_params::defaults(musiccraft_params::INSTRUMENT_PARAMS)
}

/// Per-note properties beyond pitch, velocity and pan.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NoteShape {
    /// Fine tuning in semitones (−1..1).
    pub fine: f32,
    /// Multiplier on the release time.
    pub release: f32,
    /// Filter cutoff offset in octaves.
    pub cutoff: f32,
    /// Filter resonance offset (−0.5..0.5).
    pub resonance: f32,
    /// Glide from the channel's previous note, retriggering.
    pub porta: bool,
    /// Colour group, 0..=15: slide notes bend only notes of their own group.
    pub color: u8,
    /// Samplers: seconds into the sample where the note starts (a sliced
    /// audio clip).
    pub start: f32,
}

impl Default for NoteShape {
    fn default() -> Self {
        Self {
            fine: 0.0,
            release: 1.0,
            cutoff: 0.0,
            resonance: 0.0,
            porta: false,
            color: 0,
            start: 0.0,
        }
    }
}

/// One sample of a zone sampler and the notes that play it.
#[derive(Debug, Clone)]
pub struct Zone {
    /// The sample.
    pub buffer: Arc<SampleBuffer>,
    /// Lowest and highest key that play it.
    pub low: u8,
    pub high: u8,
    /// Key at which it plays at its recorded pitch.
    pub root: u8,
    /// Lowest and highest velocity (1–127) that play it.
    pub vel_low: u8,
    pub vel_high: u8,
}

impl Zone {
    /// Whether a note of `key` at `velocity` (0..1) plays this zone.
    #[must_use]
    pub fn plays(&self, key: u8, velocity: f32) -> bool {
        let vel = (velocity.clamp(0.0, 1.0) * 127.0).round().clamp(1.0, 127.0) as u8;
        (self.low..=self.high).contains(&key) && (self.vel_low..=self.vel_high).contains(&vel)
    }
}

/// The sound source of a channel.
#[derive(Debug, Clone)]
pub enum Instrument {
    /// Plays a sample, pitched relative to the `root_key` parameter.
    Sampler(Arc<SampleBuffer>),
    /// Two-oscillator subtractive synthesizer.
    Synth,
    /// Synthesized drums.
    DrumSynth,
    /// Plucked string.
    Pluck,
    /// Four-operator FM synth.
    Fm,
    /// Piano, electric piano or organ.
    Keys,
    /// Additive synth.
    Additive,
    /// A sample played as grains.
    Granular(Arc<SampleBuffer>),
    /// Sixteen sample pads (`None` for an empty pad).
    Kit(Vec<Option<Arc<SampleBuffer>>>),
    /// Samples mapped to key and velocity ranges.
    Zones(Vec<Zone>),
    /// A sample's harmonics played back additively.
    Resynth(Arc<musiccraft_dsp::resynth::HarmonicTable>),
    /// Forwards notes to these channel indices (the host routes them; a
    /// layer never sounds itself).
    Layer(Vec<u16>),
    /// The host's external processor (plugin) in this slot; never gets a
    /// voice.
    External(u16),
    /// Sends its notes to external MIDI gear on this MIDI channel (0–15)
    /// instead of sounding; never gets a voice.
    MidiOut(u8),
}
