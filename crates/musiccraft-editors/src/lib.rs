//! The egui pieces the app and the exported plugins share: the theme, the
//! knobs and other widgets, hints, and the effect editors. Nothing here knows
//! about projects or commands; editors report what the user changed and the
//! caller applies it.

pub mod antiserum;
pub mod effect;
pub mod format;
pub mod hint;
pub mod instrument;
pub mod theme;
mod tuner;
pub mod wavetable;
pub mod widgets;
