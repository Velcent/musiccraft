//! Controllers: `controller.add`, `controller.remove`, `controller.insert`,
//! `controller.set` and `controller.restore` (see `docs/spec/controllers.md`).

use musiccraft_formula::Formula;
use musiccraft_project::{Controller, ControllerKind, Project, check_range};

use super::automation::{current, parse_target};
use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "controller.add",
            "controller.add <lfo|envelope|keyboard|peak|formula|macro> <target> [name]",
            "Add a controller that moves a target (as for automation.create) from a live source.",
            add,
        ),
        cmd(
            "controller.remove",
            "controller.remove <controller>",
            "Delete a controller.",
            remove,
        ),
        cmd(
            "controller.insert",
            "controller.insert <position> <json>",
            "Insert a controller from its JSON form (used by undo).",
            insert,
        ),
        cmd(
            "controller.set",
            "controller.set <controller> <setting> <value>",
            "Change a controller: name, enabled, min, max, target, or its kind's settings (shape, period, phase, channel, attack, decay, sustain, release, low, high, track, gain, expr, value).",
            set,
        ),
        cmd(
            "controller.restore",
            "controller.restore <controller> <json>",
            "Replace a controller with its JSON form (used by undo).",
            restore,
        ),
    ]
}

fn controller_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    Ok(project.controller_index(args.number(i, "controller")?)?)
}

fn to_json(args: &Args<'_>, controller: &Controller) -> Result<String, CommandError> {
    serde_json::to_string(controller).map_err(|e| args.invalid(e.to_string()))
}

fn from_json(args: &Args<'_>, text: &str) -> Result<Controller, CommandError> {
    serde_json::from_str(text).map_err(|e| args.invalid(e.to_string()))
}

/// The settings a new controller of `kind` starts with.
fn default_kind(
    project: &Project,
    args: &Args<'_>,
    kind: &str,
    start: f32,
) -> Result<ControllerKind, CommandError> {
    let channel = || {
        project
            .channels
            .first()
            .map(|c| c.id)
            .ok_or_else(|| args.invalid("this controller listens to a channel; add one first"))
    };
    Ok(match kind {
        "lfo" => ControllerKind::Lfo {
            shape: 0,
            period: 4.0,
            phase: 0.0,
        },
        "envelope" => ControllerKind::Envelope {
            channel: channel()?,
            attack: 0.01,
            decay: 0.3,
            sustain: 0.5,
            release: 0.3,
        },
        "keyboard" => ControllerKind::Keyboard {
            channel: channel()?,
            low: 36,
            high: 96,
        },
        "peak" => ControllerKind::Peak {
            track: usize::from(project.mixer.len() > 1),
            gain: 1.0,
            attack: 5.0,
            release: 200.0,
        },
        "formula" => ControllerKind::Formula {
            expr: "0.5 + 0.5 * sin(t * pi / 2)".to_owned(),
        },
        "macro" => ControllerKind::Macro { value: start },
        other => {
            return Err(args.invalid(format!(
                "unknown controller kind `{other}`; choose from {}",
                ControllerKind::NAMES.join(", ")
            )));
        }
    })
}

fn add(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let target = parse_target(project, args, args.str(1)?)?;
    let (start, label) =
        current(project, &target).ok_or_else(|| args.invalid("the target does not exist"))?;
    let kind = default_kind(project, args, args.str(0)?, start)?;
    let name = if args.len() > 2 {
        args.rest(2)?
    } else {
        format!("{}: {label}", kind.name())
    };
    project.controllers.push(Controller {
        name: name.clone(),
        kind,
        target,
        min: 0.0,
        max: 1.0,
        enabled: true,
    });
    let number = project.controllers.len();
    Ok(Outcome::undoable(
        format!("added controller {number} ({name})"),
        Invocation::new("controller.remove", [number.to_string()]),
    ))
}

fn remove(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = controller_arg(project, args, 0)?;
    let controller = project.controllers.remove(index);
    let json = to_json(args, &controller)?;
    Ok(Outcome::undoable(
        format!("removed controller {} ({})", index + 1, controller.name),
        Invocation::new("controller.insert", [(index + 1).to_string(), json]),
    ))
}

fn insert(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let position = args.number(0, "position")?;
    if position > project.controllers.len() + 1 {
        return Err(args.invalid(format!("position {position} is past the last controller")));
    }
    let controller = from_json(args, args.str(1)?)?;
    let name = controller.name.clone();
    project.controllers.insert(position - 1, controller);
    Ok(Outcome::undoable(
        format!("inserted controller {position} ({name})"),
        Invocation::new("controller.remove", [position.to_string()]),
    ))
}

