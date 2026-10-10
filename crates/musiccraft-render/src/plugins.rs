//! The safe boundary between MusicCraft and a plugin host.
//!
//! A [`PluginFactory`] (the CLAP host) creates [`LivePlugin`]s: a main-thread
//! [`PluginInstance`] and an audio-thread [`External`] processor for the same
//! plugin. The [`crate::Bridge`] owns the instances and installs the
//! processors in engine slots. Nothing here uses `unsafe`.

use std::fmt;
use std::path::PathBuf;
use std::sync::Arc;

use musiccraft_commands::{Command, CommandError, Outcome};
use musiccraft_engine::External;
use musiccraft_project::Project;

/// What an installed plugin is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PluginKind {
    /// Makes sound from notes.
    Instrument,
    /// Processes audio.
    Effect,
}

impl fmt::Display for PluginKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Instrument => "instrument",
            Self::Effect => "effect",
        })
    }
}

/// An installed plugin found by scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    /// Plugin id, unique across plugins.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Maker.
    pub vendor: String,
    /// Version string.
    pub version: String,
    /// Instrument or effect.
    pub kind: PluginKind,
    /// File the plugin comes from.
    pub path: PathBuf,
}

/// One parameter of a live plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct PluginParam {
    /// Parameter id in the plugin's numbering.
    pub id: u32,
    /// Display name.
    pub name: String,
    /// Group path (for example `Filter/Env`), possibly empty.
    pub module: String,
    /// Lowest value.
    pub min: f64,
    /// Highest value.
    pub max: f64,
    /// Default value.
    pub default: f64,
    /// Whether only whole numbers are valid.
    pub stepped: bool,
    /// Whether the plugin asks hosts not to show it.
    pub hidden: bool,
    /// Whether the host may not change it.
    pub read_only: bool,
}

/// Something that happened in a live plugin since the last poll.
#[derive(Debug, Clone, PartialEq)]
pub enum PluginEvent {
    /// The plugin changed a parameter itself (usually from its own window).
    Param {
        /// Parameter id.
        id: u32,
        /// New value.
        value: f64,
    },
    /// The plugin's state changed in a way parameters do not capture.
    StateChanged,
    /// The plugin's own window was closed by the user.
    EditorClosed,
    /// The plugin's parameter list changed.
    ParamsChanged,
}

/// The main-thread side of a live plugin.
pub trait PluginInstance: fmt::Debug {
    /// The plugin's parameters.
    fn params(&self) -> Vec<PluginParam>;

    /// The current value of a parameter.
    fn param_value(&self, id: u32) -> Option<f64>;

    /// A value formatted by the plugin (for example `-6.0 dB`).
    fn format_param(&self, id: u32, value: f64) -> Option<String>;

    /// The plugin's opaque state, if it supports saving.
    fn save_state(&mut self) -> Option<Vec<u8>>;

    /// Runs main-thread housekeeping (timers, requested callbacks) and returns
    /// what happened since the last call. Call often, for example every frame.
    fn poll(&mut self) -> Vec<PluginEvent>;

    /// Whether the plugin has its own window.
    fn has_editor(&self) -> bool;

    /// Whether its window is open.
    fn editor_open(&self) -> bool;

    /// Opens or closes its window.
    ///
    /// # Errors
    ///
    /// Returns a message if the window cannot be opened.
    fn set_editor_open(&mut self, open: bool, title: &str) -> Result<(), String>;
}

/// A plugin ready to play: the main-thread instance and the audio processor.
#[derive(Debug)]
pub struct LivePlugin {
    /// Main-thread side.
    pub instance: Box<dyn PluginInstance>,
    /// Audio-thread side, to install in an engine slot.
    pub processor: Box<dyn External>,
}

/// What to create.
#[derive(Debug, Clone, Copy)]
pub struct PluginRequest<'a> {
    /// Plugin id.
    pub id: &'a str,
    /// Engine sample rate.
    pub sample_rate: u32,
    /// Largest block the engine processes.
    pub max_frames: u32,
    /// Saved state to restore.
    pub state: Option<&'a [u8]>,
}

/// Finds and creates plugins.
pub trait PluginFactory: Send + Sync {
    /// Installed plugins, as of the last scan.
    fn catalog(&self) -> Vec<PluginInfo>;

    /// Scans the plugin folders again.
    fn rescan(&self);

    /// Creates, restores and activates a plugin. Stored parameter values are
    /// applied afterwards by the bridge.
    ///
    /// # Errors
    ///
    /// Returns a message if the plugin is not installed or fails to start.
    fn instantiate(&self, request: &PluginRequest<'_>) -> Result<LivePlugin, String>;
}

/// A shared plugin factory.
pub type PluginHost = Arc<dyn PluginFactory>;

/// `plugin.list [instrument|effect]`: lists installed plugins.
#[derive(Default)]
pub struct PluginListCommand {
    /// Where plugins come from.
    pub host: Option<PluginHost>,
}

impl Command for PluginListCommand {
    fn name(&self) -> &'static str {
        "plugin.list"
    }

    fn summary(&self) -> &'static str {
        "List installed plugins (id, name, kind, file)."
    }

    fn usage(&self) -> &'static str {
        "plugin.list [instrument|effect]"
    }

    fn execute(&self, _project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let kind = match args {
            [] => None,
            [k] if k == "instrument" => Some(PluginKind::Instrument),
            [k] if k == "effect" => Some(PluginKind::Effect),
            _ => {
                return Err(CommandError::Usage {
                    command: self.name().to_owned(),
                    usage: self.usage().to_owned(),
                });
            }
        };
        let Some(host) = &self.host else {
            return Ok(Outcome::done("plugin hosting is not available here"));
        };
        let plugins: Vec<String> = host
            .catalog()
            .into_iter()
            .filter(|p| kind.is_none_or(|k| p.kind == k))
            .map(|p| format!("{}\t{}\t{}\t{}", p.id, p.name, p.kind, p.path.display()))
            .collect();
        if plugins.is_empty() {
            return Ok(Outcome::done("no plugins found"));
        }
        Ok(Outcome::done(plugins.join("\n")))
    }
}
