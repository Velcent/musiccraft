//! The bridge between the project model and the real-time engine.
//!
//! [`Bridge::sync`] compares a [`Project`] with what the engine last received and
//! sends only what changed: parameter updates as small messages, a new
//! [`Sequence`] when notes or the arrangement change, and a new [`Graph`] only
//! when channels, samples or effect chains are added, removed or replaced.
//!
//! [`render`] and [`export_wav`] run the same engine offline.

mod antiserum;
mod arp;
mod bridge;
mod consolidate;
mod delivery;
mod export;
mod midi_export;
pub mod plugins;
mod sample_edit;
mod sequence;
mod stem_engine;
mod stems;

pub use bridge::{Bridge, SyncReport};
pub use consolidate::ConsolidateCommand;
pub use export::{
    ExportCommand, ExportFlacCommand, ExportLossyCommand, ExportStemsCommand, Lossy, RenderOptions,
    export_stems, export_wav, register, register_with, render, render_with,
};
pub use midi_export::{ExportMidiCommand, midi_file};
pub use plugins::{
    CombinedHost, LivePlugin, PluginEvent, PluginFactory, PluginHost, PluginInfo, PluginInstance,
    PluginKind, PluginParam, PluginRequest,
};
pub use sample_edit::SampleEditCommand;
pub use sequence::{build_full_sequence, build_sequence};
pub use stem_engine::{
    Engine as StemEngine, MODEL_FILE as STEM_MODEL_FILE, MODEL_URL as STEM_MODEL_URL,
    model_path as stem_model_path,
};
pub use stems::{AddStemsCommand, SplitStemsCommand, StemJob};

use musiccraft_commands::Registry;

/// A registry with every built-in command plus this crate's commands, without
/// plugin hosting.
#[must_use]
pub fn full_registry() -> Registry {
    full_registry_with(None)
}

/// A registry with every built-in command plus this crate's commands, running
/// plugins from `host`.
#[must_use]
pub fn full_registry_with(host: Option<PluginHost>) -> Registry {
    let mut registry = Registry::with_builtin();
    // Registering into a fresh registry cannot collide.
    let _ = register_with(&mut registry, host);
    registry
}

#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod plugin_tests;
#[cfg(test)]
mod tests;
