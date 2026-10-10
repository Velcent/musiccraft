use std::path::Path;
use std::sync::Arc;

use musiccraft_dsp::{Effect, SampleBuffer};
use musiccraft_engine::{
    ChannelParams, EffectParams, EngineHandle, EngineMessage, Graph, Instrument, MAX_SENDS,
    NoteEvent, SendParams, Sequence, TrackParams,
};
use musiccraft_params::{BUILTIN_PREFIX, EffectKind};
use musiccraft_project::{Channel, MixerTrack, Project};
use musiccraft_samples::SampleCache;

use crate::build_sequence;

/// The parts of a project that require a new [`Graph`] when they change.
#[derive(Debug, Clone, PartialEq)]
struct Structure {
    /// `None` for a synth, otherwise the resolved sample reference.
    channels: Vec<Option<String>>,
    tracks: Vec<Vec<EffectKind>>,
}

/// Everything the engine was last told.
#[derive(Debug, Clone, PartialEq)]
struct Desc {
    structure: Structure,
    channels: Vec<ChannelParams>,
    tracks: Vec<TrackParams>,
    effects: Vec<Vec<EffectParams>>,
    events: Vec<NoteEvent>,
    length: u64,
    tempo: f64,
    playing: bool,
    metronome: bool,
}

/// What a [`Bridge::sync`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SyncReport {
    /// Number of messages sent.
    pub messages: usize,
    /// Whether the graph was rebuilt.
    pub rebuilt_graph: bool,
    /// Whether a new sequence was sent.
    pub new_sequence: bool,
    /// Problems such as samples that failed to load (they play silence).
    pub warnings: Vec<String>,
    /// Messages that did not fit in the queue; the next sync retries.
    pub dropped: usize,
}

fn sample_key(sample: &str, base: Option<&Path>) -> String {
    if sample.starts_with(BUILTIN_PREFIX) {
        return sample.to_owned();
    }
    match base {
        Some(dir) if Path::new(sample).is_relative() => {
            dir.join(sample).to_string_lossy().into_owned()
        }
        _ => sample.to_owned(),
    }
}

fn channel_params(channel: &Channel) -> ChannelParams {
    ChannelParams {
        volume: channel.volume,
        pan: channel.pan,
        mute: channel.mute,
        track: u16::try_from(channel.mixer_track).unwrap_or(0),
        instrument: channel.params.0,
    }
}

fn track_params(track: &MixerTrack, index: usize, audible: Option<&[bool]>) -> TrackParams {
    // Solo applies to inserts; the master is never silenced by another solo.
    let soloed_out = index > 0 && audible.is_some_and(|a| !a.get(index).copied().unwrap_or(false));
    let mut params = TrackParams {
        volume: track.volume,
        pan: track.pan,
        mute: track.mute || soloed_out,
        to_master: track.to_master || index == 0,
        ..TrackParams::default()
    };
    let sends = track
        .sends
        .iter()
        .filter(|s| s.target > index)
        .take(MAX_SENDS);
    for (slot, send) in params.sends.iter_mut().zip(sends) {
        *slot = SendParams {
            target: u16::try_from(send.target).unwrap_or(u16::MAX),
            level: send.level,
        };
        params.send_count += 1;
    }
    params
}

/// When any insert is soloed: which tracks stay audible. A soloed track keeps
/// everything that feeds it and everything it feeds through sends.
fn solo_audible(mixer: &[MixerTrack]) -> Option<Vec<bool>> {
    if !mixer.iter().skip(1).any(|t| t.solo) {
        return None;
    }
    let edges: Vec<(usize, usize)> = mixer
        .iter()
        .enumerate()
        .flat_map(|(i, t)| t.sends.iter().map(move |s| (i, s.target)))
        .collect();
    let mut audible: Vec<bool> = mixer.iter().map(|t| t.solo).collect();
    // Downstream: sends point to higher indices, so one forward pass suffices.
    for i in 0..mixer.len() {
        if audible[i] {
            for (from, to) in &edges {
                if *from == i && *to < audible.len() {
                    audible[*to] = true;
                }
            }
        }
    }
    // Upstream from the originally soloed tracks: one backward pass.
    let mut feeds: Vec<bool> = mixer.iter().map(|t| t.solo).collect();
    for i in (0..mixer.len()).rev() {
        if feeds[i] {
            for (from, to) in &edges {
                if *to == i {
                    feeds[*from] = true;
                }
            }
        }
    }
    Some(audible.iter().zip(&feeds).map(|(a, f)| *a || *f).collect())
}

fn describe(project: &Project) -> Desc {
    let base = project.path().and_then(Path::parent);
    let audible = solo_audible(&project.mixer);
    let sequence = build_sequence(project);
    Desc {
        structure: Structure {
            channels: project
                .channels
                .iter()
                .map(|c| match &c.instrument {
                    musiccraft_project::Instrument::Sampler { sample } => {
                        Some(sample_key(sample, base))
                    }
                    musiccraft_project::Instrument::Synth => None,
                })
                .collect(),
            tracks: project
                .mixer
                .iter()
                .map(|t| t.effects.iter().map(|e| e.kind).collect())
                .collect(),
        },
        channels: project.channels.iter().map(channel_params).collect(),
        tracks: project
            .mixer
            .iter()
            .enumerate()
            .map(|(i, t)| track_params(t, i, audible.as_deref()))
            .collect(),
        effects: project
            .mixer
            .iter()
            .map(|t| {
                t.effects
                    .iter()
                    .map(|e| EffectParams {
                        enabled: e.enabled,
                        values: e.values,
                    })
                    .collect()
            })
            .collect(),
        events: sequence.events().to_vec(),
        length: sequence.length(),
        tempo: project.tempo_bpm(),
        playing: project.transport().is_playing(),
        metronome: project.metronome,
    }
}

