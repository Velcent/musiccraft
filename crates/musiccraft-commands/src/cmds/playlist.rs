use musiccraft_project::{
    Clip, ClipSource, Instrument, Marker, MarkerAction, Meter, PLAYLIST_TRACKS, Project,
};

use crate::args::{Args, on_off, ticks};
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "playlist.add_clip",
            "playlist.add_clip <pattern> <track> <start bar> [length bars] [offset]",
            "Place a pattern on the playlist (bars from 0; <n>t for ticks); offset starts it part-way in.",
            add_clip,
        ),
        cmd(
            "playlist.add_automation_clip",
            "playlist.add_automation_clip <automation> <track> <start bar> [length bars] [offset]",
            "Place an automation on the playlist.",
            add_automation_clip,
        ),
        cmd(
            "playlist.add_audio_clip",
            "playlist.add_audio_clip <channel> <track> <start bar> <length bars> [offset]",
            "Place a sampler channel's sample on the playlist as an audio clip.",
            add_audio_clip,
        ),
        cmd(
            "playlist.remove_clip",
            "playlist.remove_clip <track> <at bar>",
            "Remove the clip covering a position.",
            remove_clip,
        ),
        cmd(
            "playlist.move_clip",
            "playlist.move_clip <track> <at bar> <new track> <new start bar>",
            "Move the clip covering a position.",
            move_clip,
        ),
        cmd(
            "playlist.resize_clip",
            "playlist.resize_clip <track> <at bar> <length bars>",
            "Change the length of the clip covering a position.",
            resize_clip,
        ),
        cmd(
            "playlist.set_track_mute",
            "playlist.set_track_mute <track> <on|off>",
            "Mute or unmute a playlist track.",
            set_track_mute,
        ),
        cmd(
            "playlist.toggle_clip_mute",
            "playlist.toggle_clip_mute <track> <at>",
            "Mute or unmute one clip: a muted clip stays but does not play.",
            toggle_clip_mute,
        ),
        cmd(
            "playlist.toggle_track_arm",
            "playlist.toggle_track_arm <track>",
            "Arm a playlist track: recorded audio takes go to the first armed track with room.",
            toggle_track_arm,
        ),
        cmd(
            "playlist.set_track_solo",
            "playlist.set_track_solo <track> <on|off>",
            "Solo a playlist track: while any track is soloed, only soloed tracks play.",
            set_track_solo,
        ),
        cmd(
            "playlist.toggle_track_solo",
            "playlist.toggle_track_solo <track>",
            "Solo or unsolo a playlist track.",
            toggle_track_solo,
        ),
        cmd(
            "playlist.rename_track",
            "playlist.rename_track <track> [name]",
            "Name a playlist track (no name restores the default).",
            rename_track,
        ),
        cmd(
            "playlist.set_track_color",
            "playlist.set_track_color <track> <1-16|none>",
            "Colour a playlist track.",
            set_track_color,
        ),
        cmd(
            "playlist.toggle_track_mute",
            "playlist.toggle_track_mute <track>",
            "Toggle a playlist track's mute.",
            toggle_track_mute,
        ),
        cmd(
            "playlist.add_marker",
            "playlist.add_marker <bar> <name>",
            "Add a named marker to the song timeline.",
            add_marker,
        ),
        cmd(
            "playlist.remove_marker",
            "playlist.remove_marker <bar>",
            "Remove the marker at a position.",
            remove_marker,
        ),
        cmd(
            "playlist.set_marker_action",
            "playlist.set_marker_action <bar> <none|pause|song_loop>",
            "Make a marker a label, a pause point or the song loop point (only one marker is the song loop).",
            set_marker_action,
        ),
        cmd(
            "playlist.set_time_signature",
            "playlist.set_time_signature <bar> <n/d|none>",
            "Change the song's time signature at a position (a new bar starts there), or remove the change.",
            set_time_signature,
        ),
        cmd(
            "playlist.set_markers",
            "playlist.set_markers <json>",
            "Replace every marker (used by undo).",
            set_markers,
        ),
        cmd(
            "playlist.clear",
            "playlist.clear",
            "Remove every clip.",
            clear,
        ),
        cmd(
            "playlist.set_clips",
            "playlist.set_clips <json>",
            "Replace every clip from JSON (used by undo).",
            set_clips,
        ),
    ]
}

