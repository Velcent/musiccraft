use musiccraft_project::Project;

use crate::{CommandError, Invocation, Outcome, Registry};

/// Reserved command name handled by [`Session`]: undo the last undoable command.
pub const UNDO: &str = "edit.undo";
/// Reserved command name handled by [`Session`]: redo the last undone command.
pub const REDO: &str = "edit.redo";
/// Reserved command name handled by [`Session`]: run each argument as a command,
/// all or nothing, as one undo step.
pub const BATCH: &str = "edit.batch";

/// Reserved names with their help text, for listings.
pub const SESSION_COMMANDS: [(&str, &str, &str); 3] = [
    (UNDO, "edit.undo", "Undo the last undoable command."),
    (REDO, "edit.redo", "Redo the last undone command."),
    (
        BATCH,
        "edit.batch <command> <command> ...",
        "Run several quoted commands as one undo step; all or nothing.",
    ),
];

#[derive(Debug, Clone)]
struct HistoryEntry {
    command: Invocation,
    inverse: Invocation,
    coalesce: Option<u64>,
    /// Identifier allocator position before the command first ran, so that a
    /// redo recreates the same identifiers that later history entries refer to.
    next_id: u32,
}

/// A project together with its command registry and undo/redo history.
pub struct Session {
    project: Project,
    registry: Registry,
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    revision: u64,
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
            revision: 0,
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

