//! The effect rack's branches: `mixer.rack_*` (see `docs/spec/mixer.md`).

use musiccraft_project::params::{
    EFFECT_KINDS, EffectKind, RACK_BRANCH_EFFECTS, RACK_BRANCHES, index_of,
};
use musiccraft_project::{EffectSlot, Project, ProjectError};

use super::mixer::{slot_arg, track_arg};
use crate::args::{Args, num, on_off};
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "mixer.rack_add",
            "mixer.rack_add <track> <slot> <branch 1-4> <kind>",
            "Append a built-in effect to a branch of an effect rack.",
            rack_add,
        ),
        cmd(
            "mixer.rack_remove",
            "mixer.rack_remove <track> <slot> <branch> <n>",
            "Remove effect n from a branch of an effect rack.",
            rack_remove,
        ),
        cmd(
            "mixer.rack_insert",
            "mixer.rack_insert <track> <slot> <branch> <n> <json>",
            "Insert an effect into a rack branch from its JSON form (used by undo).",
            rack_insert,
        ),
        cmd(
            "mixer.rack_param",
            "mixer.rack_param <track> <slot> <branch> <n> <param> <value>",
            "Set a parameter of an effect inside an effect rack.",
            rack_param,
        ),
        cmd(
            "mixer.rack_enable",
            "mixer.rack_enable <track> <slot> <branch> <n> <on|off>",
            "Enable or bypass an effect inside an effect rack.",
            rack_enable,
        ),
    ]
}

/// Where a rack command acts: track, slot and branch (0-based).
type At = (usize, usize, usize);

/// The track, rack slot and branch named by the first three arguments.
fn rack_arg(project: &Project, args: &Args<'_>) -> Result<At, CommandError> {
    let track = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, track, 1)?;
    if project.mixer[track].effects[slot].kind != EffectKind::Rack {
        return Err(args.invalid(format!(
            "slot {} of mixer track {track} is not an effect rack",
            slot + 1
        )));
    }
    let branch = args.number(2, "branch")?;
    if !(1..=RACK_BRANCHES).contains(&branch) {
        return Err(args.invalid(format!("a rack has branches 1 to {RACK_BRANCHES}")));
    }
    Ok((track, slot, branch - 1))
}

fn branch_mut(project: &mut Project, (t, s, b): At) -> &mut Vec<EffectSlot> {
    let rack = &mut project.mixer[t].effects[s];
    if rack.branches.len() < RACK_BRANCHES {
        rack.branches.resize(RACK_BRANCHES, Vec::new());
    }
    &mut rack.branches[b]
}

/// The position (0-based) of an existing effect named by argument `i`.
fn item_arg(
    project: &mut Project,
    args: &Args<'_>,
    at: At,
    i: usize,
) -> Result<usize, CommandError> {
    let n = args.number(i, "rack effect")?;
    if n > branch_mut(project, at).len() {
        return Err(ProjectError::NotFound {
            what: "rack effect",
            number: n,
        }
        .into());
    }
    Ok(n - 1)
}

/// The undo/redo arguments locating an effect.
fn place((t, s, b): At, n: usize) -> [String; 4] {
    [
        t.to_string(),
        (s + 1).to_string(),
        (b + 1).to_string(),
        (n + 1).to_string(),
    ]
}

fn check_kind(args: &Args<'_>, kind: EffectKind) -> Result<(), CommandError> {
    if kind.allowed_in_rack() {
        Ok(())
    } else {
        Err(args.invalid(format!(
            "{kind} cannot run inside an effect rack (sends, band splitters, plugins and racks)"
        )))
    }
}

fn check_room(project: &mut Project, args: &Args<'_>, at: At) -> Result<(), CommandError> {
    if branch_mut(project, at).len() >= RACK_BRANCH_EFFECTS {
        return Err(args.invalid(format!(
            "a rack branch holds at most {RACK_BRANCH_EFFECTS} effects"
        )));
    }
    Ok(())
}

