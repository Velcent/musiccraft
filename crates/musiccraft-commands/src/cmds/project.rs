use std::fs;
use std::path::PathBuf;

use musiccraft_project::Project;

use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "project.set_tempo",
            "project.set_tempo <bpm>",
            "Set the project tempo in beats per minute (undoable).",
            set_tempo,
        ),
        cmd(
            "project.rename",
            "project.rename <name>",
            "Rename the project.",
            rename,
        ),
        cmd(
            "project.new",
            "project.new",
            "Replace the project with an empty one (clears undo history).",
            new,
        ),
        cmd(
            "project.save",
            "project.save [path]",
            "Save the project as JSON (default: the file it came from).",
            save,
        ),
        cmd(
            "project.load",
            "project.load <path>",
            "Load a project file (clears undo history).",
            load,
        ),
        cmd(
            "project.recover",
            "project.recover <path>",
            "Load a crash backup as unsaved work: saving asks for a file (clears undo history).",
            recover,
        ),
    ]
}

fn set_tempo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let raw = args.str(0)?;
    let bpm: f64 = raw
        .parse()
        .map_err(|_| args.invalid(format!("`{raw}` is not a number")))?;
    let previous = project.set_tempo_bpm(bpm)?;
    let message = format!("tempo {bpm} BPM");
    if (previous - bpm).abs() < f64::EPSILON {
        Ok(Outcome::done(message))
    } else {
        Ok(Outcome::undoable(
            message,
            Invocation::new(args.name(), [previous.to_string()]),
        ))
    }
}

fn rename(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = args.rest(0)?;
    let previous = project.set_name(name.clone());
    Ok(Outcome::undoable(
        format!("project renamed to {name}"),
        Invocation::new(args.name(), [previous]),
    ))
}

fn new(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    *project = Project::default();
    Ok(Outcome::reset("new project"))
}

fn save(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 1)?;
    let path = match args.opt(0) {
        Some(path) => PathBuf::from(path),
        None => project
            .path()
            .map(PathBuf::from)
            .ok_or_else(|| args.invalid("the project has never been saved; give a path"))?,
    };
    let json = project.to_json()?;
    let temp = path.with_extension("mcproj.tmp");
    fs::write(&temp, json)
        .and_then(|()| fs::rename(&temp, &path))
        .map_err(|e| CommandError::Io(format!("cannot save {}: {e}", path.display())))?;
    let absolute = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    project.set_path(Some(absolute));
    Ok(Outcome::done(format!("saved {}", path.display())))
}

fn load(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let path = PathBuf::from(args.str(0)?);
    let text = fs::read_to_string(&path)
        .map_err(|e| CommandError::Io(format!("cannot read {}: {e}", path.display())))?;
    let mut loaded = Project::from_json(&text)?;
    loaded.set_path(Some(
        fs::canonicalize(&path).unwrap_or_else(|_| path.clone()),
    ));
    *project = loaded;
    Ok(Outcome::reset(format!("loaded {}", path.display())))
}

fn recover(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let path = PathBuf::from(args.str(0)?);
    let text = fs::read_to_string(&path)
        .map_err(|e| CommandError::Io(format!("cannot read {}: {e}", path.display())))?;
    let mut recovered = Project::from_json(&text)?;
    // A backup is not where the work belongs, so saving must ask for a file
    // instead of quietly writing into the app's data folder.
    recovered.set_path(None);
    *project = recovered;
    Ok(Outcome::reset(format!(
        "recovered unsaved work from {}; save it to keep it",
        path.display()
    )))
}
