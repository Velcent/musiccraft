//! Time signatures of the project and its patterns (see
//! `docs/spec/time-signatures.md`; song changes are playlist markers).

use musiccraft_project::{PatternId, Project, TimeSig};
use serde::{Deserialize, Serialize};

use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "project.set_time_signature",
            "project.set_time_signature <n/d>",
            "Set the song's time signature, and that of every pattern that had the old one (undoable).",
            set_project,
        ),
        cmd(
            "pattern.set_time_signature",
            "pattern.set_time_signature <pattern> <n/d>",
            "Set a pattern's time signature; a length of whole bars keeps its number of bars (undoable).",
            set_pattern,
        ),
        cmd(
            "project.restore_meter",
            "project.restore_meter <json>",
            "Restore the project and pattern signatures and pattern lengths (used by undo).",
            restore,
        ),
    ]
}

/// Every signature and pattern length, for undo.
#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    time_signature: TimeSig,
    /// `(pattern id, signature, length in steps)`.
    patterns: Vec<(PatternId, TimeSig, u32)>,
}

pub(super) fn snapshot(project: &Project, args: &Args<'_>) -> Result<Invocation, CommandError> {
    let snapshot = Snapshot {
        time_signature: project.time_signature,
        patterns: project
            .patterns
            .iter()
            .map(|p| (p.id, p.time_signature, p.length_steps))
            .collect(),
    };
    let json = serde_json::to_string(&snapshot).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Invocation::new("project.restore_meter", [json]))
}

fn set_project(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let sig = args.time_sig(0)?;
    let old = project.time_signature;
    if sig == old {
        return Ok(Outcome::done(format!("the project is already in {sig}")));
    }
    let inverse = snapshot(project, args)?;
    project.time_signature = sig;
    for pattern in project
        .patterns
        .iter_mut()
        .filter(|p| p.time_signature == old)
    {
        pattern.set_time_signature(sig);
    }
    Ok(Outcome::undoable(format!("time signature {sig}"), inverse))
}

fn set_pattern(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = project.pattern_index(args.number(0, "pattern")?)?;
    let sig = args.time_sig(1)?;
    if project.patterns[index].time_signature == sig {
        return Ok(Outcome::done(format!(
            "pattern {} is already in {sig}",
            index + 1
        )));
    }
    let inverse = snapshot(project, args)?;
    project.patterns[index].set_time_signature(sig);
    Ok(Outcome::undoable(
        format!("pattern {} in {sig}", index + 1),
        inverse,
    ))
}

fn restore(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let snapshot: Snapshot =
        serde_json::from_str(args.str(0)?).map_err(|e| args.invalid(e.to_string()))?;
    let inverse = self::snapshot(project, args)?;
    project.time_signature = snapshot.time_signature;
    for (id, sig, steps) in snapshot.patterns {
        if let Some(i) = project.pattern_position(id) {
            let pattern = &mut project.patterns[i];
            pattern.time_signature = sig;
            pattern.length_steps = steps.clamp(1, musiccraft_project::MAX_PATTERN_STEPS);
        }
    }
    Ok(Outcome::undoable("restored time signatures", inverse))
}
