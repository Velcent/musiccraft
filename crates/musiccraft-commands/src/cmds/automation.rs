use musiccraft_project::params::{
    INSTRUMENT_PARAMS, TICKS_PER_BAR, TICKS_PER_STEP, index_of, volume_to_unit,
};
use musiccraft_project::{
    Automation, AutomationId, AutomationPoint, AutomationTarget, Clip, ClipSource, MAX_VOLUME,
    PLAYLIST_TRACKS, Project,
};
use serde::{Deserialize, Serialize};

use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "automation.create",
            "automation.create <target> [name]",
            "Create an automation for channel:<n>:volume|pan|<param>, mixer:<t>:volume|pan or mixer:<t>:<slot>:<param>.",
            create,
        ),
        cmd(
            "automation.remove",
            "automation.remove <automation>",
            "Delete an automation and its playlist clips.",
            remove,
        ),
        cmd(
            "automation.insert",
            "automation.insert <position> <json>",
            "Insert an automation from its JSON form (used by undo).",
            insert,
        ),
        cmd(
            "automation.rename",
            "automation.rename <automation> <name>",
            "Rename an automation.",
            rename,
        ),
        cmd(
            "automation.add_point",
            "automation.add_point <automation> <time> <value 0-1>",
            "Add a point (time in steps from the clip start); replaces one at the same time.",
            add_point,
        ),
        cmd(
            "automation.remove_point",
            "automation.remove_point <automation> <time>",
            "Remove the point at a time.",
            remove_point,
        ),
        cmd(
            "automation.move_point",
            "automation.move_point <automation> <time> <new time> <value 0-1>",
            "Move a point in time and value.",
            move_point,
        ),
        cmd(
            "automation.set_points",
            "automation.set_points <automation> <json>",
            "Replace every point with [[tick, value], ...] (used by undo).",
            set_points,
        ),
        cmd(
            "automation.record",
            "automation.record <target> <tick>",
            "Record the target's current value at a song position, creating an automation and clip if needed.",
            record,
        ),
    ]
}

/// Records `target`'s current value at song position `tick`: into an
/// automation clip of that target covering `tick`, or a new automation whose
/// clip covers the whole song (so later points, even after the song wraps,
/// land in it).
fn record(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let target = parse_target(project, args, args.str(0)?)?;
    let tick = args.time(1, TICKS_PER_STEP)?;
    let (value, label) =
        current(project, &target).ok_or_else(|| args.invalid("the target does not exist"))?;
    let covering = project.playlist.clips.iter().find_map(|clip| {
        let index = project.automation_position(clip.automation()?)?;
        let covers = clip.start <= tick && tick < clip.start + clip.length;
        (covers && project.automations[index].target == target).then_some((index, clip.start))
    });
    if let Some((index, start)) = covering {
        let at = tick - start;
        let automation = &mut project.automations[index];
        let old = automation.points.clone();
        automation.points.retain(|p| p.tick != at);
        automation.points.push(AutomationPoint { tick: at, value });
        automation.sort_points();
        return Ok(Outcome::undoable(
            format!("recorded {label} at {tick}t"),
            set_points_inverse(index, &old),
        ));
    }
    let length = project
        .song_length_ticks()
        .max(tick + 1)
        .next_multiple_of(TICKS_PER_BAR);
    let track = (0..PLAYLIST_TRACKS)
        .find(|row| {
            !project
                .playlist
                .clips
                .iter()
                .any(|c| c.track == *row && c.start < length)
        })
        .ok_or_else(|| args.invalid("no playlist track has room for an automation clip"))?;
    let id = AutomationId(project.alloc_id());
    project.automations.push(Automation {
        id,
        name: label.clone(),
        target,
        points: vec![AutomationPoint { tick, value }],
    });
    project.playlist.clips.push(Clip {
        source: ClipSource::Automation(id),
        track,
        start: 0,
        length,
        offset: 0,
        muted: false,
    });
    project.playlist.sort_clips();
    let number = project.automations.len();
    Ok(Outcome::undoable(
        format!("recording {label} into automation {number}"),
        Invocation::new("automation.remove", [number.to_string()]),
    ))
}

