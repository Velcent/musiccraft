use musiccraft_project::Project;

use crate::{CommandError, Invocation, Outcome, Registry};

/// Reserved command name handled by [`Session`]: undo the last undoable command.
pub const UNDO: &str = "edit.undo";
/// Reserved command name handled by [`Session`]: redo the last undone command.
pub const REDO: &str = "edit.redo";

#[derive(Debug, Clone)]
struct HistoryEntry {
    command: Invocation,
    inverse: Invocation,
}

/// A project together with its command registry and undo/redo history.
pub struct Session {
    project: Project,
    registry: Registry,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
}

impl Session {
    /// Creates a session with empty history.
    #[must_use]
    pub fn new(project: Project, registry: Registry) -> Self {
        Self {
            project,
            registry,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    /// The current project state.
    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    /// The registry in use.
    #[must_use]
    pub fn registry(&self) -> &Registry {
        &self.registry
    }

    /// Number of steps that can be undone.
    #[must_use]
    pub fn undo_depth(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of steps that can be redone.
    #[must_use]
    pub fn redo_depth(&self) -> usize {
        self.redo_stack.len()
    }

    /// Runs an invocation, recording undo history when the command is undoable.
    /// The reserved names [`UNDO`] and [`REDO`] operate on the history.
    ///
    /// # Errors
    ///
    /// Returns any [`CommandError`] from the command; on error history is unchanged.
    pub fn run(&mut self, invocation: &Invocation) -> Result<Outcome, CommandError> {
        match invocation.name.as_str() {
            UNDO => self.undo(),
            REDO => self.redo(),
            _ => {
                let outcome = self.registry.execute(&mut self.project, invocation)?;
                if let Some(inverse) = &outcome.inverse {
                    self.undo_stack.push(HistoryEntry {
                        command: invocation.clone(),
                        inverse: inverse.clone(),
                    });
                    self.redo_stack.clear();
                }
                Ok(outcome)
            }
        }
    }

    /// Undoes the most recent undoable command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::NothingToUndo`] if history is empty.
    pub fn undo(&mut self) -> Result<Outcome, CommandError> {
        let entry = self.undo_stack.pop().ok_or(CommandError::NothingToUndo)?;
        if let Err(err) = self.registry.execute(&mut self.project, &entry.inverse) {
            self.undo_stack.push(entry);
            return Err(err);
        }
        let message = format!("undid `{}`", entry.command);
        self.redo_stack.push(entry);
        Ok(Outcome::done(message))
    }

    /// Redoes the most recently undone command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::NothingToRedo`] if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Outcome, CommandError> {
        let entry = self.redo_stack.pop().ok_or(CommandError::NothingToRedo)?;
        if let Err(err) = self.registry.execute(&mut self.project, &entry.command) {
            self.redo_stack.push(entry);
            return Err(err);
        }
        let message = format!("redid `{}`", entry.command);
        self.undo_stack.push(entry);
        Ok(Outcome::done(message))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session::new(Project::default(), Registry::with_builtin())
    }

    fn run(session: &mut Session, line: &str) -> Result<Outcome, CommandError> {
        session.run(&Invocation::parse(line).unwrap())
    }

    fn tempo(session: &Session) -> f64 {
        session.project().tempo_bpm()
    }

    #[test]
    fn undo_and_redo_walk_tempo_history() {
        let mut s = session();
        run(&mut s, "project.set_tempo 140").unwrap();
        run(&mut s, "project.set_tempo 150").unwrap();
        assert_eq!(s.undo_depth(), 2);

        run(&mut s, "edit.undo").unwrap();
        assert!((tempo(&s) - 140.0).abs() < f64::EPSILON);
        run(&mut s, "edit.undo").unwrap();
        assert!((tempo(&s) - 130.0).abs() < f64::EPSILON);
        assert_eq!(run(&mut s, "edit.undo"), Err(CommandError::NothingToUndo));

        run(&mut s, "edit.redo").unwrap();
        assert!((tempo(&s) - 140.0).abs() < f64::EPSILON);
        assert_eq!(s.redo_depth(), 1);
    }

    #[test]
    fn new_command_clears_redo_history() {
        let mut s = session();
        run(&mut s, "project.set_tempo 140").unwrap();
        run(&mut s, "edit.undo").unwrap();
        assert_eq!(s.redo_depth(), 1);
        run(&mut s, "project.set_tempo 100").unwrap();
        assert_eq!(s.redo_depth(), 0);
        assert_eq!(run(&mut s, "edit.redo"), Err(CommandError::NothingToRedo));
    }

    #[test]
    fn transport_commands_are_not_in_history() {
        let mut s = session();
        run(&mut s, "transport.play").unwrap();
        assert_eq!(s.undo_depth(), 0);
        assert!(s.project().transport().is_playing());
    }

    #[test]
    fn failed_command_leaves_history_untouched() {
        let mut s = session();
        run(&mut s, "project.set_tempo 140").unwrap();
        assert!(run(&mut s, "project.set_tempo 5000").is_err());
        assert_eq!(s.undo_depth(), 1);
        assert!((tempo(&s) - 140.0).abs() < f64::EPSILON);
    }
}
