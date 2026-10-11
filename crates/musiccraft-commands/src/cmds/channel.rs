use std::path::Path;

use musiccraft_project::params::{BUILTIN_PREFIX, BUILTIN_SAMPLES, INSTRUMENT_PARAMS, index_of};
use musiccraft_project::{
    Channel, ChannelId, Instrument, Note, PatternId, Project, check_pan, check_volume,
};
use serde::{Deserialize, Serialize};

use crate::args::{Args, num, on_off};
use crate::command::cmd;
use crate::{BATCH, Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "channel.add_sampler",
            "channel.add_sampler <sample> [name]",
            "Add a sampler channel playing a file or builtin:<sound>.",
            add_sampler,
        ),
        cmd(
            "channel.add_synth",
            "channel.add_synth [name]",
            "Add a subtractive synth channel.",
            add_synth,
        ),
        cmd(
            "channel.add_drum",
            "channel.add_drum [kick|snare|hat|tom|clap] [name]",
            "Add a drum synth channel.",
            add_drum,
        ),
        cmd(
            "channel.add_pluck",
            "channel.add_pluck [name]",
            "Add a plucked-string channel.",
            add_pluck,
        ),
        cmd(
            "channel.add_fm",
            "channel.add_fm [name]",
            "Add a four-operator FM synth channel.",
            add_fm,
        ),
        cmd(
            "channel.add_keys",
            "channel.add_keys [piano|epiano|organ] [name]",
            "Add a keys channel (piano, electric piano or organ).",
            add_keys,
        ),
        cmd(
            "channel.add_additive",
            "channel.add_additive [name]",
            "Add an additive (harmonic) synth channel.",
            add_additive,
        ),
        cmd(
            "channel.add_granular",
            "channel.add_granular <sample> [name]",
            "Add a channel that plays a sample as grains.",
            add_granular,
        ),
        cmd(
            "channel.add_kit",
            "channel.add_kit [name]",
            "Add a 16-pad drum kit (pads 1-8 start with the built-in sounds).",
            add_kit,
        ),
        cmd(
            "channel.set_pad",
            "channel.set_pad <channel> <pad 1-16> <sample|->",
            "Load a sample onto a kit pad (- clears it).",
            set_pad,
        ),
        cmd(
            "channel.add_layer",
            "channel.add_layer [name]",
            "Add a layer channel that plays other channels.",
            add_layer,
        ),
        cmd(
            "channel.set_layer",
            "channel.set_layer <channel> <channel,channel,...|->",
            "Set the channels a layer plays.",
            set_layer,
        ),
        cmd(
            "channel.remove",
            "channel.remove <channel>",
            "Remove a channel and its notes in every pattern.",
            remove,
        ),
        cmd(
            "channel.insert",
            "channel.insert <position> <json>",
            "Insert a channel from its JSON form (used by undo).",
            insert,
        ),
        cmd(
            "channel.clone",
            "channel.clone <channel>",
            "Duplicate a channel's settings below it.",
            clone,
        ),
        cmd(
            "channel.move",
            "channel.move <channel> <position>",
            "Move a channel to another position in the rack.",
            move_channel,
        ),
        cmd(
            "channel.rename",
            "channel.rename <channel> <name>",
            "Rename a channel.",
            rename,
        ),
        cmd(
            "channel.set_volume",
            "channel.set_volume <channel> <0-1.25>",
            "Set a channel's volume (linear gain).",
            set_volume,
        ),
        cmd(
            "channel.set_pan",
            "channel.set_pan <channel> <-1..1>",
            "Set a channel's pan.",
            set_pan,
        ),
        cmd(
            "channel.set_mute",
            "channel.set_mute <channel> <on|off>",
            "Mute or unmute a channel.",
            set_mute,
        ),
        cmd(
            "channel.toggle_mute",
            "channel.toggle_mute <channel>",
            "Toggle a channel's mute.",
            toggle_mute,
        ),
        cmd(
            "channel.solo",
            "channel.solo <channel>",
            "Mute every other channel; if already solo, unmute all.",
            solo,
        ),
        cmd(
            "channel.route",
            "channel.route <channel> <mixer track>",
            "Send a channel to a mixer track (0 = master).",
            route,
        ),
        cmd(
            "channel.set_sample",
            "channel.set_sample <channel> <sample>",
            "Change the sample a sampler channel plays.",
            set_sample,
        ),
        cmd(
            "channel.set_param",
            "channel.set_param <channel> <param> <value>",
            "Set an instrument parameter (attack, cutoff, root_key, ...).",
            set_param,
        ),
    ]
}

