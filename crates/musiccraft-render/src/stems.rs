//! `sample.split_stems` and `sample.add_stems`: splits a sampler channel's
//! sound into vocals, drums, bass and other, and adds a channel and clips for
//! each (see `docs/spec/stems.md`). [`StemJob`] runs the split on a worker
//! thread for the app.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use musiccraft_commands::{BATCH, Command, CommandError, Invocation, Outcome};
use musiccraft_project::{
    Channel, ChannelId, Clip, ClipSource, Instrument, PLAYLIST_TRACKS, Project,
};
use musiccraft_samples::{SampleCache, WavFormat, write_wav};
use musiccraft_stems::{Stem, Stems, split};

/// Rate built-in sounds are made at when split.
const BUILTIN_RATE: u32 = 48_000;

/// `sample.split_stems <channel>`.
#[derive(Default)]
pub struct SplitStemsCommand;

/// `sample.add_stems <channel> <vocals> <drums> <bass> <other>`.
#[derive(Default)]
pub struct AddStemsCommand;

fn invalid(command: &str, reason: impl Into<String>) -> CommandError {
    CommandError::InvalidArgument {
        command: command.to_owned(),
        reason: reason.into(),
    }
}

/// The index of the sampler channel `raw` names, and its sample.
fn sampler(project: &Project, command: &str, raw: &str) -> Result<(usize, String), CommandError> {
    let number: usize = raw
        .parse()
        .map_err(|_| invalid(command, format!("`{raw}` is not a channel number")))?;
    let index = project.channel_index(number)?;
    match &project.channels[index].instrument {
        Instrument::Sampler { sample } => Ok((index, sample.clone())),
        _ => Err(invalid(
            command,
            format!("channel {number} is not a sampler channel"),
        )),
    }
}

/// A split ready to run: the sound and where its stems go.
struct Source {
    left: Vec<f32>,
    right: Vec<f32>,
    rate: f64,
    /// The stems' folder, created and so reserved.
    folder: PathBuf,
    /// The same folder relative to the project.
    relative: String,
}

impl Source {
    /// Reads channel `raw`'s sample and reserves a stems folder for it.
    fn read(project: &Project, command: &str, raw: &str) -> Result<Self, CommandError> {
        let (_, sample) = sampler(project, command, raw)?;
        let dir = project
            .path()
            .and_then(Path::parent)
            .ok_or_else(|| {
                invalid(
                    command,
                    "save the project first: stems are written beside it",
                )
            })?
            .to_path_buf();
        // A path from another system reads the file it translates to.
        let source = musiccraft_paths::Resolver::new()
            .resolve(&sample, Some(&dir))
            .map_or_else(|| sample.clone(), |p| p.to_string_lossy().into_owned());
        let buffer = SampleCache::new()
            .get(&source, Some(&dir), BUILTIN_RATE)
            .map_err(|e| invalid(command, format!("cannot read `{sample}`: {e}")))?;
        let stem = file_stem(&sample);
        let (folder, relative) = (1..)
            .map(|n| format!("stems/{stem}-{n}"))
            .map(|relative| (dir.join(&relative), relative))
            .find(|(folder, _)| !folder.exists())
            .unwrap_or_else(|| (dir.join("stems").join(&stem), format!("stems/{stem}")));
        std::fs::create_dir_all(&folder).map_err(|e| CommandError::Io(e.to_string()))?;
        Ok(Self {
            left: buffer.left().to_vec(),
            right: buffer.right().to_vec(),
            rate: buffer.sample_rate(),
            folder,
            relative,
        })
    }

    /// Splits the sound and writes the stems. Returns their paths relative
    /// to the project, in [`Stem::ALL`] order, or `None` if cancelled.
    fn split(&self, progress: &mut dyn FnMut(f32) -> bool) -> Result<Option<[String; 4]>, String> {
        let Ok(stems) = split(&self.left, &self.right, self.rate, progress) else {
            let _ = std::fs::remove_dir(&self.folder);
            return Ok(None);
        };
        self.write(&stems).map(Some)
    }

