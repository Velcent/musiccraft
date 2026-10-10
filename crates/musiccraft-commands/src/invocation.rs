use std::fmt;

use crate::CommandError;

/// A command name plus string arguments. Shortcuts, menu items, CLI lines and MCP
/// tool calls all become invocations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Registered command name, such as `transport.play`.
    pub name: String,
    /// Positional arguments.
    pub args: Vec<String>,
}

impl Invocation {
    /// Builds an invocation from a name and arguments.
    pub fn new<S: Into<String>>(
        name: impl Into<String>,
        args: impl IntoIterator<Item = S>,
    ) -> Self {
        Self {
            name: name.into(),
            args: args.into_iter().map(Into::into).collect(),
        }
    }

    /// Builds an invocation with no arguments.
    pub fn bare(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            args: Vec::new(),
        }
    }

    /// Parses a whitespace-separated line such as `project.set_tempo 140`.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::EmptyLine`] if the line has no tokens.
    pub fn parse(line: &str) -> Result<Self, CommandError> {
        let mut parts = line.split_whitespace();
        let name = parts.next().ok_or(CommandError::EmptyLine)?;
        Ok(Self {
            name: name.to_owned(),
            args: parts.map(str::to_owned).collect(),
        })
    }
}

impl fmt::Display for Invocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        for arg in &self.args {
            write!(f, " {arg}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_name_and_arguments() {
        let inv = Invocation::parse("  project.set_tempo   140 ").unwrap();
        assert_eq!(inv, Invocation::new("project.set_tempo", ["140"]));
    }

    #[test]
    fn empty_line_is_an_error() {
        assert_eq!(Invocation::parse("   "), Err(CommandError::EmptyLine));
    }

    #[test]
    fn display_round_trips() {
        let inv = Invocation::new("project.set_tempo", ["140"]);
        assert_eq!(Invocation::parse(&inv.to_string()).unwrap(), inv);
        assert_eq!(
            Invocation::bare("transport.play").to_string(),
            "transport.play"
        );
    }
}
