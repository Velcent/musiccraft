use musiccraft_engine::{AutomationLane, AutomationTarget as EngineTarget, NoteEvent, Sequence};
use musiccraft_params::TICKS_PER_STEP;
use musiccraft_project::{AutomationTarget, ClipSource, MAX_VOLUME, Pattern, PlayMode, Project};

/// Where a note starts once the pattern's swing is applied: notes exactly on an
/// off-beat sixteenth move later by up to half a step.
fn swung(pattern: &Pattern, start: u32) -> u32 {
    let on_offbeat = start % (2 * TICKS_PER_STEP) == TICKS_PER_STEP;
    if on_offbeat && pattern.swing > 0.0 {
        start + (pattern.swing.clamp(0.0, 1.0) * (TICKS_PER_STEP / 2) as f32).round() as u32
    } else {
        start
    }
}

fn event(
    project: &Project,
    note: &musiccraft_project::Note,
    start: u32,
    length: u32,
) -> Option<NoteEvent> {
    let channel = u16::try_from(project.channel_position(note.channel)?).ok()?;
    Some(NoteEvent {
        start: u64::from(start),
        length: u64::from(length.max(1)),
        channel,
        key: note.key,
        velocity: f32::from(note.velocity) / 127.0,
    })
}

fn pattern_events(project: &Project, pattern: &Pattern) -> Vec<NoteEvent> {
    pattern
        .notes
        .iter()
        .filter_map(|note| event(project, note, swung(pattern, note.start), note.length))
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
                        start: u64::from(clip.start),
                        length: u64::from(clip.length.max(1)),
                        channel,
                        key: root,
                        velocity: 1.0,
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
            let start = offset + swung(pattern, note.start);
            if start >= clip.length {
                continue;
            }
            let length = note.length.min(clip.length - start);
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
/// whole arrangement in song mode.
#[must_use]
pub fn build_sequence(project: &Project) -> Sequence {
    match project.play_mode {
        PlayMode::Pattern => {
            let pattern = project.current_pattern();
            Sequence::new(
                pattern_events(project, pattern),
                u64::from(pattern.length_ticks()),
            )
        }
        PlayMode::Song => {
            Sequence::new(song_events(project), u64::from(project.song_length_ticks()))
                .with_automation(song_automation(project))
        }
    }
}
