//! Glue between running plugins and the session: builds what the UI shows,
//! records changes a plugin makes in its own window as commands, and stores
//! plugin state in the project before it is saved or exported.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use musiccraft_commands::{Invocation, Session};
use musiccraft_project::{PluginSlot, Project, base64};
use musiccraft_render::{Bridge, PluginEvent, PluginHost, PluginKind};
use musiccraft_ui::{LivePluginView, PluginEntry, PluginParamView, Plugins, UiState};

/// How long after a plugin reports a state change its state is stored.
const STATE_DELAY: Duration = Duration::from_secs(2);
/// Marks coalescing keys of parameter changes made in a plugin's own window,
/// so they never merge with a UI gesture.
const PLUGIN_GESTURE: u64 = 1 << 63;

/// Commands that write the project to disk or render it.
const PERSISTING: [&str; 4] = [
    "project.save",
    "project.export_wav",
    "project.export_flac",
    "project.export_stems",
];

pub struct PluginGlue {
    host: Option<PluginHost>,
    catalog: Vec<PluginEntry>,
    /// Instances whose state changed, and when the first change was seen.
    dirty: BTreeSet<u32>,
    dirty_since: Option<Instant>,
}

/// The arguments that name `slot` in plugin commands.
fn slot_args(slot: PluginSlot) -> Vec<String> {
    match slot {
        PluginSlot::Channel(c) => vec![(c + 1).to_string()],
        PluginSlot::Effect(t, s) => vec![t.to_string(), (s + 1).to_string()],
    }
}

fn command(slot: PluginSlot, what: &str) -> String {
    match slot {
        PluginSlot::Channel(_) => format!("channel.{what}"),
        PluginSlot::Effect(..) => format!("mixer.{what}"),
    }
}

impl PluginGlue {
    pub fn new(host: Option<PluginHost>) -> Self {
        let mut glue = Self {
            host,
            catalog: Vec::new(),
            dirty: BTreeSet::new(),
            dirty_since: None,
        };
        glue.refresh_catalog();
        glue
    }

    fn refresh_catalog(&mut self) {
        self.catalog = self
            .host
            .as_ref()
            .map(|h| h.catalog())
            .unwrap_or_default()
            .into_iter()
            .map(|p| PluginEntry {
                instrument: p.kind == PluginKind::Instrument,
                id: p.id,
                name: p.name,
                vendor: p.vendor,
            })
            .collect();
        self.catalog.sort_by_key(|p| p.name.to_lowercase());
    }

    /// Scans the plugin folders again; returns a status message.
    pub fn rescan(&mut self, bridge: Option<&mut Bridge>) -> String {
        let Some(host) = &self.host else {
            return "plugin hosting is not available in this build".to_owned();
        };
        host.rescan();
        self.refresh_catalog();
        if let Some(bridge) = bridge {
            bridge.retry_plugins();
        }
        format!("found {} plugins", self.catalog.len())
    }

    /// What the UI shows this frame: the catalog and the plugins of the
    /// selected channel and the selected mixer slot.
    pub fn view(&self, bridge: Option<&mut Bridge>, project: &Project, ui: &UiState) -> Plugins {
        let mut plugins = Plugins {
            available: self.host.is_some(),
            catalog: self.catalog.clone(),
            ..Plugins::default()
        };
        let Some(bridge) = bridge else {
            return plugins;
        };
        plugins.running = project
            .plugins()
            .iter()
            .map(|(_, p)| p.instance)
            .filter(|i| bridge.plugin_running(*i))
            .collect();
        let mut shown = Vec::new();
        if ui.windows.channel_settings {
            shown.push(PluginSlot::Channel(ui.selected_channel));
        }
        if ui.windows.mixer {
            shown.push(PluginSlot::Effect(ui.selected_track, ui.mixer_slot));
        }
        for slot in shown {
            let Some(instance) = project.plugin(slot).map(|p| p.instance) else {
                continue;
            };
            let Some(live) = bridge.plugin(instance) else {
                continue;
            };
            let params = live
                .params()
                .into_iter()
                .filter(|p| !p.hidden)
                .map(|p| {
                    let value = live.param_value(p.id).unwrap_or(p.default);
                    PluginParamView {
                        text: live
                            .format_param(p.id, value)
                            .unwrap_or_else(|| format!("{value:.3}")),
                        id: p.id,
                        name: p.name,
                        module: p.module,
                        min: p.min,
                        max: p.max,
                        default: p.default,
                        stepped: p.stepped,
                        read_only: p.read_only,
                        value,
                    }
                })
                .collect();
            plugins.live.push(LivePluginView {
                instance,
                params,
                has_editor: live.has_editor(),
                editor_open: live.editor_open(),
            });
        }
        plugins
    }

