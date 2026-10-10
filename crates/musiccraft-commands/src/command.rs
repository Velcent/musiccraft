use musiccraft_project::Project;

use crate::{CommandError, Invocation};

/// The result of a successful command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Human-readable description of what happened.
    pub message: String,
    /// An invocation that reverses this command, or `None` if the command is not
    /// part of undo history.
    pub inverse: Option<Invocation>,
}

impl Outcome {
    /// A successful outcome that is not recorded in undo history.
    pub fn done(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            inverse: None,
        }
    }

    /// A successful outcome that can be undone by running `inverse`.
    pub fn undoable(message: impl Into<String>, inverse: Invocation) -> Self {
        Self {
            message: message.into(),
            inverse: Some(inverse),
        }
    }
}

/// A named action that changes a [`Project`].
pub trait Command: Send + Sync {
    /// Unique name in `area.action` form.
    fn name(&self) -> &'static str;

    /// One-line description.
    fn summary(&self) -> &'static str;

    /// Usage string, starting with the command name.
    fn usage(&self) -> &'static str;

    /// Runs the command.
    ///
    /// # Errors
    ///
    /// Returns a [`CommandError`] if the arguments are wrong or the project rejects
    /// the change. On error the project must be left unchanged.
    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError>;
}
