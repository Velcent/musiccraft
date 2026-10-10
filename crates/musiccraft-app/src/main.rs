//! MusicCraft desktop application: wires the command session, the engine bridge,
//! the audio device, MIDI input and the user interface together.

mod autosave;
mod midi;
mod plugins;
mod recorder;
mod screenshot;
mod take;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::time::{Duration, Instant};

use eframe::egui;
use musiccraft_audio::{AudioOutput, OutputDevice};
use musiccraft_commands::{Invocation, Session};
use musiccraft_engine::{EngineHandle, EngineMessage, engine_pair};
use musiccraft_project::Project;
use musiccraft_render::Bridge;
use musiccraft_ui::{Action, EngineStatus, Frame, History, Keymap, UiState};

/// How long a click-to-audition note sounds.
const SHORT_PREVIEW: Duration = Duration::from_millis(220);

const USAGE: &str = "\
usage: musiccraft [project.mcproj] [--keymap <file>] [--empty] [--open <windows>]
  --keymap <file>    use a keymap file instead of the default shortcuts
  --empty            start with an empty project instead of the drum starter kit
  --open <windows>   comma-separated windows to show at start:
                     channel_rack, piano_roll, playlist, mixer, browser,
                     channel_settings, automation";

/// Audio running on a device.
struct Audio {
    _output: AudioOutput,
    handle: EngineHandle,
    bridge: Bridge,
}

struct App {
    session: Session,
    ui: UiState,
    audio: Option<Audio>,
    audio_label: String,
    midi: Option<midi::MidiIn>,
    midi_label: String,
    selected_channel: Arc<AtomicUsize>,
    synced_revision: Option<u64>,
    pending_offs: Vec<(Instant, u16, u8)>,
    title: String,
    screenshot: Option<screenshot::Request>,
    autosave: autosave::Autosave,
    recorder: recorder::Recorder,
    midi_notes: Option<Receiver<midi::Played>>,
    /// Waveform overview of the selected channel's sample, keyed by sample.
    overview: Option<(String, Vec<(f32, f32)>)>,
    /// Overviews and durations (seconds) of samples played by audio clips.
    clip_audio: Vec<ClipAudio>,
    /// Audio input, opened the first time an audio take starts.
    input: Option<musiccraft_audio::AudioInput>,
    take: Option<take::Take>,
    takes: u32,
    plugins: plugins::PluginGlue,
}

/// Display data for a sample that audio clips play.
#[derive(Clone)]
struct ClipAudio {
    channel: musiccraft_project::ChannelId,
    sample: String,
    peaks: Vec<(f32, f32)>,
    seconds: f64,
}

/// Min/max pairs summarizing a sample for display.
fn overview(buffer: &musiccraft_dsp::SampleBuffer, points: usize) -> Vec<(f32, f32)> {
    let len = buffer.len();
    if len == 0 {
        return Vec::new();
    }
    let per = len.div_ceil(points).max(1);
    buffer
        .left()
        .chunks(per)
        .zip(buffer.right().chunks(per))
        .map(|(l, r)| {
            l.iter()
                .zip(r)
                .fold((0.0_f32, 0.0_f32), |(lo, hi), (a, b)| {
                    let m = (a + b) * 0.5;
                    (lo.min(m), hi.max(m))
                })
        })
        .collect()
}

/// A new project with a few drum channels, ready to program.
fn starter_project() -> Project {
    let mut session = Session::new(Project::default(), musiccraft_cli::full_registry());
    for line in [
        "channel.add_sampler builtin:kick",
        "channel.add_sampler builtin:clap",
        "channel.add_sampler builtin:hat",
        "channel.add_sampler builtin:snare",
    ] {
        let _ = session.run_line(line);
    }
    session.project().clone()
}

