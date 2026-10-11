//! Channel settings: the instrument editor for the selected channel.

use crate::hint::Hint;
use egui::{Color32, Pos2, RichText, Sense, Shape, Stroke, pos2, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::music::key_name;
use musiccraft_project::params::{INSTRUMENT_PARAMS, ParamSpec, WAVEFORMS, inst};
use musiccraft_project::{Channel, DEFAULT_CHANNEL_VOLUME, Instrument, MAX_VOLUME};

use crate::dialogs::Dialog;
use crate::{Action, Frame, UiState, theme, widgets};

pub(crate) fn to_unit(spec: &ParamSpec, value: f32) -> f32 {
    spec.to_unit(value)
}

pub(crate) fn from_unit(spec: &ParamSpec, t: f32) -> f32 {
    spec.from_unit(t)
}

pub(crate) fn format_value(spec: &ParamSpec, value: f32) -> String {
    spec.format(value)
}

fn spec(index: usize) -> ParamSpec {
    INSTRUMENT_PARAMS[index]
}

/// A parameter knob bound to `channel.set_param`.
fn param_knob(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    index: usize,
    label: &str,
    actions: &mut Vec<Action>,
) {
    let spec = spec(index);
    let value = channel.params.get(index);
    let out = widgets::labeled_knob(
        ui,
        label,
        to_unit(&spec, value),
        to_unit(&spec, spec.default),
        spec.min < 0.0 && spec.max > 0.0,
        &format_value(&spec, value),
    );
    crate::automation::menu(
        &out.response,
        &format!("channel:{number}:{}", spec.name),
        actions,
    );
    if out.started {
        state.begin_gesture();
    }
    if let Some(t) = out.changed {
        let new = from_unit(&spec, t);
        if new != value {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_param",
                    [number.to_string(), spec.name.to_owned(), format!("{new}")],
                ),
                state.gesture(),
            ));
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::PANEL_DARK)
        .corner_radius(6.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).small().strong().color(theme::TEXT_DIM));
            add(ui);
        });
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.channel_settings;
    let index = state.selected_channel;
    let Some(channel) = frame.project.channels.get(index) else {
        return;
    };
    let area = state.workspace;
    egui::Window::new(format!("Channel — {}", channel.name))
        .id(egui::Id::new("channel-settings"))
        .open(&mut open)
        .default_pos(area.right_top() + vec2(-470.0, 30.0))
        .default_width(440.0)
        .resizable(false)
        .constrain_to(area)
        .vscroll(true)
        .min_height((area.height() - 60.0).clamp(300.0, 760.0))
        .max_height((area.height() - 40.0).max(300.0))
        .show(ctx, |ui| {
            contents(ui, state, frame, index, channel, actions)
        });
    state.windows.channel_settings = open;
}