    fn write(&self, stems: &Stems) -> Result<[String; 4], String> {
        let rate = self.rate.round() as u32;
        for stem in Stem::ALL {
            let audio = stems.get(stem);
            let path = self.folder.join(format!("{}.wav", stem.name()));
            write_wav(&path, &audio.left, &audio.right, rate, WavFormat::Float32)
                .map_err(|e| e.to_string())?;
        }
        Ok(Stem::ALL.map(|stem| format!("{}/{}.wav", self.relative, stem.name())))
    }
}

/// A sample's file name without its folder and extension, safe for a folder
/// name.
fn file_stem(sample: &str) -> String {
    Path::new(sample.trim_start_matches(musiccraft_project::params::BUILTIN_PREFIX))
        .file_stem()
        .map_or_else(|| "sample".to_owned(), |s| s.to_string_lossy().into_owned())
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// The first of four neighbouring tracks below `track` with no clip
/// between `start` and `end`.
fn free_tracks(clips: &[Clip], track: u32, start: u32, end: u32) -> Option<u32> {
    let busy = |row: u32| {
        clips
            .iter()
            .any(|c| c.track == row && c.start < end && start < c.end())
    };
    (track + 1..=PLAYLIST_TRACKS.saturating_sub(4)).find(|&first| !(first..first + 4).any(busy))
}

/// Adds the four stem channels of channel `index` playing `paths`, and
/// stem clips under each of its audio clips (which are muted).
fn add_stems(
    project: &mut Project,
    command: &str,
    index: usize,
    paths: &[String; 4],
) -> Result<Outcome, CommandError> {
    let source = project.channels[index].clone();
    let old = serde_json::to_string(&project.playlist.clips)
        .map_err(|e| invalid(command, e.to_string()))?;
    let mut clips = project.playlist.clips.clone();
    let next_id = project.next_id();
    let ids: Vec<ChannelId> = paths
        .iter()
        .map(|_| ChannelId(project.alloc_id()))
        .collect();
    for i in 0..clips.len() {
        let clip = clips[i];
        if clip.audio() != Some(source.id) {
            continue;
        }
        let Some(first) = free_tracks(&clips, clip.track, clip.start, clip.end()) else {
            // A failed command leaves the project as it was.
            project.set_next_id(next_id);
            return Err(invalid(
                command,
                format!(
                    "no four free tracks below track {} for the stems",
                    clip.track + 1
                ),
            ));
        };
        for (row, id) in (first..).zip(&ids) {
            clips.push(Clip {
                source: ClipSource::Audio(*id),
                track: row,
                ..clip
            });
        }
        clips[i].muted = true;
    }
    for ((stem, path), id) in Stem::ALL.iter().zip(paths).zip(&ids) {
        project.channels.push(Channel {
            id: *id,
            name: format!("{} ({})", source.name, stem.name()),
            instrument: Instrument::Sampler {
                sample: path.clone(),
            },
            ..source.clone()
        });
    }
    project.playlist.clips = clips;
    project.playlist.sort_clips();
    let count = project.channels.len();
    let mut undo = vec![Invocation::new("playlist.set_clips", [old]).to_string()];
    undo.extend(
        (count - 3..=count)
            .rev()
            .map(|n| format!("channel.remove {n}")),
    );
    Ok(Outcome::undoable(
        format!(
            "split channel {} into stems: channels {} to {count}",
            index + 1,
            count - 3
        ),
        Invocation::new(BATCH, undo),
    ))
}

impl Command for SplitStemsCommand {
    fn name(&self) -> &'static str {
        "sample.split_stems"
    }

    fn summary(&self) -> &'static str {
        "Split a sampler channel's sound into vocals, drums, bass and other stems, each a new channel (audio clips get stem clips)."
    }

    fn usage(&self) -> &'static str {
        "sample.split_stems <channel>"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let [channel] = args else {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        };
        let source = Source::read(project, self.name(), channel)?;
        let paths = source
            .split(&mut |_| true)
            .map_err(CommandError::Io)?
            .ok_or_else(|| invalid(self.name(), "cancelled"))?;
        let (index, _) = sampler(project, self.name(), channel)?;
        add_stems(project, self.name(), index, &paths)
    }
}