struct Options {
    project: Option<PathBuf>,
    keymap: Option<PathBuf>,
    empty: bool,
    open: Option<Vec<String>>,
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options {
        project: None,
        keymap: None,
        empty: false,
        open: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--keymap" => options.keymap = Some(args.next().ok_or(USAGE)?.into()),
            "--empty" => options.empty = true,
            "--open" => {
                let list = args.next().ok_or(USAGE)?;
                options.open = Some(list.split(',').map(|w| w.trim().to_owned()).collect());
            }
            "-h" | "--help" => return Err(USAGE.to_owned()),
            other if other.starts_with('-') => {
                return Err(format!("unknown option {other}\n{USAGE}"));
            }
            path => options.project = Some(path.into()),
        }
    }
    Ok(options)
}

impl App {
    fn new(options: &Options, ctx: &egui::Context) -> Self {
        let mut status = None;
        let keymap = match &options.keymap {
            Some(path) => match std::fs::read_to_string(path)
                .map_err(|e| e.to_string())
                .and_then(|text| Keymap::parse(&text))
            {
                Ok(keymap) => keymap,
                Err(err) => {
                    status = Some(format!("keymap {}: {err}; using defaults", path.display()));
                    Keymap::default_keymap()
                }
            },
            None => Keymap::default_keymap(),
        };
        let project = if options.empty {
            Project::default()
        } else {
            starter_project()
        };
        let mut session = Session::new(project, musiccraft_cli::full_registry());
        if let Some(path) = &options.project {
            let load = Invocation::new("project.load", [path.display().to_string()]);
            if let Err(err) = session.run(&load) {
                status = Some(err.to_string());
            }
        }

        let selected_channel = Arc::new(AtomicUsize::new(0));
        let (played_tx, played_rx) = channel();
        let (audio, audio_label, midi, midi_label) = match OutputDevice::default_device() {
            Ok(device) => {
                let label = format!("{} · {} Hz", device.name(), device.sample_rate());
                let rate = device.sample_rate();
                let (engine, mut handle) = engine_pair(rate);
                let live = handle.take_live_sender();
                let meters = Arc::clone(handle.meters());
                match device.start(engine) {
                    Ok(output) => {
                        let connected = live.map(|l| {
                            midi::connect(
                                l,
                                Arc::clone(&selected_channel),
                                meters,
                                played_tx,
                                ctx.clone(),
                            )
                        });
                        let (midi, midi_label) = match connected {
                            Some(Ok(midi)) => {
                                let label = format!("MIDI: {}", midi.name);
                                (Some(midi), label)
                            }
                            Some(Err(err)) => (None, format!("MIDI: {err}")),
                            None => (None, "MIDI: unavailable".to_owned()),
                        };
                        (
                            Some(Audio {
                                _output: output,
                                handle,
                                bridge: Bridge::with_plugins(rate, musiccraft_cli::plugin_host()),
                            }),
                            label,
                            midi,
                            midi_label,
                        )
                    }
                    Err(err) => (None, format!("no audio: {err}"), None, String::new()),
                }
            }
            Err(err) => (None, format!("no audio: {err}"), None, String::new()),
        };

        let revision = session.revision();
        let mut ui = UiState::new(keymap);
        ui.autosave_path = autosave::path()
            .filter(|p| p.is_file())
            .map(|p| p.display().to_string());
        if let Some(windows) = &options.open {
            let w = &mut ui.windows;
            w.channel_rack = windows.iter().any(|n| n == "channel_rack");
            w.piano_roll = windows.iter().any(|n| n == "piano_roll");
            w.playlist = windows.iter().any(|n| n == "playlist");
            w.mixer = windows.iter().any(|n| n == "mixer");
            w.browser = windows.iter().any(|n| n == "browser");
            w.channel_settings = windows.iter().any(|n| n == "channel_settings");
            if windows.iter().any(|n| n == "automation") {
                ui.automation_editor = session.project().automations.first().map(|a| a.id);
            }
        }
        if let Some(text) = status {
            ui.set_status(text, true);
        } else {
            ui.set_status("Ready. Space plays, F5–F9 open windows.", false);
        }
        Self {
            session,
            ui,
            audio,
            audio_label,
            midi,
            midi_label,
            selected_channel,
            synced_revision: None,
            pending_offs: Vec::new(),
            title: String::new(),
            screenshot: screenshot::Request::from_env(),
            autosave: autosave::Autosave::new(revision),
            recorder: recorder::Recorder::default(),
            midi_notes: Some(played_rx),
            overview: None,
            clip_audio: Vec::new(),
            input: None,
            take: None,
            takes: 0,
            plugins: plugins::PluginGlue::new(musiccraft_cli::plugin_host()),
        }
    }

