//! Built-in commands, grouped by area.

mod automation;
mod channel;
mod mixer;
mod notes;
mod pattern;
mod playlist;
mod project;
mod transport;

#[cfg(test)]
mod tests;

use crate::Command;

/// All commands registered by default.
pub(crate) fn all() -> Vec<Box<dyn Command>> {
    let mut commands = Vec::new();
    commands.extend(transport::commands());
    commands.extend(project::commands());
    commands.extend(channel::commands());
    commands.extend(notes::commands());
    commands.extend(pattern::commands());
    commands.extend(mixer::commands());
    commands.extend(playlist::commands());
    commands.extend(automation::commands());
    commands
}
