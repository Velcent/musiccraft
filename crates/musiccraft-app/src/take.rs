//! Audio recording: collects input while recording and turns a finished take
//! into a WAV file plus the commands that place it on the playlist.

use std::path::{Path, PathBuf};

use musiccraft_commands::{BATCH, Invocation};
use musiccraft_project::{PLAYLIST_TRACKS, Project};

/// Takes shorter than this are discarded (accidental clicks).
const MIN_SECONDS: f64 = 0.1;

/// Input gathered since recording started.
pub struct Take {
    /// Song position (ticks) when recording started.
    pub start: f64,
    /// Interleaved input samples.
    pub samples: Vec<f32>,
}

/// Where takes are written: a `recordings` folder beside the project file, or
/// the per-user data folder for unsaved projects.
pub fn folder(project: &Project) -> Option<PathBuf> {
    match project.path().and_then(Path::parent) {
        Some(dir) => Some(dir.join("recordings")),
        None => crate::autosave::path()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .map(|d| d.join("recordings")),
    }
}

/// Where a take from `start` lasting `length` ticks goes (1-based): the first
/// armed row with room for it, else the first row with no clips.
fn take_track(project: &Project, start: u32, length: u32) -> u32 {
    let clips = &project.playlist.clips;
    let room = |row: u32| {
        !clips
            .iter()
            .any(|c| c.track == row && c.start < start + length && start < c.end())
    };
    project
        .playlist
        .armed_tracks
        .iter()
        .copied()
        .find(|row| room(*row))
        .or_else(|| (0..PLAYLIST_TRACKS).find(|row| !clips.iter().any(|c| c.track == *row)))
        .unwrap_or(PLAYLIST_TRACKS - 1)
        + 1
}

/// One undo step that adds the take as a sampler channel and an audio clip.
pub fn place(project: &Project, path: &Path, start_tick: u32, seconds: f64) -> Invocation {
    let ticks_per_second = project.tempo_bpm() / 60.0 * f64::from(musiccraft_project::params::PPQ);
    let length = (seconds * ticks_per_second).ceil().max(1.0) as u32;
    let number = project.channels.len() + 1;
    Invocation::new(
        BATCH,
        [
            Invocation::new("channel.add_sampler", [path.display().to_string()]).to_string(),
            format!("channel.set_param {number} one_shot 0"),
            format!(
                "playlist.add_audio_clip {number} {} {start_tick}t {length}t",
                take_track(project, start_tick, length)
            ),
        ],
    )
}

/// Writes a finished take and returns the command that places it, or `None`
/// if the take was too short.
///
/// # Errors
///
/// Returns a message if the file cannot be written.
pub fn finish(
    take: Take,
    channels: u16,
    sample_rate: u32,
    project: &Project,
    name: &str,
) -> Result<Option<Invocation>, String> {
    let (left, right) = musiccraft_audio::deinterleave(&take.samples, channels);
    let seconds = left.len() as f64 / f64::from(sample_rate.max(1));
    if seconds < MIN_SECONDS {
        return Ok(None);
    }
    let dir = folder(project).ok_or("no folder for recordings")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let path = dir.join(format!("{name}.wav"));
    musiccraft_samples::write_wav(
        &path,
        &left,
        &right,
        sample_rate,
        musiccraft_samples::WavFormat::Float32,
    )
    .map_err(|e| e.to_string())?;
    Ok(Some(place(
        project,
        &path,
        take.start.max(0.0) as u32,
        seconds,
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_commands::Session;

    #[test]
    fn finished_takes_become_audio_clips() {
        let dir = std::env::temp_dir().join(format!("musiccraft-take-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut session = Session::new(Project::default(), musiccraft_cli::full_registry());
        session
            .run_line(&format!(
                "project.save \"{}\"",
                dir.join("p.mcproj").display()
            ))
            .unwrap();
        session.run_line("project.set_tempo 120").unwrap();
        let take = Take {
            start: 384.0,
            samples: vec![0.25; 48_000 * 2],
        };
        let invocation = finish(take, 2, 48_000, session.project(), "take-1")
            .unwrap()
            .unwrap();
        session.run(&invocation).unwrap();
        let project = session.project();
        assert_eq!(project.channels.len(), 1);
        let clip = project.playlist.clips[0];
        // One second at 120 BPM = two beats = 192 ticks.
        assert_eq!((clip.start, clip.length), (384, 192));
        assert!(dir.join("recordings/take-1.wav").exists());
        let short = Take {
            start: 0.0,
            samples: vec![0.0; 100],
        };
        assert!(
            finish(short, 1, 48_000, session.project(), "take-2")
                .unwrap()
                .is_none()
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn takes_go_to_the_first_armed_track_with_room() {
        let mut project = Project::default();
        let path = Path::new("take.wav");
        // No arm: the first empty row.
        assert!(
            place(&project, path, 0, 1.0)
                .to_string()
                .contains("add_audio_clip 1 1 0t")
        );
        project.playlist.armed_tracks.insert(4);
        project.playlist.armed_tracks.insert(6);
        assert!(
            place(&project, path, 0, 1.0)
                .to_string()
                .contains("add_audio_clip 1 5 0t")
        );
        // Row 5 is busy at that time: the next armed row takes it.
        project.playlist.clips.push(musiccraft_project::Clip {
            source: musiccraft_project::ClipSource::Pattern(musiccraft_project::PatternId(1)),
            track: 4,
            start: 0,
            length: 384,
            offset: 0,
            muted: false,
        });
        assert!(
            place(&project, path, 0, 1.0)
                .to_string()
                .contains("add_audio_clip 1 7 0t")
        );
    }
}