/// 0-based playlist row from a 1-based track number in argument `i`.
pub(super) fn track_arg(args: &Args<'_>, i: usize) -> Result<u32, CommandError> {
    let track = args.number(i, "playlist track")?;
    u32::try_from(track)
        .ok()
        .filter(|t| *t <= PLAYLIST_TRACKS)
        .map(|t| t - 1)
        .ok_or_else(|| {
            args.invalid(format!(
                "playlist tracks are numbered 1 to {PLAYLIST_TRACKS}"
            ))
        })
}

pub(super) fn clip_arg(
    project: &Project,
    args: &Args<'_>,
    i: usize,
) -> Result<usize, CommandError> {
    let track = track_arg(args, i)?;
    let at = args.bars(i + 1, &project.song_meter())?;
    project
        .playlist
        .clip_at(track, at)
        .ok_or_else(|| args.invalid(format!("no clip on track {} at {at}t", track + 1)))
}

pub(super) fn overlaps(project: &Project, clip: &Clip, ignore: Option<usize>) -> bool {
    project.playlist.clips.iter().enumerate().any(|(i, other)| {
        Some(i) != ignore
            && other.track == clip.track
            && other.start < clip.end()
            && clip.start < other.end()
    })
}

fn add_inverse(project: &Project, clip: &Clip) -> Invocation {
    let (name, number) = match clip.source {
        ClipSource::Pattern(id) => ("playlist.add_clip", project.pattern_position(id)),
        ClipSource::Automation(id) => (
            "playlist.add_automation_clip",
            project.automation_position(id),
        ),
        ClipSource::Audio(id) => ("playlist.add_audio_clip", project.channel_position(id)),
    };
    let mut args = vec![
        number.map_or(0, |i| i + 1).to_string(),
        (clip.track + 1).to_string(),
        ticks(clip.start),
        ticks(clip.length),
    ];
    if clip.offset > 0 {
        args.push(ticks(clip.offset));
    }
    let add = Invocation::new(name, args);
    if !clip.muted {
        return add;
    }
    // A muted clip comes back muted.
    let mute = Invocation::new(
        "playlist.toggle_clip_mute",
        [(clip.track + 1).to_string(), ticks(clip.start)],
    );
    Invocation::new(crate::BATCH, [add.to_string(), mute.to_string()])
}

fn toggle_clip_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = clip_arg(project, args, 0)?;
    let clip = &mut project.playlist.clips[index];
    clip.muted = !clip.muted;
    Ok(Outcome::undoable(
        format!(
            "clip on track {} at {}t {}",
            clip.track + 1,
            clip.start,
            if clip.muted { "muted" } else { "unmuted" }
        ),
        Invocation::new(
            args.name(),
            [(clip.track + 1).to_string(), ticks(clip.start)],
        ),
    ))
}

fn remove_inverse(clip: &Clip) -> Invocation {
    Invocation::new(
        "playlist.remove_clip",
        [(clip.track + 1).to_string(), ticks(clip.start)],
    )
}

/// The optional offset argument `i` (bars of the clip's source, `meter`, or
/// `<n>t`), 0 when absent.
fn offset_arg(args: &Args<'_>, i: usize, meter: &Meter) -> Result<u32, CommandError> {
    match args.opt(i) {
        Some(_) => args.bars(i, meter),
        None => Ok(0),
    }
}

fn add_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 5)?;
    let pattern_index = project.pattern_index(args.number(0, "pattern")?)?;
    let pattern = &project.patterns[pattern_index];
    let meter = project.song_meter();
    let start = args.bars(2, &meter)?;
    let length = match args.opt(3) {
        Some(_) => args.bar_length(3, start, &meter)?,
        None => pattern.length_ticks(),
    };
    let clip = Clip {
        source: ClipSource::Pattern(pattern.id),
        track: track_arg(args, 1)?,
        start,
        length,
        offset: offset_arg(args, 4, &pattern.meter())?,
        muted: false,
    };
    place(
        project,
        args,
        clip,
        format!("placed pattern {}", pattern_index + 1),
    )
}

/// Adds a clip unless it overlaps another on its track.
fn place(
    project: &mut Project,
    args: &Args<'_>,
    clip: Clip,
    what: String,
) -> Result<Outcome, CommandError> {
    if overlaps(project, &clip, None) {
        return Err(args.invalid(format!("track {} already has a clip there", clip.track + 1)));
    }
    project.playlist.clips.push(clip);
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!("{what} on track {} at {}t", clip.track + 1, clip.start),
        remove_inverse(&clip),
    ))
}