/// Keeps an engine in step with a project. Lives on the main thread.
pub struct Bridge {
    sample_rate: u32,
    samples: SampleCache,
    last: Option<Desc>,
}

impl Bridge {
    /// A bridge for an engine running at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            samples: SampleCache::new(),
            last: None,
        }
    }

    /// Forgets what the engine was told, so the next sync sends everything.
    pub fn invalidate(&mut self) {
        self.last = None;
    }

    /// Forgets decoded samples and forces a full resync (after files changed).
    pub fn reload_samples(&mut self) {
        self.samples.clear();
        self.last = None;
    }

    /// Loads a channel's sample, substituting silence (and a warning) on error.
    fn load(&mut self, key: &str, warnings: &mut Vec<String>) -> Arc<SampleBuffer> {
        match self.samples.get(key, None, self.sample_rate) {
            Ok(buffer) => buffer,
            Err(err) => {
                warnings.push(format!("sample `{key}`: {err}"));
                Arc::new(SampleBuffer::silent(f64::from(self.sample_rate)))
            }
        }
    }

    fn build_graph(&mut self, project: &Project, desc: &Desc, warnings: &mut Vec<String>) -> Graph {
        let mut graph = Graph::new();
        for (key, params) in desc.structure.channels.iter().zip(&desc.channels) {
            let instrument = match key {
                Some(key) => Instrument::Sampler(self.load(key, warnings)),
                None => Instrument::Synth,
            };
            graph.add_channel(instrument, *params);
        }
        let sr = self.sample_rate as f32;
        for (i, (track, params)) in project.mixer.iter().zip(&desc.tracks).enumerate() {
            let effects = track
                .effects
                .iter()
                .map(|e| {
                    (
                        Effect::new(e.kind, sr),
                        EffectParams {
                            enabled: e.enabled,
                            values: e.values,
                        },
                    )
                })
                .collect();
            if i == 0 {
                graph.set_master(*params, effects);
            } else {
                graph.add_track(*params, effects);
            }
        }
        graph
    }

    /// Brings the engine up to date with `project`.
    pub fn sync(&mut self, project: &Project, handle: &mut EngineHandle) -> SyncReport {
        handle.collect_garbage();
        let desc = describe(project);
        let mut report = SyncReport::default();
        let mut messages = Vec::new();
        let previous = self.last.take();

        let rebuild = previous
            .as_ref()
            .is_none_or(|p| p.structure != desc.structure);
        if rebuild {
            let graph = self.build_graph(project, &desc, &mut report.warnings);
            messages.push(EngineMessage::SetGraph(Box::new(graph)));
            report.rebuilt_graph = true;
        } else if let Some(prev) = &previous {
            for (i, (now, before)) in desc.channels.iter().zip(&prev.channels).enumerate() {
                if now != before {
                    messages.push(EngineMessage::Channel {
                        channel: u16::try_from(i).unwrap_or(u16::MAX),
                        params: *now,
                    });
                }
            }
            for (i, (now, before)) in desc.tracks.iter().zip(&prev.tracks).enumerate() {
                if now != before {
                    messages.push(EngineMessage::Track {
                        track: u16::try_from(i).unwrap_or(u16::MAX),
                        params: *now,
                    });
                }
            }
            for (t, (now, before)) in desc.effects.iter().zip(&prev.effects).enumerate() {
                for (s, (now, before)) in now.iter().zip(before).enumerate() {
                    if now != before {
                        messages.push(EngineMessage::Effect {
                            track: u16::try_from(t).unwrap_or(u16::MAX),
                            slot: u8::try_from(s).unwrap_or(u8::MAX),
                            params: *now,
                        });
                    }
                }
            }
        }
        let sequence_changed = previous
            .as_ref()
            .is_none_or(|p| p.events != desc.events || p.length != desc.length);
        if sequence_changed {
            messages.push(EngineMessage::SetSequence(Box::new(Sequence::new(
                desc.events.clone(),
                desc.length,
            ))));
            report.new_sequence = true;
        }
        if previous.as_ref().is_none_or(|p| p.tempo != desc.tempo) {
            messages.push(EngineMessage::SetTempo(desc.tempo));
        }
        if previous
            .as_ref()
            .is_none_or(|p| p.metronome != desc.metronome)
        {
            messages.push(EngineMessage::SetMetronome(desc.metronome));
        }
        if previous.as_ref().is_none_or(|p| p.playing != desc.playing) {
            messages.push(if desc.playing {
                EngineMessage::Play
            } else {
                EngineMessage::Stop
            });
        }

        let mut complete = true;
        for message in messages {
            if handle.send(message).is_ok() {
                report.messages += 1;
            } else {
                report.dropped += 1;
                complete = false;
            }
        }
        // On a full queue, forget the state so the next sync resends everything.
        self.last = complete.then_some(desc);
        report
    }

    /// The decoded sample a channel plays, loading it if needed.
    ///
    /// # Errors
    ///
    /// Returns a message if the sample cannot be loaded.
    pub fn sample(&mut self, sample: &str, project: &Project) -> Result<Arc<SampleBuffer>, String> {
        let key = sample_key(sample, project.path().and_then(Path::parent));
        self.samples
            .get(&key, None, self.sample_rate)
            .map_err(|e| e.to_string())
    }

    /// Sample rate this bridge prepares data for.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
