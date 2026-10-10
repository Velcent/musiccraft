//! The command registry: the only way to change a MusicCraft project.
//!
//! UI actions, keyboard shortcuts, the CLI and the MCP server all become an
//! [`Invocation`] that a [`Session`] runs against a [`Registry`] of [`Command`]s.
//!
//! Conventions shared by every built-in command:
//! - Channels, patterns, playlist tracks and effect slots are numbered from 1.
//!   Mixer track 0 is the master; inserts are numbered from 1.
//! - Note and step times are in steps (1/16 notes) and playlist times in bars;
//!   fractions are allowed, and `<n>t` gives an exact tick count.
//! - Note and step commands act on the selected pattern unless a trailing
//!   `@<pattern>` argument names another. Inverses always name the pattern.

mod args;
mod cmds;
mod command;
mod error;
mod invocation;
mod registry;
mod session;

pub use command::{Command, Outcome};
pub use error::CommandError;
pub use invocation::Invocation;
pub use registry::Registry;
pub use session::{BATCH, REDO, SESSION_COMMANDS, Session, UNDO};
