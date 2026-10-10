//! Commands that add hosted plugins and store their parameters and state.
//! Which plugins exist is the host's business; these commands only record
//! references, so a project naming a missing plugin still loads and edits.

use musiccraft_project::params::MAX_EFFECT_SLOTS;
use musiccraft_project::{
    Channel, ChannelId, EffectSlot, Instrument, PluginRef, PluginSlot, Project, base64,
};

use super::channel::channel_arg;
use super::mixer::{slot_arg, track_arg};
use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

/// Argument that removes a stored parameter value.
const UNSET: &str = "unset";

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "channel.add_plugin",
            "channel.add_plugin <plugin-id> [name]",
            "Add a channel played by a plugin instrument.",
            add_channel,
        ),
        cmd(
            "mixer.add_plugin",
            "mixer.add_plugin <track> <plugin-id> [name]",
            "Add a plugin effect to a mixer track.",
            add_effect,
        ),
        cmd(
            "channel.set_plugin_param",
            "channel.set_plugin_param <channel> <param-id> <value|unset>",
            "Set (or forget) a parameter of a channel's plugin.",
            set_channel_param,
        ),
        cmd(
            "mixer.set_plugin_param",
            "mixer.set_plugin_param <track> <slot> <param-id> <value|unset>",
            "Set (or forget) a parameter of a plugin effect.",
            set_effect_param,
        ),
        cmd(
            "channel.set_plugin_state",
            "channel.set_plugin_state <channel> <base64>",
            "Store a channel plugin's saved state (not undoable).",
            set_channel_state,
        ),
        cmd(
            "mixer.set_plugin_state",
            "mixer.set_plugin_state <track> <slot> <base64>",
            "Store a plugin effect's saved state (not undoable).",
            set_effect_state,
        ),
    ]
}

fn plugin_id<'a>(args: &Args<'a>, i: usize) -> Result<&'a str, CommandError> {
    let id = args.str(i)?;
    if id.is_empty() || id.chars().any(char::is_whitespace) {
        return Err(args.invalid(format!("`{id}` is not a plugin id")));
    }
    Ok(id)
}

/// The name argument from `i` on, or the plugin id's last component.
fn plugin_name(args: &Args<'_>, i: usize, id: &str) -> Result<String, CommandError> {
    if args.len() > i {
        return args.rest(i);
    }
    Ok(id.rsplit('.').next().unwrap_or(id).to_owned())
}

fn add_channel(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, usize::MAX)?;
    let id = plugin_id(args, 0)?;
    let name = plugin_name(args, 1, id)?;
    let channel_id = ChannelId(project.alloc_id());
    let instance = project.alloc_id();
    let plugin = PluginRef::new(id, name.clone(), instance);
    project.channels.push(Channel::new(
        channel_id,
        name.clone(),
        Instrument::Plugin { plugin },
    ));
    let number = project.channels.len();
    Ok(Outcome::undoable(
        format!("added channel {number} ({name})"),
        Invocation::new("channel.remove", [number.to_string()]),
    ))
}

fn add_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let track = track_arg(project, args, 0)?;
    let id = plugin_id(args, 1)?;
    let name = plugin_name(args, 2, id)?;
    if project.mixer[track].effects.len() >= MAX_EFFECT_SLOTS {
        return Err(args.invalid(format!("a track holds at most {MAX_EFFECT_SLOTS} effects")));
    }
    let instance = project.alloc_id();
    let effects = &mut project.mixer[track].effects;
    effects.push(EffectSlot::plugin(PluginRef::new(
        id,
        name.clone(),
        instance,
    )));
    let slot = effects.len();
    Ok(Outcome::undoable(
        format!("added {name} to mixer track {track} slot {slot}"),
        Invocation::new("mixer.remove_effect", [track.to_string(), slot.to_string()]),
    ))
}

fn channel_slot(project: &Project, args: &Args<'_>) -> Result<PluginSlot, CommandError> {
    let index = channel_arg(project, args, 0)?;
    if project.channels[index].instrument.plugin().is_none() {
        return Err(args.invalid(format!("channel {} is not a plugin", index + 1)));
    }
    Ok(PluginSlot::Channel(index))
}

fn effect_slot(project: &Project, args: &Args<'_>) -> Result<PluginSlot, CommandError> {
    let track = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, track, 1)?;
    if project.mixer[track].effects[slot].plugin.is_none() {
        return Err(args.invalid(format!(
            "slot {} of mixer track {track} is not a plugin",
            slot + 1
        )));
    }
    Ok(PluginSlot::Effect(track, slot))
}

/// Arguments that name `slot` again, for inverse invocations.
fn slot_args(slot: PluginSlot) -> Vec<String> {
    match slot {
        PluginSlot::Channel(c) => vec![(c + 1).to_string()],
        PluginSlot::Effect(t, s) => vec![t.to_string(), (s + 1).to_string()],
    }
}

fn format_value(value: Option<f64>) -> String {
    value.map_or_else(|| UNSET.to_owned(), |v| v.to_string())
}

fn set_param(
    project: &mut Project,
    args: &Args<'_>,
    slot: PluginSlot,
    first: usize,
) -> Result<Outcome, CommandError> {
    let id: u32 = args.parse(first, "parameter id")?;
    let raw = args.str(first + 1)?;
    let value = if raw == UNSET {
        None
    } else {
        let value: f64 = args.parse(first + 1, "value")?;
        if !value.is_finite() {
            return Err(args.invalid(format!("`{raw}` is not a finite number")));
        }
        Some(value)
    };
    let Some(plugin) = project.plugin_mut(slot) else {
        return Err(args.invalid("no plugin there"));
    };
    let previous = match value {
        Some(v) => plugin.params.insert(id, v),
        None => plugin.params.remove(&id),
    };
    let message = format!("{} parameter {id} = {}", plugin.name, format_value(value));
    if previous == value {
        return Ok(Outcome::done(message));
    }
    let mut inverse = slot_args(slot);
    inverse.push(id.to_string());
    inverse.push(format_value(previous));
    Ok(Outcome::undoable(
        message,
        Invocation::new(args.name(), inverse),
    ))
}

fn set_channel_param(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let slot = channel_slot(project, args)?;
    set_param(project, args, slot, 1)
}

fn set_effect_param(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let slot = effect_slot(project, args)?;
    set_param(project, args, slot, 2)
}

fn set_state(
    project: &mut Project,
    args: &Args<'_>,
    slot: PluginSlot,
    i: usize,
) -> Result<Outcome, CommandError> {
    let state = args.str(i)?;
    let Some(bytes) = base64::decode(state) else {
        return Err(args.invalid("the state is not valid base64"));
    };
    let Some(plugin) = project.plugin_mut(slot) else {
        return Err(args.invalid("no plugin there"));
    };
    plugin.state = base64::encode(&bytes);
    Ok(Outcome::done(format!(
        "stored {} bytes of {} state",
        bytes.len(),
        plugin.name
    )))
}

fn set_channel_state(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let slot = channel_slot(project, args)?;
    set_state(project, args, slot, 1)
}

fn set_effect_state(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let slot = effect_slot(project, args)?;
    set_state(project, args, slot, 2)
}
