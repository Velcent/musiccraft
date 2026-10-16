//! Bridge and render tests against an in-process fake plugin host.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use musiccraft_commands::{Registry, Session};
use musiccraft_engine::{External, ExternalTransport, engine_pair};
use musiccraft_project::Project;

use crate::plugins::{
    LivePlugin, PluginEvent, PluginFactory, PluginHost, PluginInfo, PluginInstance, PluginKind,
    PluginParam, PluginRequest,
};
use crate::{Bridge, RenderOptions, register_with, render_with};

/// Parameter id of the fake plugins' gain.
const GAIN: u32 = 7;

/// What the fake host saw, shared with the test.
#[derive(Debug, Default)]
struct Seen {
    created: AtomicU32,
    states: Mutex<Vec<Vec<u8>>>,
    /// Events the next poll of any instance reports.
    events: Mutex<Vec<PluginEvent>>,
}

#[derive(Debug)]
struct FakeHost {
    seen: Arc<Seen>,
}

#[derive(Debug)]
struct FakeInstance {
    seen: Arc<Seen>,
    gain: f64,
}

/// `test.tone` plays a constant 0.25 while any note is held; `test.gain`
/// multiplies its input. Both scale by the gain parameter.
#[derive(Debug)]
struct FakeProcessor {
    instrument: bool,
    gain: f32,
    held: u32,
    remaining: Option<u64>,
}

impl External for FakeProcessor {
    fn note_on(&mut self, _delay: u32, _key: u8, _velocity: f32, _pan: f32, gate: Option<u64>) {
        self.held += 1;
        self.remaining = gate;
    }

    fn note_off(&mut self, _delay: u32, _key: u8) {
        self.held = self.held.saturating_sub(1);
    }

    fn all_notes_off(&mut self) {
        self.held = 0;
    }

    fn set_param(&mut self, id: u32, value: f64) {
        if id == GAIN {
            self.gain = value as f32;
        }
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], _transport: &ExternalTransport) {
        if self.instrument {
            for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                if let Some(n) = &mut self.remaining {
                    if *n == 0 {
                        self.held = 0;
                        self.remaining = None;
                    } else {
                        *n -= 1;
                    }
                }
                if self.held > 0 {
                    *l += 0.25;
                    *r += 0.25;
                }
            }
        }
        for s in left.iter_mut().chain(right.iter_mut()) {
            *s *= self.gain;
        }
    }

    fn stop(&mut self) {}
}

impl PluginInstance for FakeInstance {
    fn params(&self) -> Vec<PluginParam> {
        vec![PluginParam {
            id: GAIN,
            name: "Gain".into(),
            module: String::new(),
            min: 0.0,
            max: 2.0,
            default: 1.0,
            stepped: false,
            hidden: false,
            read_only: false,
        }]
    }

    fn param_value(&self, id: u32) -> Option<f64> {
        (id == GAIN).then_some(self.gain)
    }

    fn format_param(&self, _id: u32, value: f64) -> Option<String> {
        Some(format!("{value:.2}x"))
    }

    fn save_state(&mut self) -> Option<Vec<u8>> {
        Some(b"fake".to_vec())
    }

    fn poll(&mut self) -> Vec<PluginEvent> {
        std::mem::take(&mut *self.seen.events.lock().unwrap())
    }

    fn has_editor(&self) -> bool {
        false
    }

    fn editor_open(&self) -> bool {
        false
    }

    fn set_editor_open(&mut self, _open: bool, _title: &str) -> Result<(), String> {
        Err("no editor".into())
    }
}

impl PluginFactory for FakeHost {
    fn catalog(&self) -> Vec<PluginInfo> {
        ["test.tone", "test.gain"]
            .iter()
            .map(|id| PluginInfo {
                id: (*id).to_owned(),
                name: id.to_uppercase(),
                vendor: "Test".into(),
                version: "1".into(),
                kind: if *id == "test.tone" {
                    PluginKind::Instrument
                } else {
                    PluginKind::Effect
                },
                path: "/fake.clap".into(),
            })
            .collect()
    }

    fn rescan(&self) {}

    fn instantiate(&self, request: &PluginRequest<'_>) -> Result<LivePlugin, String> {
        let instrument = match request.id {
            "test.tone" => true,
            "test.gain" => false,
            other => return Err(format!("`{other}` is not installed")),
        };
        self.seen.created.fetch_add(1, Ordering::Relaxed);
        if let Some(state) = request.state {
            self.seen.states.lock().unwrap().push(state.to_vec());
        }
        Ok(LivePlugin {
            instance: Box::new(FakeInstance {
                seen: Arc::clone(&self.seen),
                gain: 1.0,
            }),
            processor: Box::new(FakeProcessor {
                instrument,
                gain: 1.0,
                held: 0,
                remaining: None,
            }),
        })
    }
}

fn fake_host() -> (PluginHost, Arc<Seen>) {
    let seen = Arc::new(Seen::default());
    let host: PluginHost = Arc::new(FakeHost {
        seen: Arc::clone(&seen),
    });
    (host, seen)
}

fn session(host: &PluginHost) -> Session {
    let mut registry = Registry::with_builtin();
    register_with(&mut registry, Some(Arc::clone(host))).unwrap();
    Session::new(Project::default(), registry)
}

fn run(s: &mut Session, lines: &str) {
    for line in lines.lines().map(str::trim).filter(|l| !l.is_empty()) {
        s.run_line(line).unwrap_or_else(|e| panic!("`{line}`: {e}"));
    }
}

fn peak(data: &[f32]) -> f32 {
    data.iter().fold(0.0, |p, s| p.max(s.abs()))
}

