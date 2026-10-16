use musiccraft_engine::{AutomationLane, AutomationTarget as EngineTarget, NoteEvent, Sequence};
use musiccraft_params::{TICKS_PER_STEP, inst};
use musiccraft_project::{AutomationTarget, ClipSource, MAX_VOLUME, Pattern, PlayMode, Project};

/// A note's start and length within its pattern once swing is applied: notes
/// exactly on an off-beat sixteenth move later by up to half a step (the
/// project swing times the channel's swing mix); with `swing_truncate` they
/// keep their end instead.
fn swung(project: &Project, note: &musiccraft_project::Note) -> (u32, u32) {
    let on_offbeat = note.start % (2 * TICKS_PER_STEP) == TICKS_PER_STEP;
    let params = project
        .channel_position(note.channel)
        .and_then(|i| project.channels.get(i))
        .map(|c| &c.params);
    let mix = params.map_or(1.0, |p| p.get(inst::SWING_MIX));
    let truncate = params.is_some_and(|p| p.get(inst::SWING_TRUNCATE) >= 0.5);
    let amount = (project.swing * mix).clamp(0.0, 1.0);
    if !on_offbeat || amount <= 0.0 {
        return (note.start, note.length);
    }
    let delay = (amount * (TICKS_PER_STEP / 2) as f32).round() as u32;
    let length = if truncate {
        note.length.saturating_sub(delay).max(1)
    } else {
        note.length
    };
    (note.start + delay, length)
}

fn event(
    project: &Project,
    note: &musiccraft_project::Note,
    start: u32,
    length: u32,
) -> Option<NoteEvent> {
    if note.props.muted {
        return None;
    }
    let channel = u16::try_from(project.channel_position(note.channel)?).ok()?;
    let p = note.props;
    let centred = |v: u8| (f32::from(v) - 64.0) / 64.0;
    Some(NoteEvent {
        shape: musiccraft_engine::NoteShape {
            fine: f32::from(p.fine) / 100.0,
            // Release velocity 0..127 scales the release ×0.25..×4.
            release: 4.0_f32.powf(centred(p.release)),
            cutoff: centred(p.mod_x) * 3.0,
            resonance: centred(p.mod_y) * 0.5,
            porta: p.porta,
            color: p.color,
        },
        slide: note.slide,
        start: u64::from(start),
        length: u64::from(length.max(1)),
        channel,
        key: note.key,
        velocity: f32::from(note.velocity) / 127.0,
        pan: f32::from(note.pan) / 100.0,
    })
}

fn pattern_events(project: &Project, pattern: &Pattern) -> Vec<NoteEvent> {
    pattern
        .notes
        .iter()
        .filter_map(|note| {
            let (start, length) = swung(project, note);
            event(project, note, start, length)
        })
        .collect()
}

fn song_events(project: &Project) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for clip in &project.playlist.clips {
        if project.playlist.muted_tracks.contains(&clip.track) {
            continue;
        }
        match clip.source {
            ClipSource::Pattern(id) => {
                if let Some(pattern) = project
                    .pattern_position(id)
                    .and_then(|i| project.patterns.get(i))
                {
                    pattern_clip_events(project, pattern, clip, &mut events);
                }
            }
            ClipSource::Audio(id) => {
                // The sample plays once, unpitched (at its root key), for the clip's length.
                let Some(index) = project.channel_position(id) else {
                    continue;
                };
                let root = project.channels[index]
                    .params
                    .get(musiccraft_params::inst::ROOT_KEY) as u8;
                if let Ok(channel) = u16::try_from(index) {
                    events.push(NoteEvent {
                        shape: musiccraft_engine::NoteShape::default(),
                        slide: false,
                        start: u64::from(clip.start),
                        length: u64::from(clip.length.max(1)),
                        channel,
                        key: root,
                        velocity: 1.0,
                        pan: 0.0,
                    });
                }
            }
            ClipSource::Automation(_) => {}
        }
    }
    events
}

fn pattern_clip_events(
    project: &Project,
    pattern: &Pattern,
    clip: &musiccraft_project::Clip,
    events: &mut Vec<NoteEvent>,
) {
    let period = pattern.length_ticks().max(1);
    let mut offset = 0;
    // The pattern repeats for as long as the clip lasts.
    while offset < clip.length {
        for note in &pattern.notes {
            let (start, length) = swung(project, note);
            let start = offset + start;
            if start >= clip.length {
                continue;
            }
            let length = length.min(clip.length - start);
            events.extend(event(project, note, clip.start + start, length));
        }
        offset += period;
    }
}

/// Maps a control position (0..1) to a parameter's real units.
type Convert = Box<dyn Fn(f32) -> f32>;

