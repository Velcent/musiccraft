use musiccraft_project::params::TICKS_PER_BAR;
use musiccraft_project::{Clip, PLAYLIST_TRACKS, Project};

use crate::args::{Args, on_off, ticks};
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "playlist.add_clip",
            "playlist.add_clip <pattern> <track> <start bar> [length bars]",
            "Place a pattern on the playlist (bars from 0; <n>t for ticks).",
            add_clip,
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
            "playlist.toggle_track_mute",
            "playlist.toggle_track_mute <track>",
            "Toggle a playlist track's mute.",
            toggle_track_mute,
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
fn track_arg(args: &Args<'_>, i: usize) -> Result<u32, CommandError> {
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

fn clip_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    let track = track_arg(args, i)?;
    let at = args.time(i + 1, TICKS_PER_BAR)?;
    project
        .playlist
        .clip_at(track, at)
        .ok_or_else(|| args.invalid(format!("no clip on track {} at {at}t", track + 1)))
}

fn overlaps(project: &Project, clip: &Clip, ignore: Option<usize>) -> bool {
    project.playlist.clips.iter().enumerate().any(|(i, other)| {
        Some(i) != ignore
            && other.track == clip.track
            && other.start < clip.end()
            && clip.start < other.end()
    })
}

fn add_inverse(project: &Project, clip: &Clip) -> Invocation {
    let pattern = project.pattern_position(clip.pattern).map_or(0, |i| i + 1);
    Invocation::new(
        "playlist.add_clip",
        [
            pattern.to_string(),
            (clip.track + 1).to_string(),
            ticks(clip.start),
            ticks(clip.length),
        ],
    )
}

fn remove_inverse(clip: &Clip) -> Invocation {
    Invocation::new(
        "playlist.remove_clip",
        [(clip.track + 1).to_string(), ticks(clip.start)],
    )
}

fn add_clip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 4)?;
    let pattern_index = project.pattern_index(args.number(0, "pattern")?)?;
    let pattern = &project.patterns[pattern_index];
    let length = match args.opt(3) {
        Some(_) => args.duration(3, TICKS_PER_BAR)?,
        None => pattern.length_ticks(),
    };
    let clip = Clip {
        pattern: pattern.id,
        track: track_arg(args, 1)?,
        start: args.time(2, TICKS_PER_BAR)?,
        length,
    };
    if overlaps(project, &clip, None) {
        return Err(args.invalid(format!("track {} already has a clip there", clip.track + 1)));
    }
    project.playlist.clips.push(clip);
    project.playlist.sort_clips();
    Ok(Outcome::undoable(
        format!(
            "placed pattern {} on track {} at {}t",
            pattern_index + 1,
            clip.track + 1,
            clip.start
        ),
        remove_inverse(&clip),
    ))
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
        start: args.time(3, TICKS_PER_BAR)?,
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
        length: args.duration(2, TICKS_PER_BAR)?,
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

fn replace_clips(
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
        if project.pattern_position(clip.pattern).is_none()
            || clip.length == 0
            || clip.track >= PLAYLIST_TRACKS
        {
            return Err(args.invalid("a clip refers to a missing pattern or is malformed"));
        }
    }
    replace_clips(project, args, clips)
}
