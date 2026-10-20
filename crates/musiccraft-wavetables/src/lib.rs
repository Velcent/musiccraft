//! Antiserum's editable wavetables (see `docs/spec/antiserum-wavetables.md`):
//! the [`Table`] of frames, the generated [`factory`] tables, WAV [`import`]
//! and [`export`], and the editing [`Op`]s.
//!
//! Everything here runs on the main thread and allocates freely. The synth
//! plays a table through `musiccraft_antiserum::Wavetable::new(table.samples())`.

pub mod export;
pub mod factory;
pub mod import;
mod ops;
mod resample;
pub mod spectrum;
mod table;

pub use ops::{MorphMode, Op};
pub use table::{FRAME_LEN, MAX_FRAMES, Table};

#[cfg(test)]
mod tests;
