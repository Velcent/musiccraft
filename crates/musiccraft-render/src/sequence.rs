use musiccraft_engine::{NoteEvent, Sequence};
use musiccraft_project::{Pattern, PlayMode, Project};

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
        .filter_map(|note| event(project, note, note.start, note.length))
        .collect()
}

fn song_events(project: &Project) -> Vec<NoteEvent> {
    let mut events = Vec::new();
    for clip in &project.playlist.clips {
        if project.playlist.muted_tracks.contains(&clip.track) {
            continue;
        }
        let Some(pattern) = project
            .pattern_position(clip.pattern)
            .and_then(|i| project.patterns.get(i))
        else {
            continue;
        };
        let period = pattern.length_ticks().max(1);
        let mut offset = 0;
        // The pattern repeats for as long as the clip lasts.
        while offset < clip.length {
            for note in &pattern.notes {
                let start = offset + note.start;
                if start >= clip.length {
                    continue;
                }
                let length = note.length.min(clip.length - start);
                events.extend(event(project, note, clip.start + start, length));
            }
            offset += period;
        }
    }
    events
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
        }
    }
}