    fn engine_status(&mut self) -> EngineStatus {
        let tracks = self.session.project().mixer.len();
        let mut status = EngineStatus {
            audio: self.audio_label.clone(),
            midi: self.midi_label.clone(),
            ..EngineStatus::default()
        };
        if let Some(audio) = &self.audio {
            let meters = audio.handle.meters();
            status.position_ticks = meters.position_ticks();
            status.playing = meters.is_playing();
            status.voices = meters.active_voices();
            status.peaks = (0..tracks).map(|t| meters.take_peak(t)).collect();
            status.load = meters.load();
            let mut scope = vec![0.0; 256];
            meters.scope(&mut scope);
            status.scope = scope;
        } else {
            status.peaks = vec![(0.0, 0.0); tracks];
        }
        status
    }

    fn send(&mut self, message: EngineMessage) {
        if let Some(audio) = &mut self.audio {
            let _ = audio.handle.send(message);
        }
    }

    fn apply(&mut self, ctx: &egui::Context, action: Action) {
        let result = match action {
            Action::Run(invocation) => {
                let bridge = self.audio.as_mut().map(|a| &mut a.bridge);
                self.plugins.before(&invocation, bridge, &mut self.session);
                self.session.run(&invocation)
            }
            Action::Coalesced(invocation, key) => self.session.run_coalesced(&invocation, key),
            Action::Preview { channel, key, on } => {
                let channel = u16::try_from(channel).unwrap_or(0);
                self.send(if on {
                    EngineMessage::NoteOn {
                        channel,
                        key,
                        velocity: 0.8,
                    }
                } else {
                    EngineMessage::NoteOff { channel, key }
                });
                return;
            }
            Action::Perform { channel, key, on } => {
                let channel16 = u16::try_from(channel).unwrap_or(0);
                self.send(if on {
                    EngineMessage::NoteOn {
                        channel: channel16,
                        key,
                        velocity: 0.8,
                    }
                } else {
                    EngineMessage::NoteOff {
                        channel: channel16,
                        key,
                    }
                });
                self.record(key, if on { 100 } else { 0 }, None);
                return;
            }
            Action::PreviewShort { channel, key } => {
                let channel = u16::try_from(channel).unwrap_or(0);
                self.send(EngineMessage::NoteOn {
                    channel,
                    key,
                    velocity: 0.8,
                });
                self.pending_offs
                    .push((Instant::now() + SHORT_PREVIEW, channel, key));
                return;
            }
            Action::PreviewSample(sample) => {
                let loaded = self
                    .audio
                    .as_mut()
                    .map(|audio| audio.bridge.sample(&sample, self.session.project()));
                match loaded {
                    Some(Ok(buffer)) => {
                        self.send(EngineMessage::PlayPreview(Box::new(
                            musiccraft_engine::Preview::new(buffer, 0.8),
                        )));
                    }
                    Some(Err(err)) => self.ui.set_status(format!("cannot preview: {err}"), true),
                    None => {}
                }
                return;
            }
            Action::DropAudio {
                sample,
                track,
                start,
            } => {
                let loaded = self
                    .audio
                    .as_mut()
                    .map(|audio| audio.bridge.sample(&sample, self.session.project()));
                let seconds = match loaded {
                    Some(Ok(buffer)) => buffer.duration_seconds(),
                    Some(Err(err)) => {
                        self.ui
                            .set_status(format!("cannot load {sample}: {err}"), true);
                        return;
                    }
                    None => 4.0 * 60.0 / self.session.project().tempo_bpm(),
                };
                let project = self.session.project();
                let ticks_per_second =
                    project.tempo_bpm() / 60.0 * f64::from(musiccraft_project::params::PPQ);
                let length = (seconds * ticks_per_second).ceil().max(1.0) as u32;
                let number = project.channels.len() + 1;
                let batch = Invocation::new(
                    musiccraft_commands::BATCH,
                    [
                        Invocation::new("channel.add_sampler", [sample.clone()]).to_string(),
                        format!("channel.set_param {number} one_shot 0"),
                        format!(
                            "playlist.add_audio_clip {number} {} {start}t {length}t",
                            track + 1
                        ),
                    ],
                );
                self.session.run(&batch)
            }
            Action::Seek(ticks) => {
                self.send(EngineMessage::Seek(ticks));
                return;
            }
            Action::ReloadSamples => {
                if let Some(audio) = &mut self.audio {
                    audio.bridge.reload_samples();
                    self.synced_revision = None;
                }
                self.ui.set_status("samples reloaded", false);
                return;
            }
            Action::PluginEditor { instance, open } => {
                let bridge = self.audio.as_mut().map(|a| &mut a.bridge);
                if let Err(err) =
                    self.plugins
                        .set_editor(bridge, self.session.project(), instance, open)
                {
                    self.ui.set_status(format!("plugin window: {err}"), true);
                }
                return;
            }
            Action::RescanPlugins => {
                let bridge = self.audio.as_mut().map(|a| &mut a.bridge);
                let message = self.plugins.rescan(bridge);
                self.synced_revision = None;
                self.ui.set_status(message, false);
                return;
            }
            Action::Quit => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
        };
        match result {
            Ok(outcome) => self.ui.set_status(outcome.message, false),
            Err(err) => self.ui.set_status(err.to_string(), true),
        }
    }

