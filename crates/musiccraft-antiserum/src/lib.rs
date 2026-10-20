//! Antiserum, MusicCraft's wavetable synthesizer (see
//! `docs/spec/antiserum.md`): three wavetable oscillators with warps and
//! unison, a sub and a noise oscillator, two multimode filters, four
//! envelopes, eight LFOs, eight macros and a 32-slot mod matrix; then an
//! effects rack, an arpeggiator and a step sequencer
//! (`docs/spec/antiserum-fx-arp.md`).
//!
//! This crate is the sound engine only, independent of the app's engine and
//! of any plugin format. [`Antiserum::new`], [`Wavetable`] construction and
//! [`Antiserum::set_wavetable`] allocate and belong on the main thread;
//! everything else, [`Antiserum::process`] included, is allocation-, lock-
//! and panic-free and may run on the audio thread.

mod filter;
pub mod fx;
pub mod modulation;
mod noise;
pub mod notes;
pub mod params;
mod player;
pub mod source;
mod synth;
mod voice;
mod warp;
pub mod wavetable;

pub use filter::FilterType;
pub use modulation::{LfoPoint, LfoShape};
pub use notes::{Sequence, Step};
pub use params::{PARAM_COUNT, choices, index_of, params};
pub use source::{Source, Zone};
pub use synth::{Antiserum, Data, MAX_VOICES};
pub use voice::{CONTROL, MAX_UNISON};
pub use warp::Warp;
pub use wavetable::Wavetable;

#[cfg(test)]
mod fx_arp_tests;
#[cfg(test)]
mod osc_stereo_tests;
#[cfg(test)]
mod osc_type_tests;
#[cfg(test)]
mod tests;
