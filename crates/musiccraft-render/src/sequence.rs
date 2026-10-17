use musiccraft_engine::{
    AutomationLane, AutomationTarget as EngineTarget, ControllerLane, ControllerSource, Convert,
    NoteEvent, Sequence,
};
use musiccraft_formula::Formula;
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
            start: 0.0,
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
    song_note_events(project, true)
}

/// The song's note events; `audio` includes the notes that play audio clips.
fn song_note_events(project: &Project, audio: bool) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for clip in &project.playlist.clips {
        if clip.muted || !project.playlist.track_audible(clip.track) {
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
            ClipSource::Audio(_) if !audio => {}
            ClipSource::Audio(id) => {
                // The sample plays once, unpitched (at its root key), for the clip's length.
                let Some(index) = project.channel_position(id) else {
                    continue;
                };
                let root = project.channels[index]
                    .params
                    .get(musiccraft_params::inst::ROOT_KEY) as u8;
                // A sliced clip starts part-way into the sample.
                let seconds = f64::from(clip.offset) / f64::from(musiccraft_params::PPQ) * 60.0
                    / project.tempo_bpm();
                if let Ok(channel) = u16::try_from(index) {
                    events.push(NoteEvent {
                        shape: musiccraft_engine::NoteShape {
                            start: seconds as f32,
                            ..musiccraft_engine::NoteShape::default()
                        },
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
    let period = u64::from(pattern.length_ticks().max(1));
    let (skip, span) = (u64::from(clip.offset), u64::from(clip.length));
    // The pattern repeats for as long as the clip lasts; a sliced clip starts
    // `skip` ticks into it (notes starting before that are cut).
    let mut repeat = skip / period * period;
    while repeat < skip + span {
        for note in &pattern.notes {
            let (start, length) = swung(project, note);
            let Some(local) = (repeat + u64::from(start)).checked_sub(skip) else {
                continue;
            };
            if local >= span {
                continue;
            }
            let length = u64::from(length).min(span - local);
            let (Ok(local), Ok(length)) = (u32::try_from(local), u32::try_from(length)) else {
                continue;
            };
            events.extend(event(project, note, clip.start + local, length));
        }
        repeat += period;
    }
}

/// Converts a control position (0..1) into the target's real units and the
/// engine's target address. `None` if the target no longer exists.
fn engine_target(project: &Project, target: &AutomationTarget) -> Option<(EngineTarget, Convert)> {
    use musiccraft_params::{INSTRUMENT_PARAMS, index_of};
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
    let volume = Convert::Volume(MAX_VOLUME);
    Some(match target {
        AutomationTarget::ChannelVolume { channel: id } => {
            (EngineTarget::ChannelVolume(channel(*id)?), volume)
        }
        AutomationTarget::ChannelPan { channel: id } => {
            (EngineTarget::ChannelPan(channel(*id)?), Convert::Pan)
        }
        AutomationTarget::ChannelParam { channel: id, param } => {
            let index = index_of(INSTRUMENT_PARAMS, param)?;
            (
                EngineTarget::ChannelParam(channel(*id)?, u8::try_from(index).ok()?),
                Convert::Spec(INSTRUMENT_PARAMS[index]),
            )
        }
        AutomationTarget::MixerVolume { track: t } => {
            (EngineTarget::TrackVolume(track(*t)?), volume)
        }
        AutomationTarget::MixerPan { track: t } => {
            (EngineTarget::TrackPan(track(*t)?), Convert::Pan)
        }
        AutomationTarget::EffectParam {
            track: t,
            slot,
            param,
        } => {
            let effect = project.mixer.get(*t)?.effects.get(*slot)?;
            let index = index_of(effect.kind.params(), param)?;
            (
                EngineTarget::EffectParam(
                    track(*t)?,
                    u8::try_from(*slot).ok()?,
                    u8::try_from(index).ok()?,
                ),
                Convert::Spec(effect.kind.params()[index]),
            )
        }
    })
}

/// The project's enabled controllers as the engine runs them.
fn controller_lanes(project: &Project) -> Vec<ControllerLane> {
    use musiccraft_project::ControllerKind as K;
    let channel = |id| {
        project
            .channel_position(id)
            .and_then(|i| u16::try_from(i).ok())
    };
    project
        .controllers
        .iter()
        .filter(|c| c.enabled)
        .filter_map(|c| {
            let (target, convert) = engine_target(project, &c.target)?;
            let source = match &c.kind {
                K::Lfo {
                    shape,
                    period,
                    phase,
                } => ControllerSource::Lfo {
                    shape: *shape,
                    period: *period,
                    phase: *phase,
                },
                K::Envelope {
                    channel: id,
                    attack,
                    decay,
                    sustain,
                    release,
                } => ControllerSource::Envelope {
                    channel: channel(*id)?,
                    attack: *attack,
                    decay: *decay,
                    sustain: *sustain,
                    release: *release,
                },
                K::Keyboard {
                    channel: id,
                    low,
                    high,
                } => ControllerSource::Keyboard {
                    channel: channel(*id)?,
                    low: *low,
                    high: *high,
                },
                K::Peak {
                    track,
                    gain,
                    attack,
                    release,
                } => ControllerSource::Peak {
                    track: u16::try_from(*track)
                        .ok()
                        .filter(|t| usize::from(*t) < project.mixer.len())?,
                    gain: *gain,
                    attack: *attack,
                    release: *release,
                },
                K::Formula { expr } => ControllerSource::Formula(Formula::parse(expr).ok()?),
                K::Macro { value } => ControllerSource::Macro(*value),
            };
            Some(ControllerLane {
                target,
                convert,
                min: c.min,
                max: c.max,
                source,
            })
        })
        .collect()
}

/// A pattern's automation lanes placed at song tick `at`, keeping only points
/// at pattern ticks `skip..skip + span` (a clip's window into the pattern).
/// Pattern lanes drive their parameter only between their points.
fn pattern_lanes(
    project: &Project,
    pattern: &Pattern,
    at: u64,
    skip: u64,
    span: u64,
) -> Vec<AutomationLane> {
    pattern
        .automation
        .iter()
        .filter_map(|lane| {
            let (target, convert) = engine_target(project, &lane.target)?;
            let points: Vec<(u64, f32)> = lane
                .points
                .iter()
                .filter(|p| (skip..skip + span).contains(&u64::from(p.tick)))
                .map(|p| (at + u64::from(p.tick) - skip, convert.apply(p.value)))
                .collect();
            (!points.is_empty()).then_some(AutomationLane { target, points })
        })
        .collect()
}

/// Pattern automation of every pattern clip, once per repeat of the pattern.
fn song_pattern_lanes(project: &Project) -> Vec<AutomationLane> {
    let mut lanes = Vec::new();
    for clip in &project.playlist.clips {
        if clip.muted || !project.playlist.track_audible(clip.track) {
            continue;
        }
        let Some(pattern) = clip
            .pattern()
            .and_then(|id| project.pattern_position(id))
            .and_then(|i| project.patterns.get(i))
        else {
            continue;
        };
        if pattern.automation.is_empty() {
            continue;
        }
        let period = u64::from(pattern.length_ticks().max(1));
        let (skip, span) = (u64::from(clip.offset), u64::from(clip.length));
        let mut repeat = skip / period * period;
        while repeat < skip + span {
            // This repeat's window, in pattern ticks.
            let from = skip.max(repeat) - repeat;
            let to = (skip + span).min(repeat + period) - repeat;
            let at = u64::from(clip.start) + (repeat + from - skip);
            lanes.extend(pattern_lanes(project, pattern, at, from, to - from));
            repeat += period;
        }
    }
    lanes
}

/// One lane per automation clip, covering exactly the clip's span.
fn song_automation(project: &Project) -> Vec<AutomationLane> {
    let mut lanes = Vec::new();
    for clip in &project.playlist.clips {
        let Some(id) = clip.automation() else {
            continue;
        };
        if clip.muted || !project.playlist.track_audible(clip.track) {
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
        // A sliced clip shows the automation from `offset` on.
        let (skip, span) = (clip.offset, clip.length);
        let mut points: Vec<(u64, f32)> = Vec::new();
        if let Some(v) = automation.value_at(f64::from(skip)) {
            points.push((start, convert.apply(v)));
        }
        for point in automation
            .points
            .iter()
            .filter(|p| p.tick > skip && p.tick < skip + span)
        {
            points.push((
                start + u64::from(point.tick - skip),
                convert.apply(point.value),
            ));
        }
        if let Some(v) = automation.value_at(f64::from(skip + span)) {
            points.push((end, convert.apply(v)));
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
            let controllers = full.controllers().to_vec();
            Sequence::new(events, u64::from(end))
                .with_automation(lanes)
                .with_controllers(controllers)
                .with_loop_start(u64::from(start))
                .with_meter(full.meter().clone())
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
/// The notes a MIDI export writes: what the transport would play once
/// (pattern or song by play mode, swing and arpeggios applied), without audio
/// clips; and the length in ticks.
pub(crate) fn midi_events(project: &Project) -> (Vec<NoteEvent>, u64) {
    match project.play_mode {
        PlayMode::Pattern => {
            let pattern = project.current_pattern();
            (
                crate::delivery::apply(
                    project,
                    crate::arp::apply(project, pattern_events(project, pattern)),
                ),
                u64::from(pattern.length_ticks()),
            )
        }
        PlayMode::Song => (
            crate::delivery::apply(
                project,
                crate::arp::apply(project, song_note_events(project, false)),
            ),
            u64::from(project.song_length_ticks()),
        ),
    }
}

pub fn build_full_sequence(project: &Project) -> Sequence {
    match project.play_mode {
        PlayMode::Pattern => {
            let pattern = project.current_pattern();
            Sequence::new(
                crate::delivery::apply(
                    project,
                    crate::arp::apply(project, pattern_events(project, pattern)),
                ),
                u64::from(pattern.length_ticks()),
            )
            .with_automation(pattern_lanes(
                project,
                pattern,
                0,
                0,
                u64::from(pattern.length_ticks()),
            ))
            .with_controllers(controller_lanes(project))
            .with_meter(pattern.meter())
        }
        PlayMode::Song => Sequence::new(
            crate::delivery::apply(project, crate::arp::apply(project, song_events(project))),
            u64::from(project.song_length_ticks()),
        )
        .with_automation({
            let mut lanes = song_automation(project);
            lanes.extend(song_pattern_lanes(project));
            lanes
        })
        .with_controllers(controller_lanes(project))
        .with_meter(project.song_meter()),
    }
}