/// Index of the channel number in argument `i`.
pub(super) fn channel_arg(
    project: &Project,
    args: &Args<'_>,
    i: usize,
) -> Result<usize, CommandError> {
    Ok(project.channel_index(args.number(i, "channel")?)?)
}

/// Checks a sample reference and returns a default channel name for it.
pub(super) fn check_sample(
    project: &Project,
    args: &Args<'_>,
    sample: &str,
) -> Result<String, CommandError> {
    if let Some(name) = sample.strip_prefix(BUILTIN_PREFIX) {
        if !BUILTIN_SAMPLES.contains(&name) {
            return Err(args.invalid(format!(
                "unknown built-in sound `{name}`; choose from {}",
                BUILTIN_SAMPLES.join(", ")
            )));
        }
        let mut label = name.replace('_', " ");
        if let Some(first) = label.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        return Ok(label);
    }
    let path = Path::new(sample);
    let exists = path.exists()
        || project
            .path()
            .and_then(Path::parent)
            .is_some_and(|dir| dir.join(path).exists());
    if !exists {
        return Err(args.invalid(format!("sample file `{sample}` does not exist")));
    }
    Ok(path
        .file_stem()
        .map_or_else(|| sample.to_owned(), |s| s.to_string_lossy().into_owned()))
}

fn push_channel(project: &mut Project, channel: Channel) -> Outcome {
    let name = channel.name.clone();
    project.channels.push(channel);
    let number = project.channels.len();
    Outcome::undoable(
        format!("added channel {number} ({name})"),
        Invocation::new("channel.remove", [number.to_string()]),
    )
}

fn add_sampler(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, usize::MAX)?;
    let sample = args.str(0)?;
    let default_name = check_sample(project, args, sample)?;
    let name = if args.len() > 1 {
        args.rest(1)?
    } else {
        default_name
    };
    let id = ChannelId(project.alloc_id());
    let instrument = Instrument::Sampler {
        sample: sample.to_owned(),
    };
    Ok(push_channel(project, Channel::new(id, name, instrument)))
}

fn add_synth(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = if args.len() > 0 {
        args.rest(0)?
    } else {
        format!("Synth {}", project.channels.len() + 1)
    };
    let id = ChannelId(project.alloc_id());
    Ok(push_channel(
        project,
        Channel::new(id, name, Instrument::Synth),
    ))
}

fn add_drum(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    use musiccraft_project::params::DRUM_TYPES;
    let (kind, rest) = match args.opt(0) {
        Some(word) => match DRUM_TYPES.iter().position(|t| *t == word) {
            Some(i) => (i, 1),
            None => (0, 0),
        },
        None => (0, 0),
    };
    let name = if args.len() > rest {
        args.rest(rest)?
    } else {
        let mut label = DRUM_TYPES[kind].to_owned();
        if let Some(first) = label.get_mut(0..1) {
            first.make_ascii_uppercase();
        }
        label
    };
    let id = ChannelId(project.alloc_id());
    let mut channel = Channel::new(id, name, Instrument::DrumSynth);
    channel.params.0[musiccraft_project::params::inst::DRUM_TYPE] = kind as f32;
    Ok(push_channel(project, channel))
}

fn add_pluck(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = if args.len() > 0 {
        args.rest(0)?
    } else {
        "Pluck".to_owned()
    };
    let id = ChannelId(project.alloc_id());
    Ok(push_channel(
        project,
        Channel::new(id, name, Instrument::Pluck),
    ))
}

fn name_or(args: &Args<'_>, from: usize, default: &str) -> Result<String, CommandError> {
    if args.len() > from {
        args.rest(from)
    } else {
        Ok(default.to_owned())
    }
}

fn add_simple(
    project: &mut Project,
    args: &Args<'_>,
    instrument: Instrument,
    default: &str,
) -> Result<Outcome, CommandError> {
    let name = name_or(args, 0, default)?;
    let id = ChannelId(project.alloc_id());
    Ok(push_channel(project, Channel::new(id, name, instrument)))
}

fn add_fm(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    add_simple(project, args, Instrument::Fm, "FM")
}

fn add_additive(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    add_simple(project, args, Instrument::Additive, "Additive")
}

