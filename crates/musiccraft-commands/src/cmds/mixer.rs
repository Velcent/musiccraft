use musiccraft_project::params::{
    EFFECT_KINDS, EffectKind, MAX_EFFECT_SLOTS, MAX_MIXER_TRACKS, index_of,
};
use musiccraft_project::{
    ChannelId, EffectSlot, MAX_SENDS, MixerSend, MixerTrack, Project, ProjectError, check_pan,
    check_volume,
};
use serde::{Deserialize, Serialize};

use crate::args::{Args, num, on_off};
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "mixer.add_track",
            "mixer.add_track [name]",
            "Add an insert track at the end of the mixer.",
            add_track,
        ),
        cmd(
            "mixer.remove_track",
            "mixer.remove_track <track>",
            "Remove an insert track; its channels go to the master.",
            remove_track,
        ),
        cmd(
            "mixer.insert_track",
            "mixer.insert_track <track> <json>",
            "Insert a mixer track from its JSON form (used by undo).",
            insert_track,
        ),
        cmd(
            "mixer.rename",
            "mixer.rename <track> <name>",
            "Rename a mixer track.",
            rename,
        ),
        cmd(
            "mixer.set_volume",
            "mixer.set_volume <track> <0-1.25>",
            "Set a mixer track's volume (linear gain).",
            set_volume,
        ),
        cmd(
            "mixer.set_pan",
            "mixer.set_pan <track> <-1..1>",
            "Set a mixer track's pan.",
            set_pan,
        ),
        cmd(
            "mixer.set_mute",
            "mixer.set_mute <track> <on|off>",
            "Mute or unmute a mixer track.",
            set_mute,
        ),
        cmd(
            "mixer.toggle_mute",
            "mixer.toggle_mute <track>",
            "Toggle a mixer track's mute.",
            toggle_mute,
        ),
        cmd(
            "mixer.set_solo",
            "mixer.set_solo <track> <on|off>",
            "Solo or unsolo a mixer track.",
            set_solo,
        ),
        cmd(
            "mixer.toggle_solo",
            "mixer.toggle_solo <track>",
            "Toggle a mixer track's solo.",
            toggle_solo,
        ),
        cmd(
            "mixer.set_send",
            "mixer.set_send <track> <target track> <level 0-1.25>",
            "Send an insert to a higher-numbered insert (level 0 removes the send).",
            set_send,
        ),
        cmd(
            "mixer.route_to_master",
            "mixer.route_to_master <track> <on|off>",
            "Choose whether an insert feeds the master directly.",
            route_to_master,
        ),
        cmd(
            "mixer.add_effect",
            "mixer.add_effect <track> <kind>",
            "Append an effect (filter, eq, delay, reverb, compressor, distortion).",
            add_effect,
        ),
        cmd(
            "mixer.remove_effect",
            "mixer.remove_effect <track> <slot>",
            "Remove the effect in a slot.",
            remove_effect,
        ),
        cmd(
            "mixer.insert_effect",
            "mixer.insert_effect <track> <slot> <json>",
            "Insert an effect from its JSON form (used by undo).",
            insert_effect,
        ),
        cmd(
            "mixer.move_effect",
            "mixer.move_effect <track> <slot> <new slot>",
            "Reorder an effect within a track.",
            move_effect,
        ),
        cmd(
            "mixer.set_effect_param",
            "mixer.set_effect_param <track> <slot> <param> <value>",
            "Set an effect parameter.",
            set_effect_param,
        ),
        cmd(
            "mixer.set_effect_enabled",
            "mixer.set_effect_enabled <track> <slot> <on|off>",
            "Enable or bypass an effect.",
            set_effect_enabled,
        ),
        cmd(
            "mixer.set_effect_file",
            "mixer.set_effect_file <track> <slot> <file|ir:name|->",
            "Load the convolution reverb's impulse response: an audio file or a generated one (ir:room, ir:chamber, ir:plate, ir:hall, ir:cathedral); - restores the default.",
            set_effect_file,
        ),
        cmd(
            "mixer.toggle_effect",
            "mixer.toggle_effect <track> <slot>",
            "Toggle an effect's bypass.",
            toggle_effect,
        ),
    ]
}