/// Converts a control position (0..1) into the target's real units and the
/// engine's target address. `None` if the target no longer exists.
fn engine_target(project: &Project, target: &AutomationTarget) -> Option<(EngineTarget, Convert)> {
    use musiccraft_params::{INSTRUMENT_PARAMS, index_of, volume_from_unit};
    let channel = |id| {
        project
            .channel_position(id)
            .and_then(|i| u16::try_from(i).ok())
    };
    let track = |t: usize| {
        (t < project.mixer.len())
            .then(|| u16::try_from(t).ok())
            .flatten()
    };
    let volume = |v: f32| volume_from_unit(v, MAX_VOLUME);
    let pan = |v: f32| v * 2.0 - 1.0;
    Some(match target {
        AutomationTarget::ChannelVolume { channel: id } => {
            (EngineTarget::ChannelVolume(channel(*id)?), Box::new(volume))
        }
        AutomationTarget::ChannelPan { channel: id } => {
            (EngineTarget::ChannelPan(channel(*id)?), Box::new(pan))
        }
        AutomationTarget::ChannelParam { channel: id, param } => {
            let index = index_of(INSTRUMENT_PARAMS, param)?;
            let spec = INSTRUMENT_PARAMS[index];
            (
                EngineTarget::ChannelParam(channel(*id)?, u8::try_from(index).ok()?),
                Box::new(move |v| spec.from_unit(v)),
            )
        }
        AutomationTarget::MixerVolume { track: t } => {
            (EngineTarget::TrackVolume(track(*t)?), Box::new(volume))
        }
        AutomationTarget::MixerPan { track: t } => {
            (EngineTarget::TrackPan(track(*t)?), Box::new(pan))
        }
        AutomationTarget::EffectParam {
            track: t,
            slot,
            param,
        } => {
            let effect = project.mixer.get(*t)?.effects.get(*slot)?;
            let index = index_of(effect.kind.params(), param)?;
            let spec = effect.kind.params()[index];
            (
                EngineTarget::EffectParam(
                    track(*t)?,
                    u8::try_from(*slot).ok()?,
                    u8::try_from(index).ok()?,
                ),
                Box::new(move |v| spec.from_unit(v)),
            )
        }
    })
}

/// One lane per automation clip, covering exactly the clip's span.
fn song_automation(project: &Project) -> Vec<AutomationLane> {
    let mut lanes = Vec::new();
    for clip in &project.playlist.clips {
        let Some(id) = clip.automation() else {
            continue;
        };
        if project.playlist.muted_tracks.contains(&clip.track) {
            continue;
        }
        let Some(automation) = project
            .automation_position(id)
            .map(|i| &project.automations[i])
        else {
            continue;
        };
        let Some((target, convert)) = engine_target(project, &automation.target) else {
            continue;
        };
        let start = u64::from(clip.start);
        let end = u64::from(clip.end());
        let mut points: Vec<(u64, f32)> = Vec::new();
        if let Some(v) = automation.value_at(0.0) {
            points.push((start, convert(v)));
        }
        for point in automation
            .points
            .iter()
            .filter(|p| p.tick > 0 && p.tick < clip.length)
        {
            points.push((start + u64::from(point.tick), convert(point.value)));
        }
        if let Some(v) = automation.value_at(f64::from(clip.length)) {
            points.push((end, convert(v)));
        }
        lanes.push(AutomationLane { target, points });
    }
    lanes
}
/// The loop the transport plays: the selected pattern in pattern mode, the
/// whole arrangement in song mode (with its pause and song-loop markers, unless
/// a loop region is set).
#[must_use]
pub fn build_sequence(project: &Project) -> Sequence {
    let full = build_full_sequence(project);
    match (project.play_mode, project.playlist.loop_range) {
        (PlayMode::Song, Some((start, end))) => {
            let events = full.events().to_vec();
            let lanes = full.automation().to_vec();
            Sequence::new(events, u64::from(end))
                .with_automation(lanes)
                .with_loop_start(u64::from(start))
        }
        (PlayMode::Song, None) => {
            let pauses = project.playlist.pause_ticks();
            let mut sequence = full.with_pauses(pauses.into_iter().map(u64::from).collect());
            if let Some(at) = project.playlist.song_loop_tick() {
                // At the end the song returns to the song-loop marker; Stop
                // still goes back to the beginning.
                sequence = sequence.with_loop_start(u64::from(at)).with_home(0);
            }
            sequence
        }
        _ => full,
    }
}

/// The whole pattern or song, ignoring the loop region (used for export).
#[must_use]
pub fn build_full_sequence(project: &Project) -> Sequence {
    match project.play_mode {
        PlayMode::Pattern => {
            let pattern = project.current_pattern();
            Sequence::new(
                crate::arp::apply(project, pattern_events(project, pattern)),
                u64::from(pattern.length_ticks()),
            )
        }
        PlayMode::Song => Sequence::new(
            crate::arp::apply(project, song_events(project)),
            u64::from(project.song_length_ticks()),
        )
        .with_automation(song_automation(project)),
    }
}
