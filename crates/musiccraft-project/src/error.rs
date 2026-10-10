use std::error::Error;
use std::fmt;

use crate::{MAX_TEMPO_BPM, MIN_TEMPO_BPM};

/// Errors produced when a change is rejected by the project model.
#[derive(Debug, Clone, PartialEq)]
pub enum ProjectError {
    /// The requested tempo is not finite or is outside the supported range.
    TempoOutOfRange(f64),
    /// A numeric value is outside its allowed range.
    OutOfRange {
        /// What the value is (for example "volume").
        what: String,
        /// The rejected value.
        value: f64,
        /// Lowest allowed value.
        min: f64,
        /// Highest allowed value.
        max: f64,
    },
    /// A numbered item does not exist.
    NotFound {
        /// What was looked up (for example "channel").
        what: &'static str,
        /// The number the user gave.
        number: usize,
    },
    /// Any other rejected change.
    Invalid(String),
    /// A project file could not be read.
    Format(String),
}

impl ProjectError {
    /// Builds an [`ProjectError::OutOfRange`].
    pub fn out_of_range(what: impl Into<String>, value: f64, min: f64, max: f64) -> Self {
        Self::OutOfRange {
            what: what.into(),
            value,
            min,
            max,
        }
    }
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TempoOutOfRange(bpm) => write!(
                f,
                "tempo {bpm} is outside the supported range {MIN_TEMPO_BPM}-{MAX_TEMPO_BPM} BPM"
            ),
            Self::OutOfRange {
                what,
                value,
                min,
                max,
            } => write!(f, "{what} {value} is outside the range {min} to {max}"),
            Self::NotFound { what, number } => write!(f, "there is no {what} {number}"),
            Self::Invalid(reason) => f.write_str(reason),
            Self::Format(reason) => write!(f, "cannot read project: {reason}"),
        }
    }
}

impl Error for ProjectError {}
