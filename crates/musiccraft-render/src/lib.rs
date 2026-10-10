//! The bridge between the project model and the real-time engine.
//!
//! [`Bridge::sync`] compares a [`Project`] with what the engine last received and
//! sends only what changed: parameter updates as small messages, a new
//! [`Sequence`] when notes or the arrangement change, and a new [`Graph`] only
//! when channels, samples or effect chains are added, removed or replaced.
//!
//! [`render`] and [`export_wav`] run the same engine offline.

mod bridge;
mod export;
mod sequence;

pub use bridge::{Bridge, SyncReport};
pub use export::{ExportCommand, RenderOptions, export_wav, register, render};
pub use sequence::build_sequence;

#[cfg(test)]
mod tests;