fn add_automation_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 5)?;
    let index = project.automation_index(args.number(0, "automation")?)?;
    let automation = &project.automations[index];
    let meter = project.song_meter();
    let start = args.bars(2, &meter)?;
    let length = match args.opt(3) {
        Some(_) => args.bar_length(3, start, &meter)?,
        // Whole bars of the song from where the clip starts.
        None => meter.round_up_to_bar(start.saturating_add(automation.length_ticks())) - start,
    };
    let clip = Clip {
        source: ClipSource::Automation(automation.id),
        track: track_arg(args, 1)?,
        start,
        length,
        offset: offset_arg(args, 4, &meter)?,
        muted: false,
    };
    place(
        project,
        args,
        clip,
        format!("placed automation {}", index + 1),
    )
}

fn add_audio_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 5)?;
    let index = project.channel_index(args.number(0, "channel")?)?;
    let channel = &project.channels[index];
    if !matches!(channel.instrument, Instrument::Sampler { .. }) {
        return Err(args.invalid(format!("channel {} is not a sampler", index + 1)));
    }
    let meter = project.song_meter();
    let start = args.bars(2, &meter)?;
    let clip = Clip {
        source: ClipSource::Audio(channel.id),
        track: track_arg(args, 1)?,
        start,
        length: args.bar_length(3, start, &meter)?,
        offset: offset_arg(args, 4, &meter)?,
        muted: false,
    };
    place(
        project,
        args,
        clip,
        format!("placed audio of channel {}", index + 1),
    )
}

fn remove_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = clip_arg(project, args, 0)?;
    let clip = project.playlist.clips[index];
    let inverse = add_inverse(project, &clip);
    project.playlist.clips.remove(index);
    Ok(Outcome::undoable(
        format!(
            "removed clip on track {} at {}t",
            clip.track + 1,
            clip.start
        ),
        inverse,
    ))
}

fn move_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let index = clip_arg(project, args, 0)?;
    let old = project.playlist.clips[index];
    let moved = Clip {
        track: track_arg(args, 2)?,
        start: args.bars(3, &project.song_meter())?,
        ..old
    };
    if overlaps(project, &moved, Some(index)) {
        return Err(args.invalid("the destination overlaps another clip"));
    }
    project.playlist.clips[index] = moved;
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!(
            "moved clip to track {} at {}t",
            moved.track + 1,
            moved.start
        ),
        Invocation::new(
            args.name(),
            [
                (moved.track + 1).to_string(),
                ticks(moved.start),
                (old.track + 1).to_string(),
                ticks(old.start),
            ],
        ),
    ))
}

fn resize_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = clip_arg(project, args, 0)?;
    let old = project.playlist.clips[index];
    let resized = Clip {
        length: args.bar_length(2, old.start, &project.song_meter())?,
        ..old
    };
    if overlaps(project, &resized, Some(index)) {
        return Err(args.invalid("the new length overlaps another clip"));
    }
    project.playlist.clips[index] = resized;
    Ok(Outcome::undoable(
        format!("clip length {}t", resized.length),
        Invocation::new(
            args.name(),
            [
                (old.track + 1).to_string(),
                ticks(old.start),
                ticks(old.length),
            ],
        ),
    ))
}

fn set_mute_state(project: &mut Project, track: u32, mute: bool) -> bool {
    if mute {
        !project.playlist.muted_tracks.insert(track)
    } else {
        project.playlist.muted_tracks.remove(&track)
    }
}

fn set_track_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let track = track_arg(args, 0)?;
    let mute = args.flag(1)?;
    let was_muted = set_mute_state(project, track, mute);
    let message = format!("playlist track {} mute {}", track + 1, on_off(mute));
    if was_muted == mute {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [(track + 1).to_string(), on_off(was_muted).to_owned()],
        ),
    ))
}

fn toggle_track_mute(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let track = track_arg(args, 0)?;
    let mute = !project.playlist.muted_tracks.contains(&track);
    set_mute_state(project, track, mute);
    Ok(Outcome::undoable(
        format!("playlist track {} mute {}", track + 1, on_off(mute)),
        Invocation::new(args.name(), [(track + 1).to_string()]),
    ))
}

fn set_track_solo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let track = track_arg(args, 0)?;
    let solo = args.flag(1)?;
    let was = project.playlist.soloed_tracks.contains(&track);
    if solo {
        project.playlist.soloed_tracks.insert(track);
    } else {
        project.playlist.soloed_tracks.remove(&track);
    }
    let message = format!("playlist track {} solo {}", track + 1, on_off(solo));
    if was == solo {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [(track + 1).to_string(), on_off(was).to_owned()],
        ),
    ))
}