    /// Mutable access to the registry, for front ends that add commands.
    pub fn registry_mut(&mut self) -> &mut Registry {
        &mut self.registry
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

    /// The command that `edit.undo` would reverse.
    #[must_use]
    pub fn undo_label(&self) -> Option<&Invocation> {
        self.undo_stack.last().map(|e| &e.command)
    }

    /// The commands that `edit.undo` would reverse, newest first, and those
    /// `edit.redo` would re-run, next first.
    #[must_use]
    pub fn history(&self) -> (Vec<&Invocation>, Vec<&Invocation>) {
        let undo = self.undo_stack.iter().rev().map(|e| &e.command).collect();
        let redo = self.redo_stack.iter().rev().map(|e| &e.command).collect();
        (undo, redo)
    }

    /// Counter that increases whenever a command succeeds. Front ends use it to
    /// notice that the project may have changed.
    #[must_use]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Runs an invocation, recording undo history when the command is undoable.
    /// The reserved names [`UNDO`], [`REDO`] and [`BATCH`] are handled here.
    ///
    /// # Errors
    ///
    /// Returns any [`CommandError`] from the command; on error the project and
    /// history are unchanged.
    pub fn run(&mut self, invocation: &Invocation) -> Result<Outcome, CommandError> {
        self.run_inner(invocation, None)
    }

    /// Like [`Session::run`], but consecutive runs with the same `key` and the
    /// same command name merge into one undo step. Use one key per continuous
    /// gesture on one control (for example a fader drag).
    ///
    /// # Errors
    ///
    /// As [`Session::run`].
    pub fn run_coalesced(
        &mut self,
        invocation: &Invocation,
        key: u64,
    ) -> Result<Outcome, CommandError> {
        self.run_inner(invocation, Some(key))
    }

    /// Parses and runs one line.
    ///
    /// # Errors
    ///
    /// Returns parse errors and any error from [`Session::run`].
    pub fn run_line(&mut self, line: &str) -> Result<Outcome, CommandError> {
        self.run(&Invocation::parse(line)?)
    }

    fn run_inner(
        &mut self,
        invocation: &Invocation,
        coalesce: Option<u64>,
    ) -> Result<Outcome, CommandError> {
        let outcome = match invocation.name.as_str() {
            UNDO => self.undo()?,
            REDO => self.redo()?,
            _ => {
                let next_id = self.project.next_id();
                let outcome = self.execute(invocation)?;
                if outcome.reset_history {
                    self.undo_stack.clear();
                    self.redo_stack.clear();
                } else if let Some(inverse) = &outcome.inverse {
                    self.record(invocation, inverse, coalesce, next_id);
                }
                outcome
            }
        };
        self.revision += 1;
        Ok(outcome)
    }

    fn record(
        &mut self,
        invocation: &Invocation,
        inverse: &Invocation,
        coalesce: Option<u64>,
        next_id: u32,
    ) {
        self.redo_stack.clear();
        if let (Some(key), Some(top)) = (coalesce, self.undo_stack.last_mut()) {
            // The gesture key identifies the control being dragged, so the
            // command name is enough to tell the gesture continues.
            if top.coalesce == Some(key) && top.command.name == invocation.name {
                top.command = invocation.clone();
                return;
            }
        }
        self.undo_stack.push(HistoryEntry {
            command: invocation.clone(),
            inverse: inverse.clone(),
            coalesce,
            next_id,
        });
    }

    /// Executes without touching history. Handles nested batches.
    fn execute(&mut self, invocation: &Invocation) -> Result<Outcome, CommandError> {
        if invocation.name == BATCH {
            self.execute_batch(&invocation.args)
        } else {
            self.registry.execute(&mut self.project, invocation)
        }
    }

    fn execute_batch(&mut self, lines: &[String]) -> Result<Outcome, CommandError> {
        let mut inverses = Vec::new();
        let mut messages = Vec::new();
        for line in lines {
            let result = Invocation::parse(line).and_then(|inv| {
                if inv.name == UNDO || inv.name == REDO {
                    Err(CommandError::InvalidArgument {
                        command: BATCH.to_owned(),
                        reason: "undo and redo cannot be batched".to_owned(),
                    })
                } else {
                    self.execute(&inv)
                }
            });
            match result {
                Ok(outcome) if outcome.reset_history => {
                    return Err(CommandError::InvalidArgument {
                        command: BATCH.to_owned(),
                        reason: "commands that replace the project cannot be batched".to_owned(),
                    });
                }
                Ok(outcome) => {
                    messages.push(outcome.message);
                    inverses.extend(outcome.inverse);
                }
                Err(err) => {
                    // Roll back what already ran, newest first.
                    for inverse in inverses.iter().rev() {
                        let _ = self.execute(inverse);
                    }
                    return Err(err);
                }
            }
        }
        let message = messages.join("; ");
        if inverses.is_empty() {
            return Ok(Outcome::done(message));
        }
        let inverse = if inverses.len() == 1 {
            inverses.remove(0)
        } else {
            Invocation::new(BATCH, inverses.iter().rev().map(ToString::to_string))
        };
        Ok(Outcome::undoable(message, inverse))
    }

    /// Undoes the most recent undoable command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::NothingToUndo`] if history is empty.
    pub fn undo(&mut self) -> Result<Outcome, CommandError> {
        let entry = self.undo_stack.pop().ok_or(CommandError::NothingToUndo)?;
        if let Err(err) = self.execute(&entry.inverse) {
            self.undo_stack.push(entry);
            return Err(err);
        }
        let message = format!("undid `{}`", entry.command);
        self.redo_stack.push(HistoryEntry {
            coalesce: None,
            ..entry
        });
        Ok(Outcome::done(message))
    }

    /// Redoes the most recently undone command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandError::NothingToRedo`] if there is nothing to redo.
    pub fn redo(&mut self) -> Result<Outcome, CommandError> {
        let entry = self.redo_stack.pop().ok_or(CommandError::NothingToRedo)?;
        let current_next_id = self.project.next_id();
        self.project.set_next_id(entry.next_id);
        if let Err(err) = self.execute(&entry.command) {
            self.project.set_next_id(current_next_id);
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

    #[test]
    fn history_lists_undo_steps_newest_first_and_redo_steps_next_first() {
        let mut s = session();
        for line in [
            "project.set_tempo 100",
            "project.set_tempo 110",
            "project.set_tempo 120",
        ] {
            s.run_line(line).unwrap();
        }
        s.undo().unwrap();
        s.undo().unwrap();
        let (undo, redo) = s.history();
        let names = |v: Vec<&Invocation>| v.iter().map(ToString::to_string).collect::<Vec<_>>();
        assert_eq!(names(undo), ["project.set_tempo 100"]);
        assert_eq!(
            names(redo),
            ["project.set_tempo 110", "project.set_tempo 120"]
        );
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
        let revision = s.revision();
        assert!(run(&mut s, "project.set_tempo 5000").is_err());
        assert_eq!(s.undo_depth(), 1);
        assert_eq!(s.revision(), revision);
        assert!((tempo(&s) - 140.0).abs() < f64::EPSILON);
    }

    #[test]
    fn batch_is_one_undo_step() {
        let mut s = session();
        run(
            &mut s,
            r#"edit.batch "project.set_tempo 100" "project.rename Demo""#,
        )
        .unwrap();
        assert_eq!(s.undo_depth(), 1);
        assert_eq!(s.project().name(), "Demo");
        run(&mut s, "edit.undo").unwrap();
        assert_eq!(s.project().name(), "Untitled");
        assert!((tempo(&s) - 130.0).abs() < f64::EPSILON);
        run(&mut s, "edit.redo").unwrap();
        assert!((tempo(&s) - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn failing_batch_rolls_back() {
        let mut s = session();
        let err = run(
            &mut s,
            r#"edit.batch "project.set_tempo 100" "project.set_tempo 5000""#,
        );
        assert!(err.is_err());
        assert!((tempo(&s) - 130.0).abs() < f64::EPSILON);
        assert_eq!(s.undo_depth(), 0);
        assert!(run(&mut s, r#"edit.batch "edit.undo""#).is_err());
    }

    #[test]
    fn coalesced_runs_merge_into_one_step() {
        let mut s = session();
        for bpm in [131, 132, 133] {
            s.run_coalesced(&Invocation::new("project.set_tempo", [bpm.to_string()]), 7)
                .unwrap();
        }
        assert_eq!(s.undo_depth(), 1);
        s.run_coalesced(&Invocation::new("project.set_tempo", ["150"]), 8)
            .unwrap();
        assert_eq!(s.undo_depth(), 2);
        run(&mut s, "edit.undo").unwrap();
        run(&mut s, "edit.undo").unwrap();
        assert!((tempo(&s) - 130.0).abs() < f64::EPSILON);
        run(&mut s, "edit.redo").unwrap();
        assert!((tempo(&s) - 133.0).abs() < f64::EPSILON);
    }

    #[test]
    fn project_new_clears_history() {
        let mut s = session();
        run(&mut s, "project.set_tempo 140").unwrap();
        run(&mut s, "project.new").unwrap();
        assert_eq!(s.undo_depth(), 0);
        assert!((tempo(&s) - 130.0).abs() < f64::EPSILON);
    }
}
