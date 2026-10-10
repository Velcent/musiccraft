//! Headless front end for MusicCraft. It only builds invocations and hands them to
//! a [`Session`]; it contains no project logic of its own.

use std::io::{self, Write};

use musiccraft_commands::{Invocation, Registry, SESSION_COMMANDS, Session};
use musiccraft_project::Project;

/// A registry with every built-in command plus rendering and export.
#[must_use]
pub fn full_registry() -> Registry {
    let mut registry = Registry::with_builtin();
    // Registration into a fresh registry cannot collide.
    let _ = musiccraft_render::register(&mut registry);
    registry
}

/// Creates a session with a fresh project and every command.
#[must_use]
pub fn new_session() -> Session {
    Session::new(Project::default(), full_registry())
}

/// One-line summary of the project state.
#[must_use]
pub fn format_state(project: &Project) -> String {
    format!(
        "state: tempo={} bpm, transport={}, mode={}, pattern={}/{}, channels={}, clips={}",
        project.tempo_bpm(),
        project.transport(),
        project.play_mode.label(),
        project.current_pattern + 1,
        project.patterns.len(),
        project.channels.len(),
        project.playlist.clips.len(),
    )
}

/// Prints every available command with its usage.
///
/// # Errors
///
/// Returns any error from writing to `out`.
pub fn list_commands(session: &Session, out: &mut impl Write) -> io::Result<()> {
    for command in session.registry().iter() {
        writeln!(out, "{:<28} {}", command.name(), command.summary())?;
        writeln!(out, "{:<28} usage: {}", "", command.usage())?;
    }
    for (name, usage, summary) in SESSION_COMMANDS {
        writeln!(out, "{name:<28} {summary}")?;
        writeln!(out, "{:<28} usage: {usage}", "")?;
    }
    Ok(())
}

/// Runs a script of one command per line. Blank lines and `#` comments are skipped.
/// Execution stops at the first error. Returns `Ok(true)` if every command succeeded.
///
/// # Errors
///
/// Returns any error from writing to `out`.
pub fn run_script(session: &mut Session, script: &str, out: &mut impl Write) -> io::Result<bool> {
    for line in script.lines() {
        let invocation = match Invocation::parse(line) {
            Ok(invocation) => invocation,
            Err(musiccraft_commands::CommandError::EmptyLine) => continue,
            Err(err) => {
                writeln!(out, "> {}", line.trim())?;
                writeln!(out, "error: {err}")?;
                writeln!(out, "{}", format_state(session.project()))?;
                return Ok(false);
            }
        };
        writeln!(out, "> {invocation}")?;
        match session.run(&invocation) {
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
        assert!(output.contains("state: tempo=140 bpm, transport=playing"));
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
    fn parse_errors_stop_the_script() {
        let (ok, output) = run("project.rename \"unterminated\n");
        assert!(!ok);
        assert!(output.contains("error: cannot parse command"));
    }

    #[test]
    fn undo_and_redo_work_in_scripts() {
        let (ok, output) = run("project.set_tempo 150\nedit.undo\nedit.redo\n");
        assert!(ok);
        assert!(output.contains("ok: undid `project.set_tempo 150`"));
        assert!(output.contains("ok: redid `project.set_tempo 150`"));
        assert!(output.contains("state: tempo=150 bpm, transport=stopped"));
    }

    #[test]
    fn list_includes_commands_and_history_actions() {
        let session = new_session();
        let mut out = Vec::new();
        list_commands(&session, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        for name in [
            "transport.toggle",
            "project.set_tempo",
            "project.export_wav",
            "edit.undo",
            "edit.redo",
            "edit.batch",
        ] {
            assert!(text.contains(name), "{name} missing");
        }
    }
}
