//! Any automatable control by its target name: `control.set` and
//! `control.reset` (see `docs/spec/controls.md`, "Control menu").

use musiccraft_project::params::{INSTRUMENT_PARAMS, index_of, volume_from_unit, volume_to_unit};
use musiccraft_project::{AutomationTarget, DEFAULT_CHANNEL_VOLUME, MAX_VOLUME, Project};

use super::automation::{current, parse_target};
use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "control.set",
            "control.set <target> <position 0-1>",
            "Set a control (named as for automation.create) to a position along its range.",
            set,
        ),
        cmd(
            "control.reset",
            "control.reset <target>",
            "Return a control (named as for automation.create) to its default.",
            reset,
        ),
    ]
}

/// The target's default as a control position.
fn default_position(project: &Project, target: &AutomationTarget) -> Option<f32> {
    let channel = |id| project.channel_position(id).map(|i| &project.channels[i]);
    match target {
        AutomationTarget::ChannelVolume { .. } => {
            Some(volume_to_unit(DEFAULT_CHANNEL_VOLUME, MAX_VOLUME))
        }
        AutomationTarget::MixerVolume { .. } => Some(volume_to_unit(1.0, MAX_VOLUME)),
        AutomationTarget::ChannelPan { .. } | AutomationTarget::MixerPan { .. } => Some(0.5),
        AutomationTarget::ChannelParam { channel: id, param } => {
            if channel(*id)?.instrument.patch().is_some() {
                let spec =
                    musiccraft_antiserum::params().get(musiccraft_antiserum::index_of(param)?)?;
                return Some(spec.to_unit(spec.default));
            }
            let spec = &INSTRUMENT_PARAMS[index_of(INSTRUMENT_PARAMS, param)?];
            Some(spec.to_unit(spec.default))
        }
        AutomationTarget::EffectParam { track, slot, param } => {
            let kind = project.mixer.get(*track)?.effects.get(*slot)?.kind;
            let spec = &kind.params()[index_of(kind.params(), param)?];
            Some(spec.to_unit(spec.default))
        }
    }
}

/// Writes a control position to the target. `None` when it no longer exists.
fn write(project: &mut Project, target: &AutomationTarget, position: f32) -> Option<()> {
    let index = |project: &Project, id| project.channel_position(id);
    match target {
        AutomationTarget::ChannelVolume { channel } => {
            let i = index(project, *channel)?;
            project.channels[i].volume = volume_from_unit(position, MAX_VOLUME);
        }
        AutomationTarget::ChannelPan { channel } => {
            let i = index(project, *channel)?;
            project.channels[i].pan = position * 2.0 - 1.0;
        }
        AutomationTarget::ChannelParam { channel, param } => {
            let i = index(project, *channel)?;
            let c = &mut project.channels[i];
            if let Some(patch) = c.instrument.patch_mut() {
                let spec =
                    musiccraft_antiserum::params().get(musiccraft_antiserum::index_of(param)?)?;
                patch.params.insert(param.clone(), spec.from_unit(position));
            } else {
                let p = index_of(INSTRUMENT_PARAMS, param)?;
                *c.params.0.get_mut(p)? = INSTRUMENT_PARAMS[p].from_unit(position);
            }
        }
        AutomationTarget::MixerVolume { track } => {
            project.mixer.get_mut(*track)?.volume = volume_from_unit(position, MAX_VOLUME);
        }
        AutomationTarget::MixerPan { track } => {
            project.mixer.get_mut(*track)?.pan = position * 2.0 - 1.0;
        }
        AutomationTarget::EffectParam { track, slot, param } => {
            let effect = project.mixer.get_mut(*track)?.effects.get_mut(*slot)?;
            let p = index_of(effect.kind.params(), param)?;
            *effect.values.get_mut(p)? = effect.kind.params()[p].from_unit(position);
        }
    }
    Some(())
}

/// Moves `name`'s control to `position`; the inverse puts it back.
fn apply(
    project: &mut Project,
    args: &Args<'_>,
    name: &str,
    position: impl FnOnce(&Project, &AutomationTarget) -> Option<f32>,
) -> Result<Outcome, CommandError> {
    let target = parse_target(project, args, name)?;
    let gone = || args.invalid("the target does not exist");
    let (old, label) = current(project, &target).ok_or_else(gone)?;
    let new = position(project, &target).ok_or_else(gone)?.clamp(0.0, 1.0);
    write(project, &target, new).ok_or_else(gone)?;
    Ok(Outcome::undoable(
        format!("set {label} to {:.0}%", new * 100.0),
        Invocation::new("control.set", [name.to_owned(), format!("{old}")]),
    ))
}

fn set(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let position = args.f32(1, "position")?;
    if !(0.0..=1.0).contains(&position) {
        return Err(args.invalid("the position must be between 0 and 1"));
    }
    apply(project, args, args.str(0)?, |_, _| Some(position))
}

fn reset(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    apply(project, args, args.str(0)?, default_position)
}

/// What the control menu shows about one control.
#[derive(Debug, Clone, PartialEq)]
pub struct ControlInfo {
    /// Its position along its range, 0..1.
    pub position: f32,
    /// Its default position.
    pub default: f32,
    /// A readable name, such as "Master volume".
    pub label: String,
}

/// A control's current and default positions by target name, for the
/// control menu's Copy and type-in box.
#[must_use]
pub fn info(project: &Project, name: &str) -> Option<ControlInfo> {
    let items = [name.to_owned()];
    let args = Args::new("control.set", "control.set <target> <position 0-1>", &items);
    let target = parse_target(project, &args, name).ok()?;
    let (position, label) = current(project, &target)?;
    Some(ControlInfo {
        position,
        default: default_position(project, &target)?,
        label,
    })
}
