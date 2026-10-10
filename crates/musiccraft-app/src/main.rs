//! MusicCraft desktop application: wires the command session, the engine bridge,
//! the audio device, MIDI input and the user interface together.

mod autosave;
mod midi;
mod screenshot;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
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
                     channel_rack, piano_roll, playlist, mixer, browser";

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
    fn new(options: &Options) -> Self {
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
        let (audio, audio_label, midi, midi_label) = match OutputDevice::default_device() {
            Ok(device) => {
                let label = format!("{} · {} Hz", device.name(), device.sample_rate());
                let rate = device.sample_rate();
                let (engine, mut handle) = engine_pair(rate);
                let live = handle.take_live_sender();
                match device.start(engine) {
                    Ok(output) => {
                        let (midi, midi_label) =
                            match live.map(|l| midi::connect(l, Arc::clone(&selected_channel))) {
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
                                bridge: Bridge::new(rate),
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
            Action::Run(invocation) => self.session.run(&invocation),
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
            Action::ReloadSamples => {
                if let Some(audio) = &mut self.audio {
                    audio.bridge.reload_samples();
                    self.synced_revision = None;
                }
                self.ui.set_status("samples reloaded", false);
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
        let actions = {
            let frame = Frame {
                project: self.session.project(),
                engine: &engine,
                history: &history,
            };
            musiccraft_ui::show(ui, &mut self.ui, &frame)
        };
        for action in actions {
            self.apply(&ctx, action);
        }
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
        let busy = engine.playing || !self.pending_offs.is_empty() || engine.voices > 0;
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
            Ok(Box::new(App::new(&options)))
        }),
    )
}