fn add_layer(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    add_simple(
        project,
        args,
        Instrument::Layer {
            channels: Vec::new(),
        },
        "Layer",
    )
}

fn add_keys(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    use musiccraft_project::params::KEYS_TYPES;
    let kind = args
        .opt(0)
        .and_then(|w| KEYS_TYPES.iter().position(|t| *t == w));
    let from = usize::from(kind.is_some());
    let kind = kind.unwrap_or(0);
    let default = ["Piano", "E-piano", "Organ"][kind];
    let name = name_or(args, from, default)?;
    let id = ChannelId(project.alloc_id());
    let mut channel = Channel::new(id, name, Instrument::Keys);
    channel.params.0[musiccraft_project::params::inst::KEYS_TYPE] = kind as f32;
    if kind == 2 {
        // Organs hold at full level while the key is down.
        channel.params.0[musiccraft_project::params::inst::SUSTAIN] = 1.0;
        channel.params.0[musiccraft_project::params::inst::RELEASE] = 0.05;
    }
    Ok(push_channel(project, channel))
}

fn add_granular(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, usize::MAX)?;
    let sample = args.str(0)?;
    let default_name = check_sample(project, args, sample)?;
    let name = name_or(args, 1, &format!("{default_name} grains"))?;
    let id = ChannelId(project.alloc_id());
    let mut channel = Channel::new(
        id,
        name,
        Instrument::Granular {
            sample: sample.to_owned(),
        },
    );
    channel.params.0[musiccraft_project::params::inst::ATTACK] = 0.05;
    channel.params.0[musiccraft_project::params::inst::SUSTAIN] = 1.0;
    channel.params.0[musiccraft_project::params::inst::RELEASE] = 0.3;
    Ok(push_channel(project, channel))
}

fn add_kit(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = name_or(args, 0, "Kit")?;
    let mut pads: Vec<String> = BUILTIN_SAMPLES
        .iter()
        .map(|s| format!("{BUILTIN_PREFIX}{s}"))
        .collect();
    pads.resize(16, String::new());
    let id = ChannelId(project.alloc_id());
    Ok(push_channel(
        project,
        Channel::new(id, name, Instrument::Kit { pads }),
    ))
}

fn set_pad(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = channel_arg(project, args, 0)?;
    let pad: usize = args.number(1, "pad")?;
    if !(1..=16).contains(&pad) {
        return Err(args.invalid("pads are numbered 1 to 16"));
    }
    let sample = args.str(2)?;
    let sample = if sample == "-" {
        String::new()
    } else {
        check_sample(project, args, sample)?;
        sample.to_owned()
    };
    let Instrument::Kit { pads } = &mut project.channels[index].instrument else {
        return Err(args.invalid(format!("channel {} is not a kit", index + 1)));
    };
    pads.resize(16, String::new());
    let previous = std::mem::replace(&mut pads[pad - 1], sample.clone());
    let shown = if sample.is_empty() {
        "nothing"
    } else {
        &sample
    };
    Ok(Outcome::undoable(
        format!("pad {pad} plays {shown}"),
        Invocation::new(
            args.name(),
            [
                (index + 1).to_string(),
                pad.to_string(),
                if previous.is_empty() {
                    "-".to_owned()
                } else {
                    previous
                },
            ],
        ),
    ))
}

fn set_layer(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let list = args.str(1)?;
    let mut children = Vec::new();
    if list != "-" {
        for part in list.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let number: usize = part
                .parse()
                .map_err(|_| args.invalid(format!("`{part}` is not a channel number")))?;
            let child = project.channel_index(number)?;
            if child == index {
                return Err(args.invalid("a layer cannot play itself"));
            }
            if matches!(project.channels[child].instrument, Instrument::Layer { .. }) {
                return Err(args.invalid("layers cannot play other layers"));
            }
            children.push(project.channels[child].id);
        }
    }
    let positions: Vec<String> = children
        .iter()
        .filter_map(|id| project.channel_position(*id))
        .map(|i| (i + 1).to_string())
        .collect();
    let Instrument::Layer { channels } = &mut project.channels[index].instrument else {
        return Err(args.invalid(format!("channel {} is not a layer", index + 1)));
    };
    let previous = std::mem::replace(channels, children);
    let previous: Vec<String> = previous
        .iter()
        .filter_map(|id| project.channel_position(*id))
        .map(|i| (i + 1).to_string())
        .collect();
    Ok(Outcome::undoable(
        format!(
            "layer plays channels {}",
            if positions.is_empty() {
                "none".to_owned()
            } else {
                positions.join(", ")
            }
        ),
        Invocation::new(
            args.name(),
            [
                (index + 1).to_string(),
                if previous.is_empty() {
                    "-".to_owned()
                } else {
                    previous.join(",")
                },
            ],
        ),
    ))
}