fn rack_add(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let at = rack_arg(project, args)?;
    let raw = args.str(3)?;
    let kind = EffectKind::parse(raw).ok_or_else(|| {
        let names: Vec<_> = EFFECT_KINDS
            .iter()
            .filter(|k| k.allowed_in_rack())
            .map(|k| k.name())
            .collect();
        args.invalid(format!(
            "unknown effect `{raw}`; choose from {}",
            names.join(", ")
        ))
    })?;
    check_kind(args, kind)?;
    check_room(project, args, at)?;
    let branch = branch_mut(project, at);
    branch.push(EffectSlot::new(kind));
    let n = branch.len() - 1;
    Ok(Outcome::undoable(
        format!("added {kind} to rack branch {}", at.2 + 1),
        Invocation::new("mixer.rack_remove", place(at, n)),
    ))
}

fn rack_remove(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let at = rack_arg(project, args)?;
    let n = item_arg(project, args, at, 3)?;
    let effect = branch_mut(project, at).remove(n);
    let json = serde_json::to_string(&effect).map_err(|e| args.invalid(e.to_string()))?;
    let [t, s, b, n] = place(at, n);
    Ok(Outcome::undoable(
        format!("removed {} from rack branch {b}", effect.kind),
        Invocation::new("mixer.rack_insert", [t, s, b, n, json]),
    ))
}

fn rack_insert(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(5, 5)?;
    let at = rack_arg(project, args)?;
    let n = args.number(3, "rack effect")?;
    let effect: EffectSlot =
        serde_json::from_str(args.str(4)?).map_err(|e| args.invalid(e.to_string()))?;
    check_kind(args, effect.kind)?;
    check_room(project, args, at)?;
    let branch = branch_mut(project, at);
    if n == 0 || n > branch.len() + 1 {
        return Err(args.invalid(format!("cannot insert a rack effect at {n}")));
    }
    let kind = effect.kind;
    branch.insert(n - 1, effect);
    Ok(Outcome::undoable(
        format!("inserted {kind} into rack branch {}", at.2 + 1),
        Invocation::new("mixer.rack_remove", place(at, n - 1)),
    ))
}

fn rack_param(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(6, 6)?;
    let at = rack_arg(project, args)?;
    let n = item_arg(project, args, at, 3)?;
    let effect = &mut branch_mut(project, at)[n];
    let specs = effect.kind.params();
    let name = args.str(4)?;
    let param = index_of(specs, name).ok_or_else(|| {
        let names: Vec<_> = specs.iter().map(|p| p.name).collect();
        args.invalid(format!(
            "{} has no parameter `{name}`; choose from {}",
            effect.kind,
            names.join(", ")
        ))
    })?;
    let kind = effect.kind;
    if kind.channel_ref_index() == Some(param) || kind.track_ref_indices().any(|i| i == param) {
        return Err(args.invalid("effects inside a rack have no sidechain and follow no channel"));
    }
    let spec = specs[param];
    let value = args.f32(5, name)?;
    if !spec.accepts(value) {
        return Err(ProjectError::out_of_range(
            name,
            f64::from(value),
            f64::from(spec.min),
            f64::from(spec.max),
        )
        .into());
    }
    let value = spec.sanitize(value);
    let previous = std::mem::replace(&mut effect.values[param], value);
    let message = format!("rack {kind} {name} {}", num(value));
    if previous == value {
        return Ok(Outcome::done(message));
    }
    let [t, s, b, n] = place(at, n);
    Ok(Outcome::undoable(
        message,
        Invocation::new(args.name(), [t, s, b, n, name.to_owned(), num(previous)]),
    ))
}

fn rack_enable(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(5, 5)?;
    let at = rack_arg(project, args)?;
    let n = item_arg(project, args, at, 3)?;
    let enabled = args.flag(4)?;
    let effect = &mut branch_mut(project, at)[n];
    let previous = std::mem::replace(&mut effect.enabled, enabled);
    let message = format!(
        "rack {} {}",
        effect.kind,
        if enabled { "enabled" } else { "bypassed" }
    );
    if previous == enabled {
        return Ok(Outcome::done(message));
    }
    let [t, s, b, n] = place(at, n);
    Ok(Outcome::undoable(
        message,
        Invocation::new(args.name(), [t, s, b, n, on_off(previous).to_owned()]),
    ))
}