fn contents(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let number = index + 1;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&channel.name)
                .size(18.0)
                .strong()
                .color(theme::palette(index)),
        );
        ui.label(RichText::new(channel.instrument.label()).color(theme::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let track = channel.mixer_track;
            let name = frame
                .project
                .mixer
                .get(track)
                .map_or("", |t| t.name.as_str());
            ui.label(
                RichText::new(format!("Mixer {track} · {name}"))
                    .small()
                    .color(theme::TEXT_DIM),
            );
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let volume = widgets::labeled_knob(
            ui,
            "Volume",
            channel.volume / MAX_VOLUME,
            DEFAULT_CHANNEL_VOLUME / MAX_VOLUME,
            false,
            &format!("{:.0}%", channel.volume * 100.0),
        );
        crate::automation::menu(
            &volume.response,
            &format!("channel:{number}:volume"),
            actions,
        );
        if volume.started {
            state.begin_gesture();
        }
        if let Some(t) = volume.changed {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_volume",
                    [number.to_string(), format!("{:.3}", t * MAX_VOLUME)],
                ),
                state.gesture(),
            ));
        }
        let pan = widgets::labeled_knob(
            ui,
            "Pan",
            (channel.pan + 1.0) / 2.0,
            0.5,
            true,
            &format!("{:+.0}", channel.pan * 100.0),
        );
        crate::automation::menu(&pan.response, &format!("channel:{number}:pan"), actions);
        if pan.started {
            state.begin_gesture();
        }
        if let Some(t) = pan.changed {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_pan",
                    [number.to_string(), format!("{:.3}", t * 2.0 - 1.0)],
                ),
                state.gesture(),
            ));
        }
        if channel.instrument.plugin().is_none() {
            param_knob(ui, state, number, channel, inst::PITCH, "Pitch", actions);
        }
        let mut mute = channel.mute;
        if ui.checkbox(&mut mute, "Mute").changed() {
            actions.push(Action::Run(Invocation::new(
                "channel.toggle_mute",
                [number.to_string()],
            )));
        }
    });
    ui.add_space(6.0);
    section(ui, "ARPEGGIATOR", |ui| {
        ui.horizontal(|ui| {
            choice(
                ui,
                number,
                channel,
                inst::ARP_MODE,
                &["Off", "Up", "Down", "Up-down", "Random"],
                actions,
            );
        });
        if channel.params.get(inst::ARP_MODE) >= 0.5 {
            ui.horizontal(|ui| {
                let labels: Vec<&str> = musiccraft_project::params::ARP_RATES
                    .iter()
                    .map(|(l, _)| *l)
                    .collect();
                choice(ui, number, channel, inst::ARP_RATE, &labels, actions);
            });
            ui.horizontal(|ui| {
                param_knob(
                    ui,
                    state,
                    number,
                    channel,
                    inst::ARP_RANGE,
                    "Octaves",
                    actions,
                );
                param_knob(ui, state, number, channel, inst::ARP_GATE, "Gate", actions);
            });
        }
    });
    ui.add_space(6.0);
    match &channel.instrument {
        Instrument::Sampler { sample } => {
            sampler(ui, state, frame, index, channel, sample, actions)
        }
        Instrument::Synth => synth(ui, state, number, channel, actions),
        Instrument::DrumSynth => {
            drum(ui, state, number, channel, actions);
            return;
        }
        Instrument::Pluck => {
            pluck(ui, state, number, channel, actions);
            return;
        }
        Instrument::Fm => {
            fm(ui, state, number, channel, actions);
            return;
        }
        Instrument::Keys => {
            keys(ui, state, number, channel, actions);
            return;
        }
        Instrument::Additive => additive(ui, state, number, channel, actions),
        Instrument::Granular { sample } => {
            granular(ui, state, frame, index, channel, sample, actions);
        }
        Instrument::Kit { pads } => {
            kit(ui, state, index, channel, pads, actions);
            return;
        }
        Instrument::Layer { channels } => {
            layer(ui, frame, index, channels, actions);
            return;
        }
        Instrument::Plugin { plugin } => {
            // Plugins shape their own sound; the built-in sections do not apply.
            let target = crate::plugins::Target::Channel(number);
            crate::plugins::panel(ui, state, frame.plugins, plugin, target, actions);
            return;
        }
    }
    ui.add_space(6.0);
    envelope_section(ui, state, number, channel, actions);
}

/// The amplitude envelope knobs and their graph.
fn envelope_section(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    section(ui, "ENVELOPE", |ui| {
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::ATTACK, "Attack", actions);
            param_knob(ui, state, number, channel, inst::DECAY, "Decay", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::SUSTAIN,
                "Sustain",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::RELEASE,
                "Release",
                actions,
            );
            envelope_graph(ui, channel, theme::palette(number - 1));
        });
    });
}