fn automation_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    Ok(project.automation_index(args.number(i, "automation")?)?)
}

/// Parses `channel:2:volume`, `mixer:0:pan`, `mixer:3:1:mix` and similar.
pub(crate) fn parse_target(
    project: &Project,
    args: &Args<'_>,
    text: &str,
) -> Result<AutomationTarget, CommandError> {
    let parts: Vec<&str> = text.split(':').collect();
    let bad = || args.invalid(format!("`{text}` is not an automation target"));
    let number = |s: &str| s.parse::<usize>().map_err(|_| bad());
    match parts.as_slice() {
        ["channel", n, what] => {
            let index = project.channel_index(number(n)?)?;
            let channel = project.channels[index].id;
            Ok(match *what {
                "volume" => AutomationTarget::ChannelVolume { channel },
                "pan" => AutomationTarget::ChannelPan { channel },
                param => {
                    index_of(INSTRUMENT_PARAMS, param)
                        .ok_or_else(|| args.invalid(format!("unknown parameter `{param}`")))?;
                    AutomationTarget::ChannelParam {
                        channel,
                        param: param.to_owned(),
                    }
                }
            })
        }
        ["mixer", t, what] => {
            let track = project.mixer_index(number(t)?)?;
            match *what {
                "volume" => Ok(AutomationTarget::MixerVolume { track }),
                "pan" => Ok(AutomationTarget::MixerPan { track }),
                _ => Err(bad()),
            }
        }
        ["mixer", t, slot, param] => {
            let track = project.mixer_index(number(t)?)?;
            let slot = number(slot)?.checked_sub(1).ok_or_else(bad)?;
            let effect = project.mixer[track].effects.get(slot).ok_or_else(|| {
                args.invalid(format!(
                    "mixer track {track} has no effect in slot {}",
                    slot + 1
                ))
            })?;
            index_of(effect.kind.params(), param).ok_or_else(|| {
                args.invalid(format!("{} has no parameter `{param}`", effect.kind))
            })?;
            Ok(AutomationTarget::EffectParam {
                track,
                slot,
                param: (*param).to_owned(),
            })
        }
        _ => Err(bad()),
    }
}

/// The target's current value as a control position, and a readable label.
pub(crate) fn current(project: &Project, target: &AutomationTarget) -> Option<(f32, String)> {
    let channel = |id| project.channel_position(id).map(|i| &project.channels[i]);
    match target {
        AutomationTarget::ChannelVolume { channel: id } => channel(*id).map(|c| {
            (
                volume_to_unit(c.volume, MAX_VOLUME),
                format!("{} volume", c.name),
            )
        }),
        AutomationTarget::ChannelPan { channel: id } => {
            channel(*id).map(|c| ((c.pan + 1.0) / 2.0, format!("{} pan", c.name)))
        }
        AutomationTarget::ChannelParam { channel: id, param } => {
            let c = channel(*id)?;
            let i = index_of(INSTRUMENT_PARAMS, param)?;
            Some((
                INSTRUMENT_PARAMS[i].to_unit(c.params.get(i)),
                format!("{} {param}", c.name),
            ))
        }
        AutomationTarget::MixerVolume { track } => project.mixer.get(*track).map(|t| {
            (
                volume_to_unit(t.volume, MAX_VOLUME),
                format!("{} volume", t.name),
            )
        }),
        AutomationTarget::MixerPan { track } => project
            .mixer
            .get(*track)
            .map(|t| ((t.pan + 1.0) / 2.0, format!("{} pan", t.name))),
        AutomationTarget::EffectParam { track, slot, param } => {
            let t = project.mixer.get(*track)?;
            let effect = t.effects.get(*slot)?;
            let i = index_of(effect.kind.params(), param)?;
            Some((
                effect.kind.params()[i].to_unit(effect.values[i]),
                format!("{} {} {param}", t.name, effect.kind),
            ))
        }
    }
}