    /// Feeds a live note into the recorder; velocity 0 means note off.
    /// `position` is the engine position when the note happened, if known.
    fn record(&mut self, key: u8, velocity: u8, position: Option<f64>) {
        let Some(audio) = &self.audio else {
            return;
        };
        let meters = audio.handle.meters();
        if !self.ui.record || !meters.is_playing() {
            self.recorder.clear();
            return;
        }
        let position = position.unwrap_or_else(|| meters.position_ticks());
        if velocity > 0 {
            self.recorder.note_on(key, velocity, position);
            return;
        }
        let project = self.session.project();
        let pattern_ticks = project.current_pattern().length_ticks();
        let loop_ticks = match project.play_mode {
            musiccraft_project::PlayMode::Pattern => pattern_ticks,
            musiccraft_project::PlayMode::Song => project.song_length_ticks(),
        };
        let Some(note) = self
            .recorder
            .note_off(key, position, loop_ticks, pattern_ticks)
        else {
            return;
        };
        let channel = self.ui.selected_channel;
        if channel >= project.channels.len() {
            return;
        }
        let invocation = Invocation::new(
            "note.add",
            [
                (channel + 1).to_string(),
                format!("{}t", note.start),
                format!("{}t", note.length),
                note.key.to_string(),
                note.velocity.to_string(),
            ],
        );
        match self.session.run(&invocation) {
            Ok(outcome) => self
                .ui
                .set_status(format!("recorded: {}", outcome.message), false),
            Err(err) => self.ui.set_status(err.to_string(), true),
        }
    }

    /// Keeps the waveform overview in step with the selected channel's sample.
    fn update_overview(&mut self) {
        let project = self.session.project();
        let sample = match project
            .channels
            .get(self.ui.selected_channel)
            .map(|c| &c.instrument)
        {
            Some(musiccraft_project::Instrument::Sampler { sample }) => sample.clone(),
            _ => {
                self.overview = None;
                return;
            }
        };
        if self
            .overview
            .as_ref()
            .is_some_and(|(key, _)| *key == sample)
        {
            return;
        }
        let Some(audio) = &mut self.audio else {
            return;
        };
        let points = match audio.bridge.sample(&sample, self.session.project()) {
            Ok(buffer) => overview(&buffer, 400),
            Err(_) => Vec::new(),
        };
        self.overview = Some((sample, points));
    }