/// Sidechain sources of every effect that has one, as (track, slot,
/// parameter index, source).
fn sidechains(project: &Project) -> Vec<(usize, usize, usize, usize)> {
    let mut out = Vec::new();
    for (t, track) in project.mixer.iter().enumerate() {
        for (s, effect) in track.effects.iter().enumerate() {
            let Some(index) = effect.kind.sidechain_index() else {
                continue;
            };
            let source = effect.values.get(index).copied().unwrap_or(0.0).round() as usize;
            if source > 0 {
                out.push((t, s, index, source));
            }
        }
    }
    out
}

/// Shifts sidechain sources at or after `from` by `delta`.
fn shift_sidechains(project: &mut Project, from: usize, delta: isize) {
    for (t, s, index, source) in sidechains(project) {
        if source >= from {
            project.mixer[t].effects[s].values[index] = source.saturating_add_signed(delta) as f32;
        }
    }
}

pub(super) fn track_arg(
    project: &Project,
    args: &Args<'_>,
    i: usize,
) -> Result<usize, CommandError> {
    Ok(project.mixer_index(args.parse(i, "mixer track")?)?)
}

fn insert_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    let track = track_arg(project, args, i)?;
    if track == 0 {
        return Err(args.invalid("this applies to inserts, not the master"));
    }
    Ok(track)
}

pub(super) fn slot_arg(
    project: &Project,
    args: &Args<'_>,
    track: usize,
    i: usize,
) -> Result<usize, CommandError> {
    let slot = args.number(i, "effect slot")?;
    if slot > project.mixer[track].effects.len() {
        return Err(ProjectError::NotFound {
            what: "effect slot",
            number: slot,
        }
        .into());
    }
    Ok(slot - 1)
}

fn add_track(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    if project.mixer.len() >= MAX_MIXER_TRACKS {
        return Err(args.invalid(format!("the mixer is limited to {MAX_MIXER_TRACKS} tracks")));
    }
    let number = project.mixer.len();
    let name = if args.len() > 0 {
        args.rest(0)?
    } else {
        format!("Insert {number}")
    };
    project.mixer.push(MixerTrack::new(name.clone()));
    Ok(Outcome::undoable(
        format!("added mixer track {number} ({name})"),
        Invocation::new("mixer.remove_track", [number.to_string()]),
    ))
}

#[derive(Serialize, Deserialize)]
struct TrackSnapshot {
    track: MixerTrack,
    routed: Vec<ChannelId>,
    /// Sends from other (lower) tracks into this one: (source track, level).
    #[serde(default)]
    incoming: Vec<(usize, f32)>,
}

fn remove_track(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = insert_arg(project, args, 0)?;
    if super::automation::targets_track(project, index, None) {
        return Err(args.invalid("an automation drives this track; delete the automation first"));
    }
    if sidechains(project)
        .iter()
        .any(|(_, _, _, source)| *source == index)
    {
        return Err(args.invalid("an effect uses this track as its sidechain; change it first"));
    }
    shift_sidechains(project, index + 1, -1);
    let track = project.mixer.remove(index);
    super::automation::shift_tracks(project, index + 1, -1);
    let mut incoming = Vec::new();
    for (source, other) in project.mixer.iter_mut().enumerate() {
        other.sends.retain(|send| {
            let keep = send.target != index;
            if !keep {
                incoming.push((source, send.level));
            }
            keep
        });
        for send in &mut other.sends {
            if send.target > index {
                send.target -= 1;
            }
        }
    }
    let mut routed = Vec::new();
    for channel in &mut project.channels {
        if channel.mixer_track == index {
            channel.mixer_track = 0;
            routed.push(channel.id);
        } else if channel.mixer_track > index {
            channel.mixer_track -= 1;
        }
    }
    let name = track.name.clone();
    let json = serde_json::to_string(&TrackSnapshot {
        track,
        routed,
        incoming,
    })
    .map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("removed mixer track {index} ({name})"),
        Invocation::new("mixer.insert_track", [index.to_string(), json]),
    ))
}