fn sampler(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    index: usize,
    channel: &Channel,
    sample: &str,
    actions: &mut Vec<Action>,
) {
    let number = index + 1;
    section(ui, "SAMPLE", |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(sample).monospace().small()).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Change…").clicked() {
                    state.open_dialog(Dialog::set_sample(index, sample.to_owned()));
                }
            });
        });
        let slices = channel.params.get(inst::SLICES).round().max(1.0) as u32;
        waveform(ui, frame.waveform, theme::palette(index), slices);
        ui.horizontal(|ui| {
            let mut one_shot = channel.params.get(inst::ONE_SHOT) >= 0.5;
            if ui
                .checkbox(&mut one_shot, "One shot")
                .hint("Play the whole sample regardless of note length")
                .changed()
            {
                actions.push(Action::Run(Invocation::new(
                    "channel.set_param",
                    [
                        number.to_string(),
                        "one_shot".to_owned(),
                        if one_shot { "1" } else { "0" }.to_owned(),
                    ],
                )));
            }
            ui.add_space(12.0);
            let root = channel.params.get(inst::ROOT_KEY) as u8;
            ui.label("Root key");
            let mut key = u32::from(root);
            let response = ui.add(
                egui::DragValue::new(&mut key)
                    .range(0..=127)
                    .speed(0.2)
                    .custom_formatter(|v, _| key_name(v as u8)),
            );
            if response.changed() {
                actions.push(Action::Run(Invocation::new(
                    "channel.set_param",
                    [number.to_string(), "root_key".to_owned(), key.to_string()],
                )));
            }
        });
        ui.horizontal(|ui| {
            ui.label("Slices");
            let mut count = slices;
            let drag = ui
                .add(egui::DragValue::new(&mut count).range(1..=64).speed(0.1))
                .hint("Cut the sample into equal slices; key root + n plays slice n");
            if drag.changed() && count != slices {
                actions.push(Action::Run(Invocation::new(
                    "channel.set_param",
                    [number.to_string(), "slices".to_owned(), count.to_string()],
                )));
            }
            let target = if slices > 1 { slices } else { 8 };
            if ui
                .button(format!("Slice to pattern ({target})"))
                .hint("Replace this channel's notes with one note per slice, in order")
                .clicked()
            {
                actions.push(Action::Run(Invocation::new(
                    "channel.slice",
                    [number.to_string(), target.to_string()],
                )));
            }
        });
    });
}

/// A row of choice buttons bound to an integer instrument parameter.
fn choice(
    ui: &mut egui::Ui,
    number: usize,
    channel: &Channel,
    param: usize,
    labels: &[&str],
    actions: &mut Vec<Action>,
) {
    let current = channel.params.get(param).round() as usize;
    for (i, label) in labels.iter().enumerate() {
        if ui.selectable_label(i == current, *label).clicked() && i != current {
            actions.push(Action::Run(Invocation::new(
                "channel.set_param",
                [
                    number.to_string(),
                    INSTRUMENT_PARAMS[param].name.to_owned(),
                    i.to_string(),
                ],
            )));
        }
    }
}

/// Preset picker for a built-in instrument: applying one is a single undo
/// step.
fn preset_bar(
    ui: &mut egui::Ui,
    number: usize,
    channel: &Channel,
    list: &'static [musiccraft_project::params::presets::Preset],
    actions: &mut Vec<Action>,
) {
    let current = list
        .iter()
        .position(|p| p.matches(INSTRUMENT_PARAMS, &channel.params.0));
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("PRESET")
                .small()
                .strong()
                .color(theme::TEXT_DIM),
        );
        let n = list.len();
        let salt = (number, list.as_ptr() as usize);
        if ui.small_button("◀").hint("Previous preset").clicked() {
            chosen = Some(current.map_or(n - 1, |c| (c + n - 1) % n));
        }
        egui::ComboBox::from_id_salt(("instrument-preset", salt))
            .width(180.0)
            .selected_text(current.map_or("(custom)", |i| list[i].name))
            .show_ui(ui, |ui| {
                for (i, preset) in list.iter().enumerate() {
                    if ui
                        .selectable_label(current == Some(i), preset.name)
                        .clicked()
                    {
                        chosen = Some(i);
                    }
                }
            });
        if ui.small_button("▶").hint("Next preset").clicked() {
            chosen = Some(current.map_or(0, |c| (c + 1) % n));
        }
    });
    let Some(preset) = chosen.and_then(|i| list.get(i)) else {
        return;
    };
    let changes: Vec<String> = preset
        .resolve(INSTRUMENT_PARAMS)
        .into_iter()
        .filter(|(name, value)| {
            musiccraft_project::params::index_of(INSTRUMENT_PARAMS, name)
                .is_some_and(|i| (channel.params.get(i) - value).abs() > 1e-6)
        })
        .map(|(name, value)| format!("channel.set_param {number} {name} {value}"))
        .collect();
    if !changes.is_empty() {
        actions.push(Action::Run(Invocation::new(
            musiccraft_commands::BATCH,
            changes,
        )));
    }
}