fn options() -> RenderOptions {
    RenderOptions {
        sample_rate: 8_000,
        loops: 1,
        max_tail_seconds: 0.0,
    }
}

#[test]
fn plugin_instruments_and_effects_render_with_their_parameters() {
    let (host, seen) = fake_host();
    let mut s = session(&host);
    run(
        &mut s,
        "channel.add_plugin test.tone Tone
         note.add 1 0 4 C4
         channel.set_volume 1 1",
    );
    let (left, _, warnings) = render_with(s.project(), options(), Some(&host));
    assert!(warnings.is_empty(), "{warnings:?}");
    // The tone's 0.25, -3 dB from the centred channel (circular pan law).
    let tone = 0.25 * std::f32::consts::FRAC_1_SQRT_2;
    let level = peak(&left);
    assert!((level - tone).abs() < 1e-3, "tone level {level}");
    // The note lasts four steps of sixteen; the rest is silent.
    assert!(peak(&left[left.len() / 2..]) < 1e-6);

    run(
        &mut s,
        "mixer.add_plugin 0 test.gain
         mixer.set_plugin_param 0 1 7 0.5
         channel.set_plugin_param 1 7 2",
    );
    let (left, _, _) = render_with(s.project(), options(), Some(&host));
    assert!((peak(&left) - tone).abs() < 1e-3, "tone x 2 x 0.5");
    assert!(seen.created.load(Ordering::Relaxed) >= 3);

    // Without a host, plugins are silent and reported.
    let (left, _, warnings) = render_with(s.project(), options(), None);
    assert!(peak(&left) < 1e-6);
    assert!(warnings.iter().any(|w| w.contains("not available")));

    // The export command renders with the registry's host.
    let dir = std::env::temp_dir().join(format!("mc-plugin-export-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.wav");
    run(
        &mut s,
        &format!("project.export_wav \"{}\"", path.display()),
    );
    assert!(std::fs::metadata(&path).unwrap().len() > 1_000);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn missing_plugins_warn_once_and_pass_audio_through() {
    let (host, _) = fake_host();
    let mut s = session(&host);
    run(
        &mut s,
        "channel.add_plugin test.tone
         note.add 1 0 4 C4
         mixer.add_plugin 0 com.nowhere.fx Gone",
    );
    let (left, _, warnings) = render_with(s.project(), options(), Some(&host));
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("Gone"));
    assert!(peak(&left) > 0.1, "a missing effect passes audio through");
}

#[test]
fn bridge_keeps_instances_across_edits_and_drops_removed_ones() {
    let (host, seen) = fake_host();
    let mut s = session(&host);
    let (_engine, mut handle) = engine_pair(8_000);
    let mut bridge = Bridge::with_plugins(8_000, Some(Arc::clone(&host)));
    run(&mut s, "channel.add_plugin test.tone");
    bridge.sync(s.project(), &mut handle);
    let instance = s.project().channels[0]
        .instrument
        .plugin()
        .unwrap()
        .instance;
    assert!(bridge.plugin_running(instance));
    // Structural edits elsewhere keep the same instance.
    run(
        &mut s,
        "channel.add_synth
         channel.move 2 1
         mixer.add_effect 0 reverb",
    );
    let report = bridge.sync(s.project(), &mut handle);
    assert!(report.rebuilt_graph);
    assert_eq!(seen.created.load(Ordering::Relaxed), 1);
    assert!(bridge.plugin_running(instance));
    run(&mut s, "channel.remove 2");
    bridge.sync(s.project(), &mut handle);
    assert!(!bridge.plugin_running(instance));
    // Undo brings it back as a fresh instance restored from the project.
    run(&mut s, "edit.undo");
    bridge.sync(s.project(), &mut handle);
    assert!(bridge.plugin_running(instance));
    assert_eq!(seen.created.load(Ordering::Relaxed), 2);
}

#[test]
fn stored_state_is_restored_and_plugin_changes_do_not_echo() {
    let (host, seen) = fake_host();
    let mut s = session(&host);
    let (_engine, mut handle) = engine_pair(8_000);
    let mut bridge = Bridge::with_plugins(8_000, Some(Arc::clone(&host)));
    run(
        &mut s,
        "channel.add_plugin test.tone
         channel.set_plugin_state 1 c3RhdGU=",
    );
    bridge.sync(s.project(), &mut handle);
    assert_eq!(*seen.states.lock().unwrap(), [b"state".to_vec()]);

    // The plugin moves its own gain; recording that sends nothing back.
    seen.events.lock().unwrap().push(PluginEvent::Param {
        id: GAIN,
        value: 0.3,
    });
    let events = bridge.poll_plugins();
    assert_eq!(events.len(), 1);
    run(&mut s, "channel.set_plugin_param 1 7 0.3");
    assert_eq!(bridge.sync(s.project(), &mut handle).messages, 0);

    // Undoing the first stored value returns the plugin to its baseline.
    run(&mut s, "edit.undo");
    assert_eq!(bridge.sync(s.project(), &mut handle).messages, 1);
    assert_eq!(bridge.sync(s.project(), &mut handle).messages, 0);
}

#[test]
fn plugin_list_command_filters_by_kind() {
    let (host, _) = fake_host();
    let mut s = session(&host);
    let all = s.run_line("plugin.list").unwrap().message;
    assert!(all.contains("test.tone") && all.contains("test.gain"));
    let fx = s.run_line("plugin.list effect").unwrap().message;
    assert!(!fx.contains("test.tone") && fx.contains("test.gain"));
    let mut plain = Session::new(Project::default(), crate::full_registry());
    let none = plain.run_line("plugin.list").unwrap().message;
    assert!(none.contains("not available"));
}