    /// Opens or closes a plugin's own window.
    pub fn set_editor(
        &self,
        bridge: Option<&mut Bridge>,
        project: &Project,
        instance: u32,
        open: bool,
    ) -> Result<(), String> {
        let title = project
            .plugin_slot(instance)
            .and_then(|slot| project.plugin(slot))
            .map_or_else(|| "Plugin".to_owned(), |p| p.name.clone());
        let live = bridge
            .and_then(|b| b.plugin(instance))
            .ok_or("the plugin is not running")?;
        live.set_editor_open(open, &title)
    }

    /// Records what plugins reported since the last frame. Returns whether any
    /// plugin window is open (it then needs frequent polling).
    pub fn poll(&mut self, bridge: Option<&mut Bridge>, session: &mut Session) -> bool {
        let Some(bridge) = bridge else {
            return false;
        };
        for (instance, event) in bridge.poll_plugins() {
            let Some(slot) = session.project().plugin_slot(instance) else {
                continue;
            };
            match event {
                PluginEvent::Param { id, value } => {
                    let mut args = slot_args(slot);
                    args.push(id.to_string());
                    args.push(value.to_string());
                    let invocation = Invocation::new(command(slot, "set_plugin_param"), args);
                    let key = PLUGIN_GESTURE | (u64::from(instance) << 32) | u64::from(id);
                    let _ = session.run_coalesced(&invocation, key);
                }
                PluginEvent::StateChanged => {
                    self.dirty.insert(instance);
                    self.dirty_since.get_or_insert_with(Instant::now);
                }
                PluginEvent::EditorClosed | PluginEvent::ParamsChanged => {}
            }
        }
        if self
            .dirty_since
            .is_some_and(|since| since.elapsed() >= STATE_DELAY)
        {
            let dirty: Vec<u32> = std::mem::take(&mut self.dirty).into_iter().collect();
            self.dirty_since = None;
            store_states(bridge, session, &dirty);
        }
        session
            .project()
            .plugins()
            .iter()
            .any(|(_, p)| bridge.plugin(p.instance).is_some_and(|l| l.editor_open()))
    }

    /// Stores every running plugin's state in the project now.
    pub fn store_all(&mut self, bridge: Option<&mut Bridge>, session: &mut Session) {
        let Some(bridge) = bridge else {
            return;
        };
        let all: Vec<u32> = session
            .project()
            .plugins()
            .iter()
            .map(|(_, p)| p.instance)
            .collect();
        store_states(bridge, session, &all);
        self.dirty.clear();
        self.dirty_since = None;
    }

    /// Stores every running plugin's state when `invocation` is about to save
    /// or render the project.
    pub fn before(
        &mut self,
        invocation: &Invocation,
        bridge: Option<&mut Bridge>,
        session: &mut Session,
    ) {
        if !PERSISTING.contains(&invocation.name.as_str()) {
            return;
        }
        let Some(bridge) = bridge else {
            return;
        };
        let all: Vec<u32> = session
            .project()
            .plugins()
            .iter()
            .map(|(_, p)| p.instance)
            .collect();
        store_states(bridge, session, &all);
        self.dirty.clear();
        self.dirty_since = None;
    }
}

/// Copies the state of each running plugin in `instances` into the project.
fn store_states(bridge: &mut Bridge, session: &mut Session, instances: &[u32]) {
    for instance in instances {
        let Some(slot) = session.project().plugin_slot(*instance) else {
            continue;
        };
        let Some(state) = bridge.plugin(*instance).and_then(|p| p.save_state()) else {
            continue;
        };
        let encoded = base64::encode(&state);
        if session
            .project()
            .plugin(slot)
            .is_some_and(|p| p.state == encoded)
        {
            continue;
        }
        let mut args = slot_args(slot);
        args.push(encoded);
        let _ = session.run(&Invocation::new(command(slot, "set_plugin_state"), args));
    }
}