fn create(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, usize::MAX)?;
    let target = parse_target(project, args, args.str(0)?)?;
    let (value, label) =
        current(project, &target).ok_or_else(|| args.invalid("the target does not exist"))?;
    let name = if args.len() > 1 { args.rest(1)? } else { label };
    let id = AutomationId(project.alloc_id());
    project.automations.push(Automation {
        id,
        name: name.clone(),
        target,
        points: vec![
            AutomationPoint { tick: 0, value },
            AutomationPoint {
                tick: 4 * TICKS_PER_BAR,
                value,
            },
        ],
    });
    let number = project.automations.len();
    Ok(Outcome::undoable(
        format!("created automation {number} ({name})"),
        Invocation::new("automation.remove", [number.to_string()]),
    ))
}

#[derive(Serialize, Deserialize)]
struct AutomationSnapshot {
    automation: Automation,
    clips: Vec<Clip>,
}

fn remove(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let index = automation_arg(project, args, 0)?;
    let automation = project.automations.remove(index);
    let (clips, kept): (Vec<Clip>, Vec<Clip>) = project
        .playlist
        .clips
        .drain(..)
        .partition(|c| c.automation() == Some(automation.id));
    project.playlist.clips = kept;
    let name = automation.name.clone();
    let json = serde_json::to_string(&AutomationSnapshot { automation, clips })
        .map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("removed automation {} ({name})", index + 1),
        Invocation::new("automation.insert", [(index + 1).to_string(), json]),
    ))
}

fn insert(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let position = args.number(0, "position")?;
    if position > project.automations.len() + 1 {
        return Err(args.invalid(format!("position {position} is past the last automation")));
    }
    let snapshot: AutomationSnapshot =
        serde_json::from_str(args.str(1)?).map_err(|e| args.invalid(e.to_string()))?;
    if project
        .automation_position(snapshot.automation.id)
        .is_some()
    {
        return Err(args.invalid("that automation already exists"));
    }
    project.reserve_id(snapshot.automation.id.0);
    let name = snapshot.automation.name.clone();
    project
        .automations
        .insert(position - 1, snapshot.automation);
    project.playlist.clips.extend(snapshot.clips);
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!("inserted automation {position} ({name})"),
        Invocation::new("automation.remove", [position.to_string()]),
    ))
}

fn rename(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let index = automation_arg(project, args, 0)?;
    let name = args.rest(1)?;
    let previous = std::mem::replace(&mut project.automations[index].name, name.clone());
    Ok(Outcome::undoable(
        format!("automation {} renamed to {name}", index + 1),
        Invocation::new(args.name(), [(index + 1).to_string(), previous]),
    ))
}

fn value_arg(args: &Args<'_>, i: usize) -> Result<f32, CommandError> {
    Ok(musiccraft_project::check_range(
        "value",
        args.f32(i, "value")?,
        0.0,
        1.0,
    )?)
}

fn set_points_inverse(index: usize, points: &[AutomationPoint]) -> Invocation {
    let rows: Vec<(u32, f32)> = points.iter().map(|p| (p.tick, p.value)).collect();
    Invocation::new(
        "automation.set_points",
        [
            (index + 1).to_string(),
            serde_json::to_string(&rows).unwrap_or_else(|_| "[]".into()),
        ],
    )
}

/// Applies `edit` to an automation's points as one undoable change.
fn edit_points(
    project: &mut Project,
    args: &Args<'_>,
    message: String,
    edit: impl FnOnce(&mut Vec<AutomationPoint>) -> Result<(), CommandError>,
) -> Result<Outcome, CommandError> {
    let index = automation_arg(project, args, 0)?;
    let automation = &mut project.automations[index];
    let old = automation.points.clone();
    edit(&mut automation.points)?;
    automation.sort_points();
    Ok(Outcome::undoable(message, set_points_inverse(index, &old)))
}

