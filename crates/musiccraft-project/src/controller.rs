//! Controllers: parameters moved continuously by a live source (see
//! `docs/spec/controllers.md`).

use serde::{Deserialize, Serialize};

use crate::{AutomationTarget, ChannelId};

/// A controller's source and settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControllerKind {
    /// A tempo-synced low-frequency oscillator.
    Lfo {
        /// 0 sine, 1 triangle, 2 saw, 3 square, 4 random steps.
        shape: u8,
        /// Beats per cycle.
        period: f32,
        /// Start of the cycle, 0..1.
        phase: f32,
    },
    /// An envelope started by a channel's notes.
    Envelope {
        /// The channel whose notes trigger it.
        channel: ChannelId,
        /// Seconds.
        attack: f32,
        /// Seconds.
        decay: f32,
        /// Level, 0..1.
        sustain: f32,
        /// Seconds.
        release: f32,
    },
    /// The key a channel holds.
    Keyboard {
        /// The channel.
        channel: ChannelId,
        /// Key that maps to 0.
        low: u8,
        /// Key that maps to 1.
        high: u8,
    },
    /// The level of a mixer track.
    Peak {
        /// Track index (0 = master).
        track: usize,
        /// Multiplier on the level.
        gain: f32,
        /// Milliseconds.
        attack: f32,
        /// Milliseconds.
        release: f32,
    },
    /// An expression of the song position.
    Formula {
        /// The expression text.
        expr: String,
    },
    /// A knob of its own.
    Macro {
        /// 0..1.
        value: f32,
    },
}

impl ControllerKind {
    /// Names accepted by `controller.add`.
    pub const NAMES: [&'static str; 6] =
        ["lfo", "envelope", "keyboard", "peak", "formula", "macro"];

    /// The kind's name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::Lfo { .. } => "lfo",
            Self::Envelope { .. } => "envelope",
            Self::Keyboard { .. } => "keyboard",
            Self::Peak { .. } => "peak",
            Self::Formula { .. } => "formula",
            Self::Macro { .. } => "macro",
        }
    }
}

/// A parameter moved by a live source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Controller {
    /// Display name.
    pub name: String,
    /// The source and its settings.
    #[serde(flatten)]
    pub kind: ControllerKind,
    /// The parameter driven.
    pub target: AutomationTarget,
    /// Control position (0..1 of the target's range) for an output of 0.
    pub min: f32,
    /// Control position for an output of 1.
    pub max: f32,
    /// Whether it runs.
    pub enabled: bool,
}