fn toggle_track_arm(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let track = track_arg(args, 0)?;
    let armed = !project.playlist.armed_tracks.remove(&track);
    if armed {
        project.playlist.armed_tracks.insert(track);
    }
    Ok(Outcome::undoable(
        format!(
            "playlist track {} {}",
            track + 1,
            if armed { "armed" } else { "disarmed" }
        ),
        Invocation::new(args.name(), [(track + 1).to_string()]),
    ))
}

fn toggle_track_solo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let track = track_arg(args, 0)?;
    let solo = !project.playlist.soloed_tracks.remove(&track);
    if solo {
        project.playlist.soloed_tracks.insert(track);
    }
    Ok(Outcome::undoable(
        format!("playlist track {} solo {}", track + 1, on_off(solo)),
        Invocation::new(args.name(), [(track + 1).to_string()]),
    ))
}

fn rename_track(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, usize::MAX)?;
    let track = track_arg(args, 0)?;
    let name = if args.len() > 1 {
        Some(args.rest(1)?.trim().to_owned()).filter(|n| !n.is_empty())
    } else {
        None
    };
    let names = &mut project.playlist.track_names;
    let previous = match &name {
        Some(name) => names.insert(track, name.clone()),
        None => names.remove(&track),
    };
    let mut inverse = vec![(track + 1).to_string()];
    inverse.extend(previous);
    Ok(Outcome::undoable(
        format!(
            "playlist track {} is {}",
            track + 1,
            project.playlist.track_name(track)
        ),
        Invocation::new(args.name(), inverse),
    ))
}

fn set_track_color(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let track = track_arg(args, 0)?;
    let color = match args.str(1)? {
        "none" => None,
        _ => {
            let n = args.number(1, "colour")?;
            if !(1..=16).contains(&n) {
                return Err(args.invalid("colours are 1 to 16"));
            }
            Some(u8::try_from(n - 1).unwrap_or(0))
        }
    };
    let colors = &mut project.playlist.track_colors;
    let previous = match color {
        Some(c) => colors.insert(track, c),
        None => colors.remove(&track),
    };
    let label = |c: Option<u8>| c.map_or_else(|| "none".to_owned(), |c| (c + 1).to_string());
    Ok(Outcome::undoable(
        format!("playlist track {} colour {}", track + 1, label(color)),
        Invocation::new(args.name(), [(track + 1).to_string(), label(previous)]),
    ))
}

pub(super) fn replace_clips(
    project: &mut Project,
    args: &Args<'_>,
    clips: Vec<Clip>,
) -> Result<Outcome, CommandError> {
    let old = std::mem::replace(&mut project.playlist.clips, clips);
    project.playlist.sort_clips();
    let json = serde_json::to_string(&old).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("playlist has {} clips", project.playlist.clips.len()),
        Invocation::new("playlist.set_clips", [json]),
    ))
}

fn clear(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    replace_clips(project, args, Vec::new())
}

fn set_clips(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let clips: Vec<Clip> =
        serde_json::from_str(args.str(0)?).map_err(|e| args.invalid(e.to_string()))?;
    for clip in &clips {
        let exists = match clip.source {
            ClipSource::Pattern(id) => project.pattern_position(id).is_some(),
            ClipSource::Automation(id) => project.automation_position(id).is_some(),
            ClipSource::Audio(id) => project.channel_position(id).is_some(),
        };
        if !exists || clip.length == 0 || clip.track >= PLAYLIST_TRACKS {
            return Err(args.invalid("a clip refers to a missing pattern or is malformed"));
        }
    }
    replace_clips(project, args, clips)
}

/// An inverse that puts every marker back exactly as it was.
pub(super) fn restore_markers(
    args: &Args<'_>,
    markers: &[Marker],
) -> Result<Invocation, CommandError> {
    let json = serde_json::to_string(markers).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Invocation::new("playlist.set_markers", [json]))
}