fn insert_track(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index: usize = args.parse(0, "mixer track")?;
    if index == 0 || index > project.mixer.len() {
        return Err(args.invalid(format!("cannot insert a track at {index}")));
    }
    if project.mixer.len() >= MAX_MIXER_TRACKS {
        return Err(args.invalid(format!("the mixer is limited to {MAX_MIXER_TRACKS} tracks")));
    }
    let snapshot: TrackSnapshot =
        serde_json::from_str(args.str(1)?).map_err(|e| args.invalid(e.to_string()))?;
    for channel in &mut project.channels {
        if channel.mixer_track >= index {
            channel.mixer_track += 1;
        }
        if snapshot.routed.contains(&channel.id) {
            channel.mixer_track = index;
        }
    }
    for other in &mut project.mixer {
        for send in &mut other.sends {
            if send.target >= index {
                send.target += 1;
            }
        }
    }
    for (source, level) in &snapshot.incoming {
        if let Some(other) = project.mixer.get_mut(*source) {
            other.sends.push(MixerSend {
                target: index,
                level: *level,
            });
            other.sends.sort_by_key(|send| send.target);
        }
    }
    let name = snapshot.track.name.clone();
    super::automation::shift_tracks(project, index, 1);
    shift_sidechains(project, index, 1);
    project.mixer.insert(index, snapshot.track);
    Ok(Outcome::undoable(
        format!("inserted mixer track {index} ({name})"),
        Invocation::new("mixer.remove_track", [index.to_string()]),
    ))
}

fn rename(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let index = track_arg(project, args, 0)?;
    let name = args.rest(1)?;
    let previous = std::mem::replace(&mut project.mixer[index].name, name.clone());
    Ok(Outcome::undoable(
        format!("mixer track {index} renamed to {name}"),
        Invocation::new(args.name(), [index.to_string(), previous]),
    ))
}

/// Replaces a track field; `raw` formats a value as a command argument.
fn set_track_field<T: PartialEq + Copy>(
    project: &mut Project,
    args: &Args<'_>,
    index: usize,
    value: T,
    field: fn(&mut MixerTrack) -> &mut T,
    raw: fn(T) -> String,
    label: &str,
) -> Outcome {
    let previous = std::mem::replace(field(&mut project.mixer[index]), value);
    let message = format!("mixer track {index} {label} {}", raw(value));
    if previous == value {
        Outcome::done(message)
    } else {
        Outcome::undoable(
            message,
            Invocation::new(args.name(), [index.to_string(), raw(previous)]),
        )
    }
}

fn set_volume(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let value = check_volume(args.f32(1, "volume")?)?;
    Ok(set_track_field(
        project,
        args,
        index,
        value,
        |t| &mut t.volume,
        num,
        "volume",
    ))
}

fn set_pan(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let value = check_pan(args.f32(1, "pan")?)?;
    Ok(set_track_field(
        project,
        args,
        index,
        value,
        |t| &mut t.pan,
        num,
        "pan",
    ))
}

fn flag_text(value: bool) -> String {
    on_off(value).to_owned()
}

fn set_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let value = args.flag(1)?;
    Ok(set_track_field(
        project,
        args,
        index,
        value,
        |t| &mut t.mute,
        flag_text,
        "mute",
    ))
}

fn toggle_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = track_arg(project, args, 0)?;
    let track = &mut project.mixer[index];
    track.mute = !track.mute;
    Ok(Outcome::undoable(
        format!("mixer track {index} mute {}", on_off(track.mute)),
        Invocation::new(args.name(), [index.to_string()]),
    ))
}

fn set_solo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let value = args.flag(1)?;
    Ok(set_track_field(
        project,
        args,
        index,
        value,
        |t| &mut t.solo,
        flag_text,
        "solo",
    ))
}

fn toggle_solo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = track_arg(project, args, 0)?;
    let track = &mut project.mixer[index];
    track.solo = !track.solo;
    Ok(Outcome::undoable(
        format!("mixer track {index} solo {}", on_off(track.solo)),
        Invocation::new(args.name(), [index.to_string()]),
    ))
}