fn drum(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::DRUM,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "DRUM", |ui| {
        ui.horizontal(|ui| {
            choice(
                ui,
                number,
                channel,
                inst::DRUM_TYPE,
                &["Kick", "Snare", "Hat", "Tom", "Clap"],
                actions,
            );
        });
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::DRUM_TUNE, "Tune", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::DRUM_DECAY,
                "Decay",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::DRUM_SWEEP,
                "Sweep",
                actions,
            );
            param_knob(ui, state, number, channel, inst::DRUM_TONE, "Tone", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::DRUM_CLICK,
                "Click",
                actions,
            );
        });
    });
}

fn fm(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::FM,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "ALGORITHM", |ui| {
        ui.horizontal(|ui| {
            choice(
                ui,
                number,
                channel,
                inst::FM_ALGO,
                &[
                    "4→3→2→1",
                    "4→3, 3+2→1",
                    "4→3 2→1",
                    "4→1,2,3",
                    "2→1 +3 +4",
                    "1+2+3+4",
                ],
                actions,
            );
        });
        ui.horizontal(|ui| {
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::FM_FEEDBACK,
                "Feedback 4",
                actions,
            );
        });
    });
    for op in 0..4 {
        ui.add_space(4.0);
        section(ui, &format!("OPERATOR {}", op + 1), |ui| {
            ui.horizontal(|ui| {
                let base = inst::OP1_RATIO + op * 4;
                param_knob(ui, state, number, channel, base, "Ratio", actions);
                param_knob(ui, state, number, channel, base + 1, "Level", actions);
                param_knob(ui, state, number, channel, base + 2, "Decay", actions);
                param_knob(ui, state, number, channel, base + 3, "Sustain", actions);
            });
        });
    }
    ui.add_space(4.0);
    envelope_section(ui, state, number, channel, actions);
}

fn keys(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::KEYS,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "KEYS", |ui| {
        ui.horizontal(|ui| {
            choice(
                ui,
                number,
                channel,
                inst::KEYS_TYPE,
                &["Piano", "Electric piano", "Organ"],
                actions,
            );
        });
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::KEYS_TONE, "Tone", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::KEYS_DECAY,
                "Ring",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::KEYS_TREMOLO,
                "Tremolo",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::KEYS_HAMMER,
                "Hammer",
                actions,
            );
        });
    });
    ui.add_space(4.0);
    envelope_section(ui, state, number, channel, actions);
}

fn additive(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::ADDITIVE,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "HARMONICS", |ui| {
        ui.horizontal(|ui| {
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::ADD_PARTIALS,
                "Harmonics",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::ADD_BRIGHT,
                "Bright",
                actions,
            );
            param_knob(ui, state, number, channel, inst::ADD_EVEN, "Even", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::ADD_INHARM,
                "Stretch",
                actions,
            );
            param_knob(ui, state, number, channel, inst::ADD_DECAY, "Fade", actions);
        });
        harmonics_graph(ui, channel);
    });
}

/// Bars for each harmonic's level.
fn harmonics_graph(ui: &mut egui::Ui, channel: &Channel) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 50.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let count = channel.params.get(inst::ADD_PARTIALS).round().max(1.0) as usize;
    let slope = 2.0 - 1.6 * channel.params.get(inst::ADD_BRIGHT);
    let even = channel.params.get(inst::ADD_EVEN);
    let w = rect.width() / 32.0;
    for n in 1..=count {
        let level = (n as f32).powf(-slope) * if n % 2 == 0 { even } else { 1.0 };
        let x = rect.left() + (n - 1) as f32 * w;
        let top = rect.bottom() - level.min(1.0) * (rect.height() - 4.0);
        painter.rect_filled(
            egui::Rect::from_min_max(pos2(x + 1.0, top), pos2(x + w - 1.0, rect.bottom())),
            1.0,
            theme::ACCENT,
        );
    }
}

fn granular(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    index: usize,
    channel: &Channel,
    sample: &str,
    actions: &mut Vec<Action>,
) {
    let number = index + 1;
    section(ui, "GRAINS", |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(sample).monospace().small()).truncate());
            if ui.small_button("Change…").clicked() {
                state.open_dialog(Dialog::set_sample(index, sample.to_owned()));
            }
        });
        waveform(ui, frame.waveform, theme::palette(index), 1);
        ui.horizontal(|ui| {
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::GRAN_POSITION,
                "Position",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::GRAN_SPREAD,
                "Spread",
                actions,
            );
            param_knob(ui, state, number, channel, inst::GRAN_SIZE, "Size", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::GRAN_DENSITY,
                "Density",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::GRAN_PITCH_RAND,
                "Detune",
                actions,
            );
            param_knob(ui, state, number, channel, inst::GRAN_SCAN, "Scan", actions);
        });
    });
}

