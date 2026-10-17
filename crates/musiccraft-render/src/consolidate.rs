//! `playlist.consolidate`: renders one clip to a WAV file and replaces it with
//! an audio clip of that file.

use std::path::{Path, PathBuf};

use musiccraft_commands::{BATCH, Command, CommandError, Invocation, Outcome};
use musiccraft_params::{PPQ, inst};
use musiccraft_project::{Channel, ChannelId, Clip, ClipSource, Instrument, PlayMode, Project};
use musiccraft_samples::{WavFormat, write_wav};

use crate::{PluginHost, RenderOptions, render_with};

/// `playlist.consolidate <track> <at>`.
#[derive(Default)]
pub struct ConsolidateCommand {
    /// Plugins to render with.
    pub host: Option<PluginHost>,
}

/// Sample rate of consolidated audio.
const RATE: u32 = 48_000;

/// A time argument: bars of `meter` from 0, or `<n>t` ticks.
fn time(raw: &str, meter: &musiccraft_project::Meter) -> Option<u32> {
    match raw.strip_suffix('t') {
        Some(ticks) => ticks.parse().ok(),
        None => raw
            .parse::<f64>()
            .ok()
            .filter(|bars| bars.is_finite() && *bars >= 0.0)
            .map(|bars| meter.bars_to_ticks(bars)),
    }
}

/// A free file name in `dir` for `stem`.
fn free_path(dir: &Path, stem: &str) -> PathBuf {
    (1..)
        .map(|n| dir.join(format!("{stem}-{n}.wav")))
        .find(|p| !p.exists())
        .unwrap_or_else(|| dir.join(format!("{stem}.wav")))
}

/// The project with only `clip` playing, moved to the song start.
fn solo_clip(project: &Project, clip: &Clip) -> Project {
    let mut alone = project.clone();
    alone.play_mode = PlayMode::Song;
    alone.metronome = false;
    let playlist = &mut alone.playlist;
    playlist.clips = vec![Clip { start: 0, ..*clip }];
    playlist.markers.clear();
    playlist.loop_range = None;
    playlist.muted_tracks.clear();
    playlist.soloed_tracks.clear();
    alone
}

impl Command for ConsolidateCommand {
    fn name(&self) -> &'static str {
        "playlist.consolidate"
    }

    fn summary(&self) -> &'static str {
        "Render a pattern or audio clip to a WAV file and replace it with an audio clip."
    }

    fn usage(&self) -> &'static str {
        "playlist.consolidate <track> <at>"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let invalid = |reason: String| CommandError::InvalidArgument {
            command: self.name().to_owned(),
            reason,
        };
        let [track, at] = args else {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        };
        let track = track
            .parse::<u32>()
            .ok()
            .and_then(|t| t.checked_sub(1))
            .ok_or_else(|| invalid(format!("`{track}` is not a playlist track")))?;
        let at = time(at, &project.song_meter())
            .ok_or_else(|| invalid(format!("`{at}` is not a time")))?;
        let index = project
            .playlist
            .clip_at(track, at)
            .ok_or_else(|| invalid(format!("no clip on track {} at {at}t", track + 1)))?;
        let clip = project.playlist.clips[index];
        let label = match clip.source {
            ClipSource::Pattern(id) => project
                .pattern_position(id)
                .map(|i| project.patterns[i].name.clone()),
            ClipSource::Audio(id) => project
                .channel_position(id)
                .map(|i| project.channels[i].name.clone()),
            ClipSource::Automation(_) => {
                return Err(invalid("automation clips have no audio".to_owned()));
            }
        }
        .ok_or_else(|| invalid("the clip's source is missing".to_owned()))?;
        let dir = project
            .path()
            .and_then(Path::parent)
            .map(|d| d.join("consolidated"))
            .ok_or_else(|| invalid("save the project first: audio is written beside it".into()))?;
        std::fs::create_dir_all(&dir).map_err(|e| CommandError::Io(e.to_string()))?;

        let options = RenderOptions {
            sample_rate: RATE,
            loops: 1,
            max_tail_seconds: 0.0,
        };
        let (mut left, mut right, _) =
            render_with(&solo_clip(project, &clip), options, self.host.as_ref());
        let seconds = f64::from(clip.length) / f64::from(PPQ) * 60.0 / project.tempo_bpm();
        let frames = (seconds * f64::from(RATE)).round() as usize;
        left.resize(frames, 0.0);
        right.resize(frames, 0.0);
        let stem: String = label
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect();
        let path = free_path(&dir, &stem);
        write_wav(&path, &left, &right, RATE, WavFormat::Float32)
            .map_err(|e| CommandError::Io(e.to_string()))?;

        let old =
            serde_json::to_string(&project.playlist.clips).map_err(|e| invalid(e.to_string()))?;
        let id = ChannelId(project.alloc_id());
        let mut channel = Channel::new(
            id,
            format!("{label} (audio)"),
            Instrument::Sampler {
                sample: path.display().to_string(),
            },
        );
        channel.params.0[inst::ONE_SHOT] = 0.0;
        project.channels.push(channel);
        let number = project.channels.len();
        project.playlist.clips[index] = Clip {
            source: ClipSource::Audio(id),
            offset: 0,
            ..clip
        };
        Ok(Outcome::undoable(
            format!("consolidated {label} to {}", path.display()),
            Invocation::new(
                BATCH,
                [
                    Invocation::new("playlist.set_clips", [old]).to_string(),
                    format!("channel.remove {number}"),
                ],
            ),
        ))
    }
}
