//! Headless front end for MusicCraft. It only builds invocations and hands them to
//! a [`Session`]; it contains no project logic of its own.

use std::io::{self, Write};

use musiccraft_commands::{Invocation, REDO, Registry, Session, UNDO};
use musiccraft_project::Project;

/// Creates a session with a fresh project and every built-in command.
#[must_use]
pub fn new_session() -> Session {
    Session::new(Project::default(), Registry::with_builtin())
}

/// One-line summary of the project state.
#[must_use]
pub fn format_state(project: &Project) -> String {
    format!(
        "state: tempo={} bpm, transport={}",
        project.tempo_bpm(),
        project.transport()
    )
}

/// Prints every available command with its usage.
///
/// # Errors
///
/// Returns any error from writing to `out`.
pub fn list_commands(session: &Session, out: &mut impl Write) -> io::Result<()> {
    for command in session.registry().iter() {
        writeln!(out, "{:<20} {}", command.name(), command.summary())?;
        writeln!(out, "{:<20} usage: {}", "", command.usage())?;
    }
    writeln!(out, "{UNDO:<20} Undo the last undoable command.")?;
    writeln!(out, "{REDO:<20} Redo the last undone command.")?;
    Ok(())
}

/// Runs a script of one command per line. Blank lines and `#` comments are skipped.
/// Execution stops at the first error. Returns `Ok(true)` if every command succeeded.
///
/// # Errors
///
/// Returns any error from writing to `out`.
pub fn run_script(session: &mut Session, script: &str, out: &mut impl Write) -> io::Result<bool> {
    for raw in script.lines() {
        let line = raw
            .split_once('#')
            .map_or(raw, |(code, _comment)| code)
            .trim();
        if line.is_empty() {
            continue;
        }
        writeln!(out, "> {line}")?;
        match Invocation::parse(line).and_then(|invocation| session.run(&invocation)) {
            Ok(outcome) => writeln!(out, "ok: {}", outcome.message)?,
            Err(err) => {
                writeln!(out, "error: {err}")?;
                writeln!(out, "{}", format_state(session.project()))?;
                return Ok(false);
            }
        }
    }
    writeln!(out, "{}", format_state(session.project()))?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(script: &str) -> (bool, String) {
        let mut session = new_session();
        let mut out = Vec::new();
        let ok = run_script(&mut session, script, &mut out).unwrap();
        (ok, String::from_utf8(out).unwrap())
    }

    #[test]
    fn script_runs_commands_and_prints_final_state() {
        let (ok, output) = run("project.set_tempo 140\ntransport.play\n");
        assert!(ok);
        assert!(output.contains("> project.set_tempo 140"));
        assert!(output.contains("ok: tempo 140 BPM"));
        assert!(output.ends_with("state: tempo=140 bpm, transport=playing\n"));
    }

    #[test]
    fn comments_and_blank_lines_are_skipped() {
        let (ok, output) = run("# set up\n\nproject.set_tempo 100 # slow\n");
        assert!(ok);
        assert!(output.contains("state: tempo=100 bpm, transport=stopped"));
    }

    #[test]
    fn script_stops_at_first_error() {
        let (ok, output) = run("project.set_tempo 5000\ntransport.play\n");
        assert!(!ok);
        assert!(output.contains("error: tempo 5000 is outside the supported range"));
        assert!(!output.contains("transport.play"));
    }

    #[test]
    fn undo_and_redo_work_in_scripts() {
        let (ok, output) = run("project.set_tempo 150\nedit.undo\nedit.redo\n");
        assert!(ok);
        assert!(output.contains("ok: undid `project.set_tempo 150`"));
        assert!(output.contains("ok: redid `project.set_tempo 150`"));
        assert!(output.ends_with("state: tempo=150 bpm, transport=stopped\n"));
    }

    #[test]
    fn list_includes_commands_and_history_actions() {
        let session = new_session();
        let mut out = Vec::new();
        list_commands(&session, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("transport.toggle"));
        assert!(text.contains("project.set_tempo"));
        assert!(text.contains("edit.undo"));
        assert!(text.contains("edit.redo"));
    }
}