/// A channel with all of its notes, as stored by `channel.remove` for undo.
#[derive(Serialize, Deserialize)]
struct ChannelSnapshot {
    channel: Channel,
    notes: Vec<(PatternId, Note)>,
    /// Audio clips that played this channel.
    #[serde(default)]
    clips: Vec<musiccraft_project::Clip>,
}

fn remove(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = channel_arg(project, args, 0)?;
    let channel = project.channels.remove(index);
    let mut notes = Vec::new();
    for pattern in &mut project.patterns {
        let id = pattern.id;
        pattern.notes.retain(|note| {
            let keep = note.channel != channel.id;
            if !keep {
                notes.push((id, *note));
            }
            keep
        });
    }
    let (clips, kept): (Vec<_>, Vec<_>) = project
        .playlist
        .clips
        .drain(..)
        .partition(|c| c.audio() == Some(channel.id));
    project.playlist.clips = kept;
    let name = channel.name.clone();
    let json = serde_json::to_string(&ChannelSnapshot {
        channel,
        notes,
        clips,
    })
    .map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("removed channel {} ({name})", index + 1),
        Invocation::new("channel.insert", [(index + 1).to_string(), json]),
    ))
}

fn insert(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let position: usize = args.number(0, "position")?;
    if position > project.channels.len() + 1 {
        return Err(args.invalid(format!("position {position} is past the end of the rack")));
    }
    let snapshot: ChannelSnapshot =
        serde_json::from_str(args.str(1)?).map_err(|e| args.invalid(e.to_string()))?;
    let mut channel = snapshot.channel;
    if project.channel_position(channel.id).is_some() {
        return Err(args.invalid(format!("channel id {} already exists", channel.id.0)));
    }
    if channel.mixer_track >= project.mixer.len() {
        channel.mixer_track = 0;
    }
    if let Some(plugin) = channel.instrument.plugin() {
        if project.plugin_slot(plugin.instance).is_some() {
            return Err(args.invalid(format!(
                "plugin instance {} already exists",
                plugin.instance
            )));
        }
        project.reserve_id(plugin.instance);
    }
    project.reserve_id(channel.id.0);
    for (pattern_id, note) in snapshot.notes {
        if let Some(i) = project.pattern_position(pattern_id) {
            project.patterns[i].put_note(note);
        }
    }
    let name = channel.name.clone();
    project.channels.insert(position - 1, channel);
    project.playlist.clips.extend(snapshot.clips);
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!("inserted channel {position} ({name})"),
        Invocation::new("channel.remove", [position.to_string()]),
    ))
}

fn clone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = channel_arg(project, args, 0)?;
    let mut copy = project.channels[index].clone();
    copy.id = ChannelId(project.alloc_id());
    if let Instrument::Plugin { plugin } = &mut copy.instrument {
        // The copy runs its own instance, starting from the stored state.
        plugin.instance = project.alloc_id();
    }
    copy.name = format!("{} copy", copy.name);
    let position = index + 2;
    project.channels.insert(index + 1, copy);
    Ok(Outcome::undoable(
        format!("cloned channel {} to {position}", index + 1),
        Invocation::new("channel.remove", [position.to_string()]),
    ))
}

fn move_channel(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let from = channel_arg(project, args, 0)?;
    let to = channel_arg(project, args, 1)?;
    let channel = project.channels.remove(from);
    project.channels.insert(to, channel);
    Ok(Outcome::undoable(
        format!("moved channel {} to {}", from + 1, to + 1),
        Invocation::new(args.name(), [(to + 1).to_string(), (from + 1).to_string()]),
    ))
}

fn rename(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let index = channel_arg(project, args, 0)?;
    let name = args.rest(1)?;
    let previous = std::mem::replace(&mut project.channels[index].name, name.clone());
    Ok(Outcome::undoable(
        format!("channel {} renamed to {name}", index + 1),
        Invocation::new(args.name(), [(index + 1).to_string(), previous]),
    ))
}

