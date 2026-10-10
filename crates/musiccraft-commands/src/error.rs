use std::error::Error;
use std::fmt;

use musiccraft_project::ProjectError;

/// Why a command could not run.
#[derive(Debug, Clone, PartialEq)]
pub enum CommandError {
    /// The input line contained no command name.
    EmptyLine,
    /// The input line could not be split into arguments.
    Parse(String),
    /// No command with this name is registered.
    Unknown(String),
    /// The wrong number of arguments was supplied.
    Usage {
        /// Command that was invoked.
        command: String,
        /// Expected usage string.
        usage: String,
    },
    /// An argument could not be understood.
    InvalidArgument {
        /// Command that was invoked.
        command: String,
        /// What was wrong.
        reason: String,
    },
    /// The project model rejected the change.
    Project(ProjectError),
    /// Reading or writing a file failed.
    Io(String),
    /// A command with this name is already registered.
    Duplicate(String),
    /// The undo stack is empty.
    NothingToUndo,
    /// The redo stack is empty.
    NothingToRedo,
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyLine => write!(f, "empty command"),
            Self::Parse(reason) => write!(f, "cannot parse command: {reason}"),
            Self::Unknown(name) => write!(f, "unknown command `{name}`"),
            Self::Usage { command, usage } => {
                write!(f, "wrong arguments for `{command}`; usage: {usage}")
            }
            Self::InvalidArgument { command, reason } => {
                write!(f, "invalid argument for `{command}`: {reason}")
            }
            Self::Project(err) => write!(f, "{err}"),
            Self::Io(reason) => f.write_str(reason),
            Self::Duplicate(name) => write!(f, "command `{name}` is already registered"),
            Self::NothingToUndo => write!(f, "nothing to undo"),
            Self::NothingToRedo => write!(f, "nothing to redo"),
        }
    }
}

impl Error for CommandError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Project(err) => Some(err),
            _ => None,
        }
    }
}

impl From<ProjectError> for CommandError {
    fn from(err: ProjectError) -> Self {
        Self::Project(err)
    }
}