fn kit(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    channel: &Channel,
    pads: &[String],
    actions: &mut Vec<Action>,
) {
    let root = channel.params.get(inst::ROOT_KEY).round() as u8;
    section(ui, "PADS", |ui| {
        ui.label(
            RichText::new(
                "Click a pad to hear it; right-click to load a sample. Pad 1 is the root key.",
            )
            .small()
            .color(theme::TEXT_DIM),
        );
        egui::Grid::new(("kit-pads", index))
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                // Pads 13–16 on top, like a pad controller.
                for row in (0..4).rev() {
                    for col in 0..4 {
                        let pad = row * 4 + col;
                        let name = pads
                            .get(pad)
                            .filter(|p| !p.is_empty())
                            .map(|p| {
                                p.strip_prefix(musiccraft_project::params::BUILTIN_PREFIX)
                                    .map(str::to_owned)
                                    .unwrap_or_else(|| {
                                        std::path::Path::new(p).file_stem().map_or_else(
                                            || p.clone(),
                                            |s| s.to_string_lossy().into_owned(),
                                        )
                                    })
                            })
                            .unwrap_or_else(|| "—".to_owned());
                        let button = egui::Button::new(
                            RichText::new(format!("{}\n{name}", pad + 1)).small(),
                        )
                        .min_size(vec2(90.0, 40.0))
                        .fill(if name == "—" {
                            theme::PANEL_DARK
                        } else {
                            theme::RAISED
                        });
                        let response = ui.add(button).hint(format!(
                            "Pad {} — key {}",
                            pad + 1,
                            key_name(root.saturating_add(pad as u8))
                        ));
                        if response.clicked() {
                            actions.push(Action::PreviewShort {
                                channel: index,
                                key: root.saturating_add(pad as u8),
                            });
                        }
                        if response.secondary_clicked() {
                            state.open_dialog(Dialog::set_pad(
                                index,
                                pad,
                                pads.get(pad).cloned().unwrap_or_default(),
                            ));
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn layer(
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
    index: usize,
    children: &[musiccraft_project::ChannelId],
    actions: &mut Vec<Action>,
) {
    section(ui, "LAYER", |ui| {
        ui.label(
            RichText::new("Notes on this channel play every ticked channel.")
                .small()
                .color(theme::TEXT_DIM),
        );
        for (i, other) in frame.project.channels.iter().enumerate() {
            if i == index || matches!(other.instrument, Instrument::Layer { .. }) {
                continue;
            }
            let mut on = children.contains(&other.id);
            if ui.checkbox(&mut on, &other.name).changed() {
                let mut next: Vec<usize> = frame
                    .project
                    .channels
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| children.contains(&c.id))
                    .map(|(j, _)| j)
                    .collect();
                if on {
                    next.push(i);
                } else {
                    next.retain(|j| *j != i);
                }
                next.sort_unstable();
                let list = if next.is_empty() {
                    "-".to_owned()
                } else {
                    next.iter()
                        .map(|j| (j + 1).to_string())
                        .collect::<Vec<_>>()
                        .join(",")
                };
                actions.push(Action::Run(Invocation::new(
                    "channel.set_layer",
                    [(index + 1).to_string(), list],
                )));
            }
        }
    });
}

fn pluck(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::PLUCK,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "STRING", |ui| {
        ui.horizontal(|ui| {
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::PLUCK_DECAY,
                "Decay",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::PLUCK_DAMPING,
                "Damping",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::PLUCK_TONE,
                "Tone",
                actions,
            );
        });
    });
}

/// The synth filter's response at the current settings.
fn filter_graph(ui: &mut egui::Ui, channel: &Channel, color: Color32) {
    use musiccraft_dsp::{SvfMode, response};
    let (rect, _) = ui.allocate_exact_size(vec2(150.0, 54.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let mode = SvfMode::from_param(channel.params.get(inst::FILTER_TYPE));
    let cutoff = channel.params.get(inst::CUTOFF);
    let resonance = channel.params.get(inst::RESONANCE);
    let width = rect.width() as usize;
    let points: Vec<Pos2> = (0..=width)
        .map(|i| {
            let t = i as f32 / width as f32;
            let f = 20.0 * 1_000.0_f32.powf(t);
            let db = response::svf_db(mode, cutoff, resonance, f, 48_000.0).clamp(-40.0, 20.0);
            pos2(
                rect.left() + i as f32,
                rect.top() + rect.height() * (20.0 - db) / 60.0,
            )
        })
        .collect();
    painter.add(Shape::line(points, Stroke::new(1.5, color)));
}

fn synth(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    preset_bar(
        ui,
        number,
        channel,
        musiccraft_project::params::presets::SYNTH,
        actions,
    );
    ui.add_space(4.0);
    section(ui, "OSCILLATORS", |ui| {
        for (label, param) in [
            ("Osc 1", inst::WAVE1),
            ("Osc 2", inst::WAVE2),
            ("Osc 3", inst::WAVE3),
        ] {
            ui.horizontal(|ui| {
                ui.add_sized(
                    vec2(44.0, 20.0),
                    egui::Label::new(RichText::new(label).small()),
                );
                let current = channel.params.get(param).round() as usize;
                for (wave, name) in WAVEFORMS.iter().enumerate() {
                    if wave_button(ui, wave, wave == current).hint(*name).clicked()
                        && wave != current
                    {
                        actions.push(Action::Run(Invocation::new(
                            "channel.set_param",
                            [
                                number.to_string(),
                                INSTRUMENT_PARAMS[param].name.to_owned(),
                                wave.to_string(),
                            ],
                        )));
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::DETUNE, "Detune", actions);
            param_knob(ui, state, number, channel, inst::OSC_MIX, "Mix", actions);
            param_knob(ui, state, number, channel, inst::SUB, "Sub", actions);
            param_knob(ui, state, number, channel, inst::GLIDE, "Glide", actions);
            param_knob(ui, state, number, channel, inst::UNISON, "Unison", actions);
            param_knob(ui, state, number, channel, inst::SPREAD, "Spread", actions);
            param_knob(ui, state, number, channel, inst::DRIVE, "Drive", actions);
        });
        ui.horizontal(|ui| {
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::OSC3_LEVEL,
                "Osc 3",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::OSC3_COARSE,
                "Osc 3 pitch",
                actions,
            );
        });
    });
    ui.add_space(6.0);
    section(ui, "FILTER", |ui| {
        ui.horizontal(|ui| {
            choice(
                ui,
                number,
                channel,
                inst::FILTER_TYPE,
                &["Low-pass", "Band-pass", "High-pass"],
                actions,
            );
        });
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::CUTOFF, "Cutoff", actions);
            param_knob(ui, state, number, channel, inst::RESONANCE, "Reso", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::ENV_AMOUNT,
                "Env amt",
                actions,
            );
            filter_graph(ui, channel, theme::ACCENT);
        });
    });
    ui.add_space(6.0);
    section(ui, "LFO", |ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("To").small());
            choice(
                ui,
                number,
                channel,
                inst::LFO_TARGET,
                &["Pitch", "Cutoff", "Volume"],
                actions,
            );
        });
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::LFO_RATE, "Rate", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::LFO_DEPTH,
                "Depth",
                actions,
            );
        });
    });
}

