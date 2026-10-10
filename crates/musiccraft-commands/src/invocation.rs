use std::fmt;

use crate::CommandError;

/// A command name plus string arguments. Shortcuts, menu items, CLI lines and MCP
/// tool calls all become invocations.
///
/// The text form separates arguments with whitespace. Arguments containing
/// whitespace, quotes, backslashes or a leading `#` are written in double quotes
/// with `\"` and `\\` escapes. Outside quotes, a token starting with `#` begins a
/// comment that runs to the end of the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// Registered command name, such as `transport.play`.
    pub name: String,
    /// Positional arguments.
    pub args: Vec<String>,
}

fn tokenize(line: &str) -> Result<Vec<String>, CommandError> {
    let mut tokens = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        while chars.next_if(|c| c.is_whitespace()).is_some() {}
        let Some(&first) = chars.peek() else {
            break;
        };
        if first == '#' {
            break;
        }
        let mut token = String::new();
        if first == '"' {
            chars.next();
            loop {
                match chars.next() {
                    Some('"') => break,
                    Some('\\') => match chars.next() {
                        Some(c) => token.push(c),
                        None => return Err(CommandError::Parse("dangling `\\`".into())),
                    },
                    Some(c) => token.push(c),
                    None => return Err(CommandError::Parse("unterminated quote".into())),
                }
            }
            if chars.peek().is_some_and(|c| !c.is_whitespace()) {
                return Err(CommandError::Parse(
                    "a closing quote must be followed by whitespace".into(),
                ));
            }
        } else {
            while let Some(c) = chars.next_if(|c| !c.is_whitespace()) {
                token.push(c);
            }
        }
        tokens.push(token);
    }
    Ok(tokens)
}

fn needs_quotes(arg: &str) -> bool {
    arg.is_empty()
        || arg.starts_with('#')
        || arg
            .chars()
            .any(|c| c.is_whitespace() || c == '"' || c == '\\')
}

/// Writes one argument, quoting it if needed.
fn write_arg(f: &mut fmt::Formatter<'_>, arg: &str) -> fmt::Result {
    if !needs_quotes(arg) {
        return f.write_str(arg);
    }
    f.write_str("\"")?;
    for c in arg.chars() {
        if c == '"' || c == '\\' {
            f.write_str("\\")?;
        }
        write!(f, "{c}")?;
    }
    f.write_str("\"")
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

    /// Parses a line such as `project.set_tempo 140` or
    /// `channel.add_sampler "my kick.wav" Kick # comment`.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::EmptyLine`] if the line has no tokens (blank or
    /// comment only) and [`CommandError::Parse`] for malformed quoting.
    pub fn parse(line: &str) -> Result<Self, CommandError> {
        let mut tokens = tokenize(line)?.into_iter();
        let name = tokens.next().ok_or(CommandError::EmptyLine)?;
        Ok(Self {
            name,
            args: tokens.collect(),
        })
    }
}

impl fmt::Display for Invocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.name)?;
        for arg in &self.args {
            f.write_str(" ")?;
            write_arg(f, arg)?;
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
        assert_eq!(Invocation::parse("  # note"), Err(CommandError::EmptyLine));
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

    #[test]
    fn quotes_and_comments() {
        let inv = Invocation::parse(r#"a.b "two words" "say \"hi\"" x#y # tail"#).unwrap();
        assert_eq!(inv.args, ["two words", "say \"hi\"", "x#y"]);
        assert!(Invocation::parse(r#"a.b "open"#).is_err());
        assert!(Invocation::parse(r#"a.b "x"y"#).is_err());
    }

    #[test]
    fn awkward_arguments_round_trip() {
        let inv = Invocation::new(
            "channel.insert",
            ["", "#hash", "back\\slash", "{\"k\": [1, 2]}", "tab\there"],
        );
        assert_eq!(Invocation::parse(&inv.to_string()).unwrap(), inv);
    }
}