/// Sets a channel field and returns an undoable outcome, or a plain one if the
/// value did not change.
fn set_field<T: PartialEq + Copy>(
    project: &mut Project,
    args: &Args<'_>,
    index: usize,
    value: T,
    field: fn(&mut Channel) -> &mut T,
    show: fn(T) -> String,
) -> Outcome {
    let previous = std::mem::replace(field(&mut project.channels[index]), value);
    let message = format!("channel {} {}", index + 1, show(value));
    if previous == value {
        return Outcome::done(message);
    }
    let raw = show(previous);
    let raw = raw.rsplit(' ').next().unwrap_or_default().to_owned();
    Outcome::undoable(
        message,
        Invocation::new(args.name(), [(index + 1).to_string(), raw]),
    )
}

fn set_volume(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let volume = check_volume(args.f32(1, "volume")?)?;
    Ok(set_field(
        project,
        args,
        index,
        volume,
        |c| &mut c.volume,
        |v| format!("volume {}", num(v)),
    ))
}

fn set_pan(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let pan = check_pan(args.f32(1, "pan")?)?;
    Ok(set_field(
        project,
        args,
        index,
        pan,
        |c| &mut c.pan,
        |v| format!("pan {}", num(v)),
    ))
}

fn set_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let mute = args.flag(1)?;
    Ok(set_field(
        project,
        args,
        index,
        mute,
        |c| &mut c.mute,
        |v| format!("mute {}", on_off(v)),
    ))
}

fn toggle_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = channel_arg(project, args, 0)?;
    let channel = &mut project.channels[index];
    channel.mute = !channel.mute;
    Ok(Outcome::undoable(
        format!("channel {} mute {}", index + 1, on_off(channel.mute)),
        Invocation::new(args.name(), [(index + 1).to_string()]),
    ))
}

fn solo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = channel_arg(project, args, 0)?;
    let already_solo = project
        .channels
        .iter()
        .enumerate()
        .all(|(i, c)| c.mute == (i != index));
    let mut undo = Vec::new();
    for (i, channel) in project.channels.iter_mut().enumerate() {
        let mute = !already_solo && i != index;
        if channel.mute != mute {
            undo.push(format!(
                "channel.set_mute {} {}",
                i + 1,
                on_off(channel.mute)
            ));
            channel.mute = mute;
        }
    }
    let message = if already_solo {
        "all channels unmuted".to_owned()
    } else {
        format!("channel {} solo", index + 1)
    };
    if undo.is_empty() {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(message, Invocation::new(BATCH, undo)))
}

fn route(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let track = project.mixer_index(args.parse(1, "mixer track")?)?;
    Ok(set_field(
        project,
        args,
        index,
        track,
        |c| &mut c.mixer_track,
        |t| format!("routed to {t}"),
    ))
}

fn set_sample(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let sample = args.str(1)?;
    check_sample(project, args, sample)?;
    let current = match &mut project.channels[index].instrument {
        Instrument::Sampler { sample } | Instrument::Granular { sample } => sample,
        _ => return Err(args.invalid(format!("channel {} does not play a sample", index + 1))),
    };
    let previous = std::mem::replace(current, sample.to_owned());
    Ok(Outcome::undoable(
        format!("channel {} plays {sample}", index + 1),
        Invocation::new(args.name(), [(index + 1).to_string(), previous]),
    ))
}

fn set_param(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = channel_arg(project, args, 0)?;
    let name = args.str(1)?;
    let param = index_of(INSTRUMENT_PARAMS, name).ok_or_else(|| {
        let names: Vec<_> = INSTRUMENT_PARAMS.iter().map(|p| p.name).collect();
        args.invalid(format!(
            "unknown parameter `{name}`; choose from {}",
            names.join(", ")
        ))
    })?;
    let spec = INSTRUMENT_PARAMS[param];
    let value = args.f32(2, name)?;
    if !spec.accepts(value) {
        return Err(musiccraft_project::ProjectError::out_of_range(
            name,
            f64::from(value),
            f64::from(spec.min),
            f64::from(spec.max),
        )
        .into());
    }
    let value = spec.sanitize(value);
    let slot = &mut project.channels[index].params.0[param];
    let previous = std::mem::replace(slot, value);
    let message = format!("channel {} {name} {}", index + 1, num(value));
    if previous == value {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), name.to_owned(), num(previous)],
        ),
    ))
}
