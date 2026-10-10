//! Sample loading and writing. Runs on the main thread only.
//!
//! - WAV files are decoded with `hound`.
//! - `builtin:<name>` sounds are synthesized from scratch at load time, so the
//!   project ships no third-party audio.

mod builtin;
mod cache;
mod flac;
mod wav;

pub use builtin::builtin;
pub use cache::SampleCache;
pub use flac::{FLAC_DEPTHS, encode_flac, write_flac};
pub use wav::{WavFormat, load_wav, write_wav};

use std::error::Error;
use std::fmt;

/// Why a sample could not be loaded or written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleError {
    /// The file could not be opened, read or written.
    Io(String),
    /// The file is not a supported audio format.
    Format(String),
    /// There is no built-in sound with this name.
    UnknownBuiltin(String),
}

impl fmt::Display for SampleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "{msg}"),
            Self::Format(msg) => write!(f, "unsupported audio: {msg}"),
            Self::UnknownBuiltin(name) => write!(f, "unknown built-in sound `{name}`"),
        }
    }
}

impl Error for SampleError {}
