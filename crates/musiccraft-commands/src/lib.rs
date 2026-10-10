//! The command registry: the only way to change a MusicCraft project.
//!
//! UI actions, keyboard shortcuts, the CLI and the MCP server all become an
//! [`Invocation`] that a [`Session`] runs against a [`Registry`] of [`Command`]s.

mod builtin;
mod command;
mod error;
mod invocation;
mod registry;
mod session;

pub use command::{Command, Outcome};
pub use error::CommandError;
pub use invocation::Invocation;
pub use registry::Registry;
pub use session::{REDO, Session, UNDO};