/// A button showing one oscillator wave shape.
fn wave_button(ui: &mut egui::Ui, wave: usize, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, 22.0), Sense::click());
    let fill = if selected {
        theme::ACCENT_DIM
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    let r = rect.shrink2(vec2(6.0, 5.0));
    let at = |x: f32, y: f32| {
        pos2(
            r.left() + r.width() * x,
            r.center().y - r.height() / 2.0 * y,
        )
    };
    let points: Vec<Pos2> = match wave {
        0 => (0..=24)
            .map(|i| {
                let x = i as f32 / 24.0;
                at(x, (x * std::f32::consts::TAU).sin())
            })
            .collect(),
        1 => vec![at(0.0, -1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, 1.0)],
        2 => vec![at(0.0, 1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, -1.0)],
        3 => vec![at(0.0, 0.0), at(0.25, 1.0), at(0.75, -1.0), at(1.0, 0.0)],
        _ => (0..=16)
            .map(|i| {
                at(
                    i as f32 / 16.0,
                    [
                        0.2, -0.7, 0.9, -0.3, 0.6, -1.0, 0.1, 0.8, -0.5, 0.3, -0.9, 0.7, -0.2, 0.5,
                        -0.6, 0.9, -0.1,
                    ][i],
                )
            })
            .collect(),
    };
    let color = if selected {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    ui.painter()
        .add(Shape::line(points, Stroke::new(1.5, color)));
    response
}

/// Min/max overview of the sample.
fn waveform(ui: &mut egui::Ui, overview: Option<&[(f32, f32)]>, color: Color32, slices: u32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    painter.line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, theme::LCD_EDGE),
    );
    let Some(points) = overview.filter(|p| !p.is_empty()) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "no audio",
            egui::FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
        return;
    };
    let peak = points
        .iter()
        .fold(0.0_f32, |p, (lo, hi)| p.max(lo.abs()).max(hi.abs()))
        .max(1e-6);
    let step = rect.width() / points.len() as f32;
    for (i, (lo, hi)) in points.iter().enumerate() {
        let x = rect.left() + (i as f32 + 0.5) * step;
        let half = rect.height() * 0.45 / peak;
        painter.line_segment(
            [
                pos2(x, rect.center().y - hi * half),
                pos2(x, rect.center().y - lo * half - 0.5),
            ],
            Stroke::new(step.max(1.0), color),
        );
    }
    // Slice boundaries, numbered.
    if slices > 1 {
        for i in 0..slices {
            let x = rect.left() + rect.width() * i as f32 / slices as f32;
            if i > 0 {
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, theme::HIGHLIGHT),
                );
            }
            if slices <= 16 {
                painter.text(
                    pos2(x + 3.0, rect.top() + 2.0),
                    egui::Align2::LEFT_TOP,
                    (i + 1).to_string(),
                    egui::FontId::proportional(9.0),
                    theme::HIGHLIGHT,
                );
            }
        }
    }
}