fn add_marker(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let tick = args.bars(0, &project.song_meter())?;
    let name = args.rest(1)?;
    let before = project.playlist.markers.clone();
    let markers = &mut project.playlist.markers;
    let replaced = markers
        .iter()
        .position(|m| m.tick == tick)
        .map(|i| markers.remove(i));
    // Renaming a marker keeps what it does.
    let action = replaced.as_ref().map_or(MarkerAction::None, |m| m.action);
    markers.push(Marker {
        tick,
        name: name.clone(),
        action,
    });
    markers.sort_by_key(|m| m.tick);
    let inverse = match replaced {
        Some(_) => restore_markers(args, &before)?,
        None => Invocation::new("playlist.remove_marker", [ticks(tick)]),
    };
    Ok(Outcome::undoable(
        format!("marker {name} at {tick}t"),
        inverse,
    ))
}

fn remove_marker(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let tick = args.bars(0, &project.song_meter())?;
    let before = project.playlist.markers.clone();
    let markers = &mut project.playlist.markers;
    let index = markers
        .iter()
        .position(|m| m.tick == tick)
        .ok_or_else(|| args.invalid(format!("there is no marker at {tick}t")))?;
    let old = markers.remove(index);
    Ok(Outcome::undoable(
        format!("removed marker {}", old.name),
        restore_markers(args, &before)?,
    ))
}

fn set_marker_action(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let tick = args.bars(0, &project.song_meter())?;
    let raw = args.str(1)?;
    let action = MarkerAction::parse(raw).ok_or_else(|| {
        let names: Vec<_> = MarkerAction::ALL.iter().map(|a| a.name()).collect();
        args.invalid(format!(
            "`{raw}` is not a marker action; choose from {}",
            names.join(", ")
        ))
    })?;
    let before = project.playlist.markers.clone();
    let markers = &mut project.playlist.markers;
    let index = markers
        .iter()
        .position(|m| m.tick == tick)
        .ok_or_else(|| args.invalid(format!("there is no marker at {tick}t")))?;
    let name = markers[index].name.clone();
    if markers[index].action == action {
        return Ok(Outcome::done(format!(
            "marker {name} is already {}",
            action.name()
        )));
    }
    if action == MarkerAction::SongLoop {
        // The song has one loop point: it moves to this marker.
        for marker in markers
            .iter_mut()
            .filter(|m| m.action == MarkerAction::SongLoop)
        {
            marker.action = MarkerAction::None;
        }
    }
    markers[index].action = action;
    Ok(Outcome::undoable(
        format!("marker {name}: {}", action.name()),
        restore_markers(args, &before)?,
    ))
}

fn set_time_signature(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let tick = args.bars(0, &project.song_meter())?;
    let before = project.playlist.markers.clone();
    let markers = &mut project.playlist.markers;
    let existing = markers.iter().position(|m| m.tick == tick);
    if args.str(1)? == "none" {
        let index = existing
            .filter(|&i| matches!(markers[i].action, MarkerAction::TimeSignature(_)))
            .ok_or_else(|| args.invalid(format!("there is no time signature change at {tick}t")))?;
        markers.remove(index);
        return Ok(Outcome::undoable(
            format!("removed the time signature change at {tick}t"),
            restore_markers(args, &before)?,
        ));
    }
    let sig = args.time_sig(1)?;
    if tick == 0 {
        return Err(args.invalid(
            "the signature at the start is the project's: use project.set_time_signature",
        ));
    }
    let action = MarkerAction::TimeSignature(sig);
    match existing {
        Some(i) if markers[i].action == action => {
            return Ok(Outcome::done(format!(
                "the song is already {sig} at {tick}t"
            )));
        }
        Some(i) => {
            // A label keeps its name; one named after its old signature follows.
            let marker = &mut markers[i];
            if let MarkerAction::TimeSignature(old) = marker.action
                && marker.name == old.to_string()
            {
                marker.name = sig.to_string();
            }
            marker.action = action;
        }
        None => {
            markers.push(Marker {
                tick,
                name: sig.to_string(),
                action,
            });
            markers.sort_by_key(|m| m.tick);
        }
    }
    Ok(Outcome::undoable(
        format!("time signature {sig} at {tick}t"),
        restore_markers(args, &before)?,
    ))
}

fn set_markers(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let mut markers: Vec<Marker> =
        serde_json::from_str(args.str(0)?).map_err(|e| args.invalid(e.to_string()))?;
    markers.sort_by_key(|m| m.tick);
    if markers.windows(2).any(|pair| pair[0].tick == pair[1].tick) {
        return Err(args.invalid("two markers share a position"));
    }
    let before = std::mem::replace(&mut project.playlist.markers, markers);
    Ok(Outcome::undoable(
        format!("{} markers", project.playlist.markers.len()),
        restore_markers(args, &before)?,
    ))
}