    /// Keeps waveform data for every channel that audio clips play.
    fn update_clip_audio(&mut self) {
        let project = self.session.project();
        let mut wanted: Vec<(musiccraft_project::ChannelId, String)> = project
            .playlist
            .clips
            .iter()
            .filter_map(|c| c.audio())
            .filter_map(|id| {
                let channel = project.channels.get(project.channel_position(id)?)?;
                match &channel.instrument {
                    musiccraft_project::Instrument::Sampler { sample } => {
                        Some((id, sample.clone()))
                    }
                    musiccraft_project::Instrument::Synth
                    | musiccraft_project::Instrument::Plugin { .. } => None,
                }
            })
            .collect();
        wanted.sort();
        wanted.dedup();
        let current: Vec<(musiccraft_project::ChannelId, String)> = self
            .clip_audio
            .iter()
            .map(|c| (c.channel, c.sample.clone()))
            .collect();
        if current == wanted {
            return;
        }
        let Some(audio) = &mut self.audio else {
            return;
        };
        let mut fresh = Vec::new();
        for (id, sample) in wanted {
            if let Some(existing) = self
                .clip_audio
                .iter()
                .find(|c| c.channel == id && c.sample == sample)
            {
                fresh.push(existing.clone());
            } else if let Ok(buffer) = audio.bridge.sample(&sample, self.session.project()) {
                fresh.push(ClipAudio {
                    channel: id,
                    peaks: overview(&buffer, 600),
                    seconds: buffer.duration_seconds(),
                    sample,
                });
            }
        }
        self.clip_audio = fresh;
    }

    /// Starts, feeds and finishes audio takes as recording turns on and off.
    fn record_audio(&mut self) {
        let Some(audio) = &self.audio else {
            return;
        };
        let meters = audio.handle.meters();
        let song = self.session.project().play_mode == musiccraft_project::PlayMode::Song;
        let wanted = self.ui.record && self.ui.record_audio && meters.is_playing() && song;
        if wanted && self.take.is_none() {
            if self.input.is_none() {
                match musiccraft_audio::AudioInput::open_default() {
                    Ok(input) => self.input = Some(input),
                    Err(err) => {
                        self.ui.record_audio = false;
                        self.ui.set_status(format!("no audio input: {err}"), true);
                        return;
                    }
                }
            }
            if let Some(input) = &mut self.input {
                input.clear();
                self.take = Some(take::Take {
                    start: meters.position_ticks(),
                    samples: Vec::new(),
                });
                self.ui
                    .set_status(format!("recording from {}", input.name()), false);
            }
        }
        if let (Some(take), Some(input)) = (&mut self.take, &mut self.input) {
            input.drain(&mut take.samples);
        }
        if !wanted {
            if let (Some(take), Some(input)) = (self.take.take(), &self.input) {
                self.takes += 1;
                let name = format!(
                    "take-{}-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |d| d.as_secs()),
                    self.takes
                );
                let finished = take::finish(
                    take,
                    input.channels(),
                    input.sample_rate(),
                    self.session.project(),
                    &name,
                );
                match finished {
                    Ok(Some(invocation)) => match self.session.run(&invocation) {
                        Ok(_) => self.ui.set_status(format!("recorded {name}"), false),
                        Err(err) => self.ui.set_status(err.to_string(), true),
                    },
                    Ok(None) => {}
                    Err(err) => self.ui.set_status(format!("recording failed: {err}"), true),
                }
            }
        }
    }

    fn drain_midi(&mut self) {
        let notes: Vec<midi::Played> = self
            .midi_notes
            .as_ref()
            .map(|rx| rx.try_iter().collect())
            .unwrap_or_default();
        for (key, velocity, position) in notes {
            self.record(key, velocity, Some(position));
        }
    }

    fn sync(&mut self) {
        let revision = self.session.revision();
        if self.synced_revision == Some(revision) {
            if let Some(audio) = &mut self.audio {
                audio.handle.collect_garbage();
            }
            return;
        }
        if let Some(audio) = &mut self.audio {
            let report = audio.bridge.sync(self.session.project(), &mut audio.handle);
            if let Some(warning) = report.warnings.first() {
                self.ui.set_status(warning.clone(), true);
            }
            if report.dropped > 0 {
                // Queue full: try again next frame.
                return;
            }
        }
        self.synced_revision = Some(revision);
    }

