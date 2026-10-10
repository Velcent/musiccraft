use musiccraft_project::Project;

use crate::{CommandError, Invocation, args::Args};

/// The result of a successful command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// Human-readable description of what happened.
    pub message: String,
    /// An invocation that reverses this command, or `None` if the command is not
    /// part of undo history.
    pub inverse: Option<Invocation>,
    /// The project was replaced wholesale (new/load); undo history must be cleared.
    pub reset_history: bool,
}

impl Outcome {
    /// A successful outcome that is not recorded in undo history.
    pub fn done(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            inverse: None,
            reset_history: false,
        }
    }

    /// A successful outcome that can be undone by running `inverse`.
    pub fn undoable(message: impl Into<String>, inverse: Invocation) -> Self {
        Self {
            message: message.into(),
            inverse: Some(inverse),
            reset_history: false,
        }
    }

    /// A successful outcome after which undo history no longer applies.
    pub fn reset(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            inverse: None,
            reset_history: true,
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

/// Signature of a table-driven command body.
pub(crate) type RunFn = fn(&mut Project, &Args<'_>) -> Result<Outcome, CommandError>;

/// A command defined by a name, help text and a function.
pub(crate) struct FnCommand {
    pub name: &'static str,
    pub summary: &'static str,
    pub usage: &'static str,
    pub run: RunFn,
}

impl Command for FnCommand {
    fn name(&self) -> &'static str {
        self.name
    }

    fn summary(&self) -> &'static str {
        self.summary
    }

    fn usage(&self) -> &'static str {
        self.usage
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        (self.run)(project, &Args::new(self.name, self.usage, args))
    }
}

/// Builds a boxed [`FnCommand`].
pub(crate) fn cmd(
    name: &'static str,
    usage: &'static str,
    summary: &'static str,
    run: RunFn,
) -> Box<dyn Command> {
    Box::new(FnCommand {
        name,
        summary,
        usage,
        run,
    })
}