impl Command for AddStemsCommand {
    fn name(&self) -> &'static str {
        "sample.add_stems"
    }

    fn summary(&self) -> &'static str {
        "Add the vocals, drums, bass and other stems of a sampler channel from existing files, each a new channel (audio clips get stem clips)."
    }

    fn usage(&self) -> &'static str {
        "sample.add_stems <channel> <vocals> <drums> <bass> <other>"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let [channel, vocals, drums, bass, other] = args else {
            return Err(CommandError::Usage {
                command: self.name().to_owned(),
                usage: self.usage().to_owned(),
            });
        };
        let (index, _) = sampler(project, self.name(), channel)?;
        let dir = project.path().and_then(Path::parent);
        let paths = [vocals, drums, bass, other].map(|p| (*p).clone());
        for path in &paths {
            let file = Path::new(path);
            let file = match dir {
                Some(dir) if file.is_relative() => dir.join(file),
                _ => file.to_path_buf(),
            };
            if !file.is_file() {
                return Err(invalid(self.name(), format!("no stem file `{path}`")));
            }
        }
        add_stems(project, self.name(), index, &paths)
    }
}

/// A split running on a worker thread, for the app: it reads the sound now,
/// splits and writes the stems in the background, and then hands back the
/// `sample.add_stems` invocation to run.
pub struct StemJob {
    channel: ChannelId,
    name: String,
    progress: Arc<AtomicU32>,
    cancel: Arc<AtomicBool>,
    done: Receiver<Result<Option<[String; 4]>, String>>,
}

impl StemJob {
    /// Starts splitting channel `number` (1-based).
    ///
    /// # Errors
    ///
    /// As `sample.split_stems`: not a sampler channel, an unsaved project or
    /// an unreadable sound.
    pub fn start(project: &Project, number: usize) -> Result<Self, CommandError> {
        let command = "sample.split_stems";
        let raw = number.to_string();
        let source = Source::read(project, command, &raw)?;
        let chosen = &project.channels[number - 1];
        let (id, name) = (chosen.id, chosen.name.clone());
        let progress = Arc::new(AtomicU32::new(0));
        let cancel = Arc::new(AtomicBool::new(false));
        let (send, done) = channel();
        let (shared, stop) = (Arc::clone(&progress), Arc::clone(&cancel));
        std::thread::Builder::new()
            .name("stems".to_owned())
            .spawn(move || {
                let result = source.split(&mut |done| {
                    shared.store(done.to_bits(), Ordering::Relaxed);
                    !stop.load(Ordering::Relaxed)
                });
                let _ = send.send(result);
            })
            .map_err(|e| CommandError::Io(e.to_string()))?;
        Ok(Self {
            channel: id,
            name,
            progress,
            cancel,
            done,
        })
    }

    /// The name of the channel being split.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How far the split is, 0 to 1.
    #[must_use]
    pub fn progress(&self) -> f32 {
        f32::from_bits(self.progress.load(Ordering::Relaxed))
    }

    /// Asks the split to stop; [`StemJob::poll`] then gives `Ok(None)`.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// `None` while the split runs; then the invocation that adds the stems
    /// to `project` (to the channel wherever it has moved), `Ok(None)` if the
    /// split was cancelled, or why it failed.
    #[must_use]
    pub fn poll(&self, project: &Project) -> Option<Result<Option<Invocation>, String>> {
        let result = match self.done.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err("the split stopped".to_owned()),
        };
        Some(result.and_then(|paths| {
            let Some(paths) = paths else {
                return Ok(None);
            };
            let number = project
                .channel_position(self.channel)
                .ok_or_else(|| format!("{} was removed before its stems were ready", self.name))?;
            Ok(Some(Invocation::new(
                "sample.add_stems",
                std::iter::once((number + 1).to_string()).chain(paths),
            )))
        }))
    }
}
