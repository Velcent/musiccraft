use std::collections::BTreeMap;

use musiccraft_project::Project;

use crate::{Command, CommandError, Invocation, Outcome, builtin};

/// The set of commands that can change a project.
#[derive(Default)]
pub struct Registry {
    commands: BTreeMap<&'static str, Box<dyn Command>>,
}

impl Registry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a registry holding every built-in command.
    #[must_use]
    pub fn with_builtin() -> Self {
        let mut registry = Self::new();
        for command in builtin::all() {
            registry.commands.insert(command.name(), command);
        }
        registry
    }

    /// Adds a command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::Duplicate`] if the name is already registered.
    pub fn register(&mut self, command: Box<dyn Command>) -> Result<(), CommandError> {
        let name = command.name();
        if self.commands.contains_key(name) {
            return Err(CommandError::Duplicate(name.to_owned()));
        }
        self.commands.insert(name, command);
        Ok(())
    }

    /// Looks a command up by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&dyn Command> {
        self.commands.get(name).map(|command| &**command)
    }

    /// Iterates over commands in name order.
    pub fn iter(&self) -> impl Iterator<Item = &dyn Command> {
        self.commands.values().map(|command| &**command)
    }

    /// Runs an invocation against a project.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::Unknown`] for unregistered names, otherwise any
    /// error from the command itself.
    pub fn execute(
        &self,
        project: &mut Project,
        invocation: &Invocation,
    ) -> Result<Outcome, CommandError> {
        let command = self
            .get(&invocation.name)
            .ok_or_else(|| CommandError::Unknown(invocation.name.clone()))?;
        command.execute(project, &invocation.args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_registry_lists_commands_in_order() {
        let registry = Registry::with_builtin();
        let names: Vec<_> = registry.iter().map(Command::name).collect();
        assert_eq!(
            names,
            [
                "project.set_tempo",
                "transport.play",
                "transport.stop",
                "transport.toggle"
            ]
        );
    }

    #[test]
    fn unknown_command_is_reported() {
        let registry = Registry::with_builtin();
        let mut project = Project::default();
        let err = registry
            .execute(&mut project, &Invocation::bare("nope.nothing"))
            .unwrap_err();
        assert_eq!(err, CommandError::Unknown("nope.nothing".to_owned()));
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = Registry::with_builtin();
        let duplicate = builtin::all().remove(0);
        assert!(matches!(
            registry.register(duplicate),
            Err(CommandError::Duplicate(_))
        ));
    }
}