fn add_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let raw = args.str(1)?;
    let kind = EffectKind::parse(raw).ok_or_else(|| {
        let names: Vec<_> = EFFECT_KINDS.iter().map(|k| k.name()).collect();
        args.invalid(format!(
            "unknown effect `{raw}`; choose from {}",
            names.join(", ")
        ))
    })?;
    let effects = &mut project.mixer[index].effects;
    if effects.len() >= MAX_EFFECT_SLOTS {
        return Err(args.invalid(format!("a track holds at most {MAX_EFFECT_SLOTS} effects")));
    }
    effects.push(EffectSlot::new(kind));
    let slot = effects.len();
    Ok(Outcome::undoable(
        format!("added {kind} to mixer track {index} slot {slot}"),
        Invocation::new("mixer.remove_effect", [index.to_string(), slot.to_string()]),
    ))
}

fn remove_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, index, 1)?;
    if super::automation::targets_track(project, index, Some(slot)) {
        return Err(args.invalid("an automation drives this effect; delete the automation first"));
    }
    let effect = project.mixer[index].effects.remove(slot);
    super::automation::remap_slots(project, index, |s| Some(if s > slot { s - 1 } else { s }));
    let json = serde_json::to_string(&effect).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("removed {} from mixer track {index}", effect.label()),
        Invocation::new(
            "mixer.insert_effect",
            [index.to_string(), (slot + 1).to_string(), json],
        ),
    ))
}

fn insert_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = track_arg(project, args, 0)?;
    let slot = args.number(1, "effect slot")?;
    let effects = &project.mixer[index].effects;
    if slot > effects.len() + 1 || effects.len() >= MAX_EFFECT_SLOTS {
        return Err(args.invalid(format!("cannot insert an effect at slot {slot}")));
    }
    let effect: EffectSlot =
        serde_json::from_str(args.str(2)?).map_err(|e| args.invalid(e.to_string()))?;
    if let Some(plugin) = &effect.plugin {
        if project.plugin_slot(plugin.instance).is_some() {
            return Err(args.invalid(format!(
                "plugin instance {} already exists",
                plugin.instance
            )));
        }
        project.reserve_id(plugin.instance);
    }
    super::automation::remap_slots(project, index, |s| {
        Some(if s >= slot - 1 { s + 1 } else { s })
    });
    let label = effect.label();
    project.mixer[index].effects.insert(slot - 1, effect);
    Ok(Outcome::undoable(
        format!("inserted {label} on mixer track {index}"),
        Invocation::new("mixer.remove_effect", [index.to_string(), slot.to_string()]),
    ))
}

fn move_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = track_arg(project, args, 0)?;
    let from = slot_arg(project, args, index, 1)?;
    let to = slot_arg(project, args, index, 2)?;
    let effects = &mut project.mixer[index].effects;
    let effect = effects.remove(from);
    effects.insert(to, effect);
    super::automation::remap_slots(project, index, |s| {
        Some(if s == from {
            to
        } else if from < to && s > from && s <= to {
            s - 1
        } else if to < from && s >= to && s < from {
            s + 1
        } else {
            s
        })
    });
    Ok(Outcome::undoable(
        format!("moved effect to slot {}", to + 1),
        Invocation::new(
            args.name(),
            [
                index.to_string(),
                (to + 1).to_string(),
                (from + 1).to_string(),
            ],
        ),
    ))
}

fn set_effect_param(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let index = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, index, 1)?;
    let project_tracks = project.mixer.len();
    let effect = &mut project.mixer[index].effects[slot];
    let specs = effect.kind.params();
    let name = args.str(2)?;
    let param = index_of(specs, name).ok_or_else(|| {
        let names: Vec<_> = specs.iter().map(|p| p.name).collect();
        args.invalid(format!(
            "{} has no parameter `{name}`; choose from {}",
            effect.kind,
            names.join(", ")
        ))
    })?;
    let spec = specs[param];
    let value = args.f32(3, name)?;
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
    if effect.kind.sidechain_index() == Some(param) && value >= 0.5 {
        let source = value as usize;
        let valid = source < project_tracks && source != index && (index == 0 || source < index);
        if !valid {
            return Err(args.invalid(
                "a sidechain comes from a lower-numbered insert (or any insert for the master)",
            ));
        }
    }
    let previous = std::mem::replace(&mut effect.values[param], value);
    let message = format!("{} {name} {}", effect.kind, num(value));
    if previous == value {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [
                index.to_string(),
                (slot + 1).to_string(),
                name.to_owned(),
                num(previous),
            ],
        ),
    ))
}