fn restore(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = controller_arg(project, args, 0)?;
    let controller = from_json(args, args.str(1)?)?;
    let previous = std::mem::replace(&mut project.controllers[index], controller);
    Ok(Outcome::undoable(
        format!("restored controller {}", index + 1),
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), to_json(args, &previous)?],
        ),
    ))
}

fn set(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, usize::MAX)?;
    let index = controller_arg(project, args, 0)?;
    let setting = args.str(1)?;
    let before = project.controllers[index].clone();
    let mut after = before.clone();
    let number = |what: &str, min: f32, max: f32| -> Result<f32, CommandError> {
        Ok(check_range(what, args.f32(2, what)?, min, max)?)
    };
    let unknown = || {
        args.invalid(format!(
            "a {} controller has no setting `{setting}`",
            before.kind.name()
        ))
    };
    match setting {
        "name" => after.name = args.rest(2)?,
        "enabled" => after.enabled = args.flag(2)?,
        "min" => after.min = number("min", 0.0, 1.0)?,
        "max" => after.max = number("max", 0.0, 1.0)?,
        "target" => after.target = parse_target(project, args, args.str(2)?)?,
        _ => set_kind(project, args, setting, &mut after.kind, &number)
            .map_err(|e| e.unwrap_or_else(unknown))?,
    }
    let json = to_json(args, &before)?;
    let message = format!("controller {} {setting} set", index + 1);
    project.controllers[index] = after;
    Ok(Outcome::undoable(
        message,
        Invocation::new("controller.restore", [(index + 1).to_string(), json]),
    ))
}

/// Sets one of the kind's own settings. `Err(None)` means the kind has no
/// such setting.
fn set_kind(
    project: &Project,
    args: &Args<'_>,
    setting: &str,
    kind: &mut ControllerKind,
    number: &dyn Fn(&str, f32, f32) -> Result<f32, CommandError>,
) -> Result<(), Option<CommandError>> {
    let channel = || -> Result<_, CommandError> {
        let index = project.channel_index(args.number(2, "channel")?)?;
        Ok(project.channels[index].id)
    };
    match (kind, setting) {
        (ControllerKind::Lfo { shape, .. }, "shape") => {
            *shape = number("shape", 0.0, 4.0)?.round() as u8
        }
        (ControllerKind::Lfo { period, .. }, "period") => *period = number("period", 0.0625, 64.0)?,
        (ControllerKind::Lfo { phase, .. }, "phase") => *phase = number("phase", 0.0, 1.0)?,
        (
            ControllerKind::Envelope { channel: c, .. }
            | ControllerKind::Keyboard { channel: c, .. },
            "channel",
        ) => {
            *c = channel()?;
        }
        (ControllerKind::Envelope { attack, .. }, "attack") => {
            *attack = number("attack", 0.0, 10.0)?
        }
        (ControllerKind::Envelope { decay, .. }, "decay") => *decay = number("decay", 0.0, 10.0)?,
        (ControllerKind::Envelope { sustain, .. }, "sustain") => {
            *sustain = number("sustain", 0.0, 1.0)?
        }
        (ControllerKind::Envelope { release, .. }, "release") => {
            *release = number("release", 0.0, 10.0)?
        }
        (ControllerKind::Keyboard { low, .. }, "low") => {
            *low = number("low", 0.0, 127.0)?.round() as u8
        }
        (ControllerKind::Keyboard { high, .. }, "high") => {
            *high = number("high", 0.0, 127.0)?.round() as u8
        }
        (ControllerKind::Peak { track, .. }, "track") => {
            *track = project
                .mixer_index(args.parse(2, "mixer track")?)
                .map_err(CommandError::from)?;
        }
        (ControllerKind::Peak { gain, .. }, "gain") => *gain = number("gain", 0.0, 8.0)?,
        (ControllerKind::Peak { attack, .. }, "attack") => {
            *attack = number("attack", 0.1, 1_000.0)?
        }
        (ControllerKind::Peak { release, .. }, "release") => {
            *release = number("release", 1.0, 5_000.0)?
        }
        (ControllerKind::Formula { expr }, "expr") => {
            let text = args.rest(2)?;
            Formula::parse(&text).map_err(|e| args.invalid(format!("bad formula: {e}")))?;
            *expr = text;
        }
        (ControllerKind::Macro { value }, "value") => *value = number("value", 0.0, 1.0)?,
        _ => return Err(None),
    }
    Ok(())
}