    fn release_due_previews(&mut self) {
        let now = Instant::now();
        let (due, waiting): (Vec<_>, Vec<_>) =
            self.pending_offs.drain(..).partition(|(t, _, _)| *t <= now);
        self.pending_offs = waiting;
        for (_, channel, key) in due {
            self.send(EngineMessage::NoteOff { channel, key });
        }
    }

    fn update_title(&mut self, ctx: &egui::Context) {
        let project = self.session.project();
        let title = format!("MusicCraft — {}", project.name());
        if title != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.clone()));
            self.title = title;
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let engine = self.engine_status();
        let history = History {
            undo: self.session.undo_label().map(ToString::to_string),
            can_redo: self.session.redo_depth() > 0,
        };
        self.update_overview();
        self.update_clip_audio();
        let ticks_per_second =
            self.session.project().tempo_bpm() / 60.0 * f64::from(musiccraft_project::params::PPQ);
        let clip_waveforms: Vec<(musiccraft_project::ChannelId, Vec<(f32, f32)>)> = self
            .clip_audio
            .iter()
            .map(|c| (c.channel, c.peaks.clone()))
            .collect();
        let clip_durations: Vec<(musiccraft_project::ChannelId, u32)> = self
            .clip_audio
            .iter()
            .map(|c| (c.channel, (c.seconds * ticks_per_second).ceil() as u32))
            .collect();
        let plugins = {
            let bridge = self.audio.as_mut().map(|a| &mut a.bridge);
            self.plugins.view(bridge, self.session.project(), &self.ui)
        };
        let actions = {
            let frame = Frame {
                project: self.session.project(),
                engine: &engine,
                history: &history,
                waveform: self.overview.as_ref().map(|(_, points)| points.as_slice()),
                clip_waveforms: &clip_waveforms,
                clip_durations: &clip_durations,
                plugins: &plugins,
            };
            musiccraft_ui::show(ui, &mut self.ui, &frame)
        };
        for action in actions {
            self.apply(&ctx, action);
        }
        let editor_open = {
            let bridge = self.audio.as_mut().map(|a| &mut a.bridge);
            self.plugins.poll(bridge, &mut self.session)
        };
        self.drain_midi();
        self.record_audio();
        self.selected_channel
            .store(self.ui.selected_channel, Ordering::Relaxed);
        self.sync();
        self.release_due_previews();
        self.update_title(&ctx);
        match self
            .autosave
            .tick(self.session.project(), self.session.revision())
        {
            Some(Ok(message)) => {
                if !self.ui.status.error {
                    self.ui.set_status(message, false);
                }
                self.ui.autosave_path = autosave::path().map(|p| p.display().to_string());
            }
            Some(Err(err)) => self.ui.set_status(format!("autosave failed: {err}"), true),
            None => {}
        }
        let busy = engine.playing
            || !self.pending_offs.is_empty()
            || engine.voices > 0
            || self.take.is_some()
            || editor_open;
        let meters_moving = self.ui.meters.iter().any(|(l, r)| *l > 1e-4 || *r > 1e-4);
        if busy || meters_moving {
            ctx.request_repaint_after(Duration::from_millis(16));
        } else {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
        let _ = &self.midi;
        if let Some(request) = &mut self.screenshot {
            request.tick(&ctx);
        }
    }
}

fn main() -> eframe::Result {
    let options = match parse_args() {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };
    let native = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1360.0, 860.0])
            .with_min_inner_size([800.0, 500.0])
            .with_title("MusicCraft")
            .with_app_id("musiccraft"),
        ..Default::default()
    };
    eframe::run_native(
        "MusicCraft",
        native,
        Box::new(move |creation| {
            musiccraft_ui::apply_theme(&creation.egui_ctx);
            Ok(Box::new(App::new(&options, &creation.egui_ctx)))
        }),
    )
}