fn add_point(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let tick = args.time(1, TICKS_PER_STEP)?;
    let value = value_arg(args, 2)?;
    edit_points(
        project,
        args,
        format!("point at {tick}t = {value}"),
        |points| {
            points.retain(|p| p.tick != tick);
            points.push(AutomationPoint { tick, value });
            Ok(())
        },
    )
}

fn remove_point(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let tick = args.time(1, TICKS_PER_STEP)?;
    edit_points(
        project,
        args,
        format!("removed point at {tick}t"),
        |points| {
            let before = points.len();
            points.retain(|p| p.tick != tick);
            if points.len() == before {
                return Err(args.invalid(format!("there is no point at {tick}t")));
            }
            Ok(())
        },
    )
}

fn move_point(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let tick = args.time(1, TICKS_PER_STEP)?;
    let new_tick = args.time(2, TICKS_PER_STEP)?;
    let value = value_arg(args, 3)?;
    edit_points(
        project,
        args,
        format!("moved point to {new_tick}t = {value}"),
        |points| {
            let index = points
                .iter()
                .position(|p| p.tick == tick)
                .ok_or_else(|| args.invalid(format!("there is no point at {tick}t")))?;
            points.remove(index);
            points.retain(|p| p.tick != new_tick);
            points.push(AutomationPoint {
                tick: new_tick,
                value,
            });
            Ok(())
        },
    )
}

fn set_points(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let rows: Vec<(u32, f32)> =
        serde_json::from_str(args.str(1)?).map_err(|e| args.invalid(e.to_string()))?;
    if rows
        .iter()
        .any(|(_, v)| !v.is_finite() || !(0.0..=1.0).contains(v))
    {
        return Err(args.invalid("values must be 0 to 1"));
    }
    let count = rows.len();
    edit_points(
        project,
        args,
        format!("automation has {count} points"),
        |points| {
            *points = rows
                .into_iter()
                .map(|(tick, value)| AutomationPoint { tick, value })
                .collect();
            Ok(())
        },
    )
}

/// Updates automation targets on a mixer track when effect slots move.
/// `map(slot)` gives the new slot, or `None` when the slot disappears.
pub(crate) fn remap_slots(
    project: &mut Project,
    track: usize,
    map: impl Fn(usize) -> Option<usize>,
) {
    for automation in &mut project.automations {
        if let AutomationTarget::EffectParam { track: t, slot, .. } = &mut automation.target {
            if *t == track {
                if let Some(new) = map(*slot) {
                    *slot = new;
                }
            }
        }
    }
}

/// Whether any automation drives something on mixer track `track` (or, with
/// `slot`, that effect slot).
pub(crate) fn targets_track(project: &Project, track: usize, slot: Option<usize>) -> bool {
    project.automations.iter().any(|a| match &a.target {
        AutomationTarget::MixerVolume { track: t } | AutomationTarget::MixerPan { track: t } => {
            slot.is_none() && *t == track
        }
        AutomationTarget::EffectParam {
            track: t, slot: s, ..
        } => *t == track && slot.is_none_or(|x| x == *s),
        _ => false,
    })
}

/// Shifts mixer track numbers in automation targets: tracks at or after
/// `from` move by `delta`.
pub(crate) fn shift_tracks(project: &mut Project, from: usize, delta: isize) {
    for automation in &mut project.automations {
        let track = match &mut automation.target {
            AutomationTarget::MixerVolume { track }
            | AutomationTarget::MixerPan { track }
            | AutomationTarget::EffectParam { track, .. } => track,
            _ => continue,
        };
        if *track >= from {
            *track = track.saturating_add_signed(delta);
        }
    }
}
