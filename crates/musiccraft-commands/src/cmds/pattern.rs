use musiccraft_project::{Clip, MAX_PATTERN_STEPS, Pattern, PatternId, Project};
use serde::{Deserialize, Serialize};

use crate::args::Args;
use crate::command::cmd;
use crate::{BATCH, Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "pattern.add",
            "pattern.add [name]",
            "Create an empty pattern and select it.",
            add,
        ),
        cmd(
            "pattern.remove",
            "pattern.remove <pattern>",
            "Delete a pattern and its playlist clips.",
            remove,
        ),
        cmd(
            "pattern.insert",
            "pattern.insert <position> <json>",
            "Insert a pattern from its JSON form (used by undo).",
            insert,
        ),
        cmd(
            "pattern.clone",
            "pattern.clone <pattern>",
            "Duplicate a pattern after itself and select the copy.",
            clone,
        ),
        cmd(
            "pattern.select",
            "pattern.select <pattern>",
            "Select the pattern shown in the channel rack and piano roll.",
            select,
        ),
        cmd(
            "pattern.next",
            "pattern.next",
            "Select the next pattern.",
            next,
        ),
        cmd(
            "pattern.prev",
            "pattern.prev",
            "Select the previous pattern.",
            prev,
        ),
        cmd(
            "pattern.rename",
            "pattern.rename <pattern> <name>",
            "Rename a pattern.",
            rename,
        ),
        cmd(
            "pattern.set_length",
            "pattern.set_length <pattern> <steps>",
            "Set a pattern's length in steps.",
            set_length,
        ),
    ]
}

fn pattern_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    Ok(project.pattern_index(args.number(i, "pattern")?)?)
}

fn select_invocation(index: usize) -> String {
    format!("pattern.select {}", index + 1)
}

fn add(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = if args.len() > 0 {
        args.rest(0)?
    } else {
        format!("Pattern {}", project.patterns.len() + 1)
    };
    let previous = project.current_pattern;
    let id = PatternId(project.alloc_id());
    project.patterns.push(Pattern::new(id, name.clone()));
    let number = project.patterns.len();
    project.current_pattern = number - 1;
    Ok(Outcome::undoable(
        format!("added pattern {number} ({name})"),
        Invocation::new(
            BATCH,
            [
                format!("pattern.remove {number}"),
                select_invocation(previous),
            ],
        ),
    ))
}

#[derive(Serialize, Deserialize)]
struct PatternSnapshot {
    pattern: Pattern,
    clips: Vec<Clip>,
}

fn remove(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = pattern_arg(project, args, 0)?;
    if project.patterns.len() == 1 {
        return Err(args.invalid("a project needs at least one pattern"));
    }
    let pattern = project.patterns.remove(index);
    let (clips, kept): (Vec<Clip>, Vec<Clip>) = project
        .playlist
        .clips
        .drain(..)
        .partition(|c| c.pattern() == Some(pattern.id));
    project.playlist.clips = kept;
    let previous = project.current_pattern;
    if previous > index || previous >= project.patterns.len() {
        project.current_pattern = previous.saturating_sub(1);
    }
    let name = pattern.name.clone();
    let json = serde_json::to_string(&PatternSnapshot { pattern, clips })
        .map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("removed pattern {} ({name})", index + 1),
        Invocation::new(
            BATCH,
            [
                Invocation::new("pattern.insert", [(index + 1).to_string(), json]).to_string(),
                select_invocation(previous),
            ],
        ),
    ))
}

fn insert(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let position = args.number(0, "position")?;
    if position > project.patterns.len() + 1 {
        return Err(args.invalid(format!("position {position} is past the last pattern")));
    }
    let snapshot: PatternSnapshot =
        serde_json::from_str(args.str(1)?).map_err(|e| args.invalid(e.to_string()))?;
    let mut pattern = snapshot.pattern;
    if project.pattern_position(pattern.id).is_some() {
        return Err(args.invalid(format!("pattern id {} already exists", pattern.id.0)));
    }
    project.reserve_id(pattern.id.0);
    pattern
        .notes
        .retain(|n| project.channel_position(n.channel).is_some());
    let name = pattern.name.clone();
    project.patterns.insert(position - 1, pattern);
    if project.current_pattern >= position - 1 {
        project.current_pattern += 1;
    }
    project.playlist.clips.extend(snapshot.clips);
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!("inserted pattern {position} ({name})"),
        Invocation::new("pattern.remove", [position.to_string()]),
    ))
}

fn clone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = pattern_arg(project, args, 0)?;
    let previous = project.current_pattern;
    let mut copy = project.patterns[index].clone();
    copy.id = PatternId(project.alloc_id());
    copy.name = format!("{} copy", copy.name);
    project.patterns.insert(index + 1, copy);
    project.current_pattern = index + 1;
    let number = index + 2;
    Ok(Outcome::undoable(
        format!("cloned pattern {} to {number}", index + 1),
        Invocation::new(
            BATCH,
            [
                format!("pattern.remove {number}"),
                select_invocation(previous),
            ],
        ),
    ))
}

fn selected(project: &Project) -> Outcome {
    let pattern = project.current_pattern();
    Outcome::done(format!(
        "pattern {} ({}) selected",
        project.current_pattern + 1,
        pattern.name
    ))
}

fn select(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    project.current_pattern = pattern_arg(project, args, 0)?;
    Ok(selected(project))
}

fn next(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.current_pattern = (project.current_pattern + 1).min(project.patterns.len() - 1);
    Ok(selected(project))
}

fn prev(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.current_pattern = project.current_pattern.saturating_sub(1);
    Ok(selected(project))
}

fn rename(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let index = pattern_arg(project, args, 0)?;
    let name = args.rest(1)?;
    let previous = std::mem::replace(&mut project.patterns[index].name, name.clone());
    Ok(Outcome::undoable(
        format!("pattern {} renamed to {name}", index + 1),
        Invocation::new(args.name(), [(index + 1).to_string(), previous]),
    ))
}

fn set_length(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = pattern_arg(project, args, 0)?;
    let steps: u32 = args.parse(1, "step count")?;
    if !(1..=MAX_PATTERN_STEPS).contains(&steps) {
        return Err(args.invalid(format!("length must be 1 to {MAX_PATTERN_STEPS} steps")));
    }
    let previous = std::mem::replace(&mut project.patterns[index].length_steps, steps);
    let message = format!("pattern {} is {steps} steps", index + 1);
    if previous == steps {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(args.name(), [(index + 1).to_string(), previous.to_string()]),
    ))
}
