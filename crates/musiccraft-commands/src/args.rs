//! Argument parsing shared by the built-in commands.

use std::str::FromStr;

use musiccraft_project::Project;
use musiccraft_project::music::parse_key;

use crate::CommandError;

/// Positional arguments of one invocation, with helpers that produce consistent
/// errors.
pub(crate) struct Args<'a> {
    name: &'static str,
    usage: &'static str,
    items: &'a [String],
    /// Trailing `@<pattern>` selector, if one was given.
    pattern: Option<&'a str>,
}

impl<'a> Args<'a> {
    pub fn new(name: &'static str, usage: &'static str, items: &'a [String]) -> Self {
        let (items, pattern) = match items.split_last() {
            Some((last, rest)) if last.starts_with('@') && usage.contains("[@pattern]") => {
                (rest, Some(&last[1..]))
            }
            _ => (items, None),
        };
        Self {
            name,
            usage,
            items,
            pattern,
        }
    }

    /// Name of the command being run.
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// A usage error for this command.
    pub fn usage_error(&self) -> CommandError {
        CommandError::Usage {
            command: self.name.to_owned(),
            usage: self.usage.to_owned(),
        }
    }

    /// An invalid-argument error for this command.
    pub fn invalid(&self, reason: impl Into<String>) -> CommandError {
        CommandError::InvalidArgument {
            command: self.name.to_owned(),
            reason: reason.into(),
        }
    }

    /// Requires between `min` and `max` arguments (inclusive).
    pub fn expect(&self, min: usize, max: usize) -> Result<(), CommandError> {
        if (min..=max).contains(&self.items.len()) {
            Ok(())
        } else {
            Err(self.usage_error())
        }
    }

    /// Number of arguments.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Argument `i`, which must exist.
    pub fn str(&self, i: usize) -> Result<&'a str, CommandError> {
        self.items
            .get(i)
            .map(String::as_str)
            .ok_or_else(|| self.usage_error())
    }

    /// Argument `i` if present.
    pub fn opt(&self, i: usize) -> Option<&'a str> {
        self.items.get(i).map(String::as_str)
    }

    /// Arguments from `i` on, joined by spaces (for names typed without quotes).
    pub fn rest(&self, i: usize) -> Result<String, CommandError> {
        let rest = self.items.get(i..).unwrap_or_default();
        if rest.is_empty() {
            return Err(self.usage_error());
        }
        let text = rest.join(" ");
        if text.trim().is_empty() {
            return Err(self.invalid("name must not be empty"));
        }
        Ok(text)
    }

    /// Parses argument `i`.
    pub fn parse<T: FromStr>(&self, i: usize, what: &str) -> Result<T, CommandError> {
        let raw = self.str(i)?;
        raw.parse()
            .map_err(|_| self.invalid(format!("`{raw}` is not a valid {what}")))
    }

    /// Parses a finite float.
    pub fn f32(&self, i: usize, what: &str) -> Result<f32, CommandError> {
        let value: f32 = self.parse(i, what)?;
        if value.is_finite() {
            Ok(value)
        } else {
            Err(self.invalid(format!("{what} must be a finite number")))
        }
    }

    /// Parses a 1-based number.
    pub fn number(&self, i: usize, what: &str) -> Result<usize, CommandError> {
        let n: usize = self.parse(i, what)?;
        if n == 0 {
            return Err(self.invalid(format!("{what} numbers start at 1")));
        }
        Ok(n)
    }

    /// Parses a MIDI key as a number or note name.
    pub fn key(&self, i: usize) -> Result<u8, CommandError> {
        let raw = self.str(i)?;
        parse_key(raw)
            .ok_or_else(|| self.invalid(format!("`{raw}` is not a key (0-127 or e.g. C5)")))
    }

    /// Parses a velocity (1..=127), defaulting when absent.
    pub fn velocity(&self, i: usize, default: u8) -> Result<u8, CommandError> {
        match self.opt(i) {
            None => Ok(default),
            Some(_) => {
                let v: u8 = self.parse(i, "velocity")?;
                if (1..=127).contains(&v) {
                    Ok(v)
                } else {
                    Err(self.invalid("velocity must be 1 to 127"))
                }
            }
        }
    }

    /// Parses `on`/`off` (also `true`/`false`, `1`/`0`).
    pub fn flag(&self, i: usize) -> Result<bool, CommandError> {
        match self.str(i)? {
            "on" | "true" | "1" | "yes" => Ok(true),
            "off" | "false" | "0" | "no" => Ok(false),
            other => Err(self.invalid(format!("`{other}` is not on or off"))),
        }
    }

    /// Parses a time: `<n>t` is exact ticks, a plain number is in `unit_ticks`
    /// units (fractions allowed). Must be non-negative.
    pub fn time(&self, i: usize, unit_ticks: u32) -> Result<u32, CommandError> {
        let raw = self.str(i)?;
        let ticks = if let Some(ticks) = raw.strip_suffix('t') {
            ticks.parse::<u32>().ok()
        } else {
            raw.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v >= 0.0)
                .map(|v| (v * f64::from(unit_ticks)).round())
                .filter(|t| *t <= f64::from(u32::MAX / 2))
                .map(|t| t as u32)
        };
        ticks.ok_or_else(|| self.invalid(format!("`{raw}` is not a time (e.g. 4, 0.5 or 96t)")))
    }

    /// Parses a duration like [`Args::time`] that must be at least one tick.
    pub fn duration(&self, i: usize, unit_ticks: u32) -> Result<u32, CommandError> {
        let ticks = self.time(i, unit_ticks)?;
        if ticks == 0 {
            Err(self.invalid("length must be greater than zero"))
        } else {
            Ok(ticks)
        }
    }

    /// Index of the pattern selected with a trailing `@<n>`, else the current one.
    pub fn pattern_index(&self, project: &Project) -> Result<usize, CommandError> {
        match self.pattern {
            None => Ok(project
                .current_pattern
                .min(project.patterns.len().saturating_sub(1))),
            Some(raw) => {
                let n: usize = raw
                    .parse()
                    .map_err(|_| self.invalid(format!("`@{raw}` is not a pattern number")))?;
                Ok(project.pattern_index(n)?)
            }
        }
    }
}

/// Formats a tick count in the exact `<n>t` form used by inverse invocations.
pub(crate) fn ticks(value: u32) -> String {
    format!("{value}t")
}

/// Formats a float for an inverse invocation (shortest exact form).
pub(crate) fn num(value: f32) -> String {
    value.to_string()
}

/// Formats an on/off flag.
pub(crate) fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

/// Formats a pattern selector for an inverse invocation.
pub(crate) fn at_pattern(index: usize) -> String {
    format!("@{}", index + 1)
}