/// The envelope shape: attack, decay, a fixed sustain segment, release.
fn envelope_graph(ui: &mut egui::Ui, channel: &Channel, color: Color32) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width().max(100.0), 56.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let a = channel.params.get(inst::ATTACK);
    let d = channel.params.get(inst::DECAY);
    let s = channel.params.get(inst::SUSTAIN).clamp(0.0, 1.0);
    let r = channel.params.get(inst::RELEASE);
    // Time axis compressed with a square root so short stages stay visible.
    let w = |t: f32| t.max(0.0).sqrt();
    let hold = 0.6;
    let total = w(a) + w(d) + hold + w(r);
    let inner = rect.shrink(6.0);
    let x = |t: f32| inner.left() + t / total.max(1e-6) * inner.width();
    let y = |level: f32| inner.bottom() - level * inner.height();
    let points = vec![
        pos2(x(0.0), y(0.0)),
        pos2(x(w(a)), y(1.0)),
        pos2(x(w(a) + w(d)), y(s)),
        pos2(x(w(a) + w(d) + hold), y(s)),
        pos2(x(total), y(0.0)),
    ];
    painter.add(Shape::line(points, Stroke::new(2.0, color)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_mappings_round_trip() {
        for (index, value) in [
            (inst::CUTOFF, 1_000.0),
            (inst::ATTACK, 0.05),
            (inst::SUSTAIN, 0.4),
            (inst::DETUNE, -300.0),
        ] {
            let spec = spec(index);
            let back = from_unit(&spec, to_unit(&spec, value));
            assert!(
                (back - value).abs() / value.abs().max(1.0) < 1e-3,
                "{} {value} -> {back}",
                spec.name
            );
        }
        let cutoff = spec(inst::CUTOFF);
        assert!(
            (to_unit(&cutoff, 632.5) - 0.5).abs() < 0.01,
            "log scale centre"
        );
        assert_eq!(format_value(&cutoff, 2_500.0), "2.50 kHz");
        assert_eq!(format_value(&spec(inst::ATTACK), 0.25), "250 ms");
        assert_eq!(format_value(&spec(inst::SUSTAIN), 0.5), "50%");
    }
}