fn set_effect_enabled(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, index, 1)?;
    let enabled = args.flag(2)?;
    let effect = &mut project.mixer[index].effects[slot];
    let previous = std::mem::replace(&mut effect.enabled, enabled);
    let message = format!(
        "{} {}",
        effect.kind,
        if enabled { "enabled" } else { "bypassed" }
    );
    if previous == enabled {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [
                index.to_string(),
                (slot + 1).to_string(),
                on_off(previous).to_owned(),
            ],
        ),
    ))
}

fn set_effect_file(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, index, 1)?;
    let file = args.str(2)?;
    if project.mixer[index].effects[slot].kind != EffectKind::Convolution {
        return Err(args.invalid("only the convolution reverb loads a file"));
    }
    let new = match file {
        "-" => None,
        _ => {
            if let Some(name) = file.strip_prefix("ir:") {
                if !musiccraft_project::params::IMPULSE_RESPONSES.contains(&name) {
                    return Err(args.invalid(format!(
                        "unknown impulse response `{name}`; choose from {}",
                        musiccraft_project::params::IMPULSE_RESPONSES.join(", ")
                    )));
                }
            } else {
                super::channel::check_sample(project, args, file)?;
            }
            Some(file.to_owned())
        }
    };
    let effect = &mut project.mixer[index].effects[slot];
    let previous = std::mem::replace(&mut effect.file, new);
    Ok(Outcome::undoable(
        format!("{} loads {file}", effect.kind),
        Invocation::new(
            args.name(),
            [
                index.to_string(),
                (slot + 1).to_string(),
                previous.unwrap_or_else(|| "-".to_owned()),
            ],
        ),
    ))
}

fn toggle_effect(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = track_arg(project, args, 0)?;
    let slot = slot_arg(project, args, index, 1)?;
    let effect = &mut project.mixer[index].effects[slot];
    effect.enabled = !effect.enabled;
    Ok(Outcome::undoable(
        format!(
            "{} {}",
            effect.kind,
            if effect.enabled {
                "enabled"
            } else {
                "bypassed"
            }
        ),
        Invocation::new(args.name(), [index.to_string(), (slot + 1).to_string()]),
    ))
}

fn set_send(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = insert_arg(project, args, 0)?;
    let target = track_arg(project, args, 1)?;
    if target <= index {
        return Err(args.invalid("sends go to a higher-numbered insert"));
    }
    let level = check_volume(args.f32(2, "level")?)?;
    let sends = &mut project.mixer[index].sends;
    let previous = sends
        .iter()
        .find(|s| s.target == target)
        .map_or(0.0, |s| s.level);
    sends.retain(|s| s.target != target);
    if level > 0.0 {
        if sends.len() >= MAX_SENDS {
            return Err(args.invalid(format!("a track has at most {MAX_SENDS} sends")));
        }
        sends.push(MixerSend { target, level });
        sends.sort_by_key(|s| s.target);
    }
    let message = if level > 0.0 {
        format!("mixer track {index} sends {} to {target}", num(level))
    } else {
        format!("mixer track {index} no longer sends to {target}")
    };
    if (previous - level).abs() < f32::EPSILON {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [index.to_string(), target.to_string(), num(previous)],
        ),
    ))
}

fn route_to_master(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = insert_arg(project, args, 0)?;
    let value = args.flag(1)?;
    Ok(set_track_field(
        project,
        args,
        index,
        value,
        |t| &mut t.to_master,
        flag_text,
        "to master",
    ))
}
