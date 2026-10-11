//! The mixer: one strip per track plus the selected track's effect chain.

use crate::hint::Hint;
use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{EFFECT_KINDS, MAX_EFFECT_SLOTS};
use musiccraft_project::{MAX_VOLUME, MixerTrack, Project};

use crate::dialogs::Dialog;
use crate::{Action, Frame, UiState, theme, widgets};

const STRIP_WIDTH: f32 = 62.0;

fn run(actions: &mut Vec<Action>, name: &str, args: impl IntoIterator<Item = String>) {
    actions.push(Action::Run(Invocation::new(name, args)));
}

/// Draws the mixer docked at the bottom of the window.
/// Height of the strips area in the mixer window.
const STRIPS_HEIGHT: f32 = 330.0;

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    let mut open = state.windows.mixer;
    let area = state.workspace;
    let title = match project.mixer.get(state.selected_track) {
        Some(track) if state.selected_track > 0 => format!("Mixer — {}", track.name),
        _ => "Mixer — Master".to_owned(),
    };
    egui::Window::new(title)
        .id(egui::Id::new("mixer-window"))
        .open(&mut open)
        .default_pos(area.left_bottom() + egui::vec2(8.0, -(STRIPS_HEIGHT + 90.0)))
        .default_width((area.width() - 72.0).clamp(500.0, 1200.0))
        .max_width((area.width() - 72.0).max(500.0))
        .constrain_to(area)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui
                    .small_button("+ Insert")
                    .hint("Add an insert track")
                    .clicked()
                {
                    run(actions, "mixer.add_track", []);
                }
                ui.label(
                    RichText::new(
                        "Click a strip to select it · arrows under the strips set where it sends",
                    )
                    .small()
                    .color(theme::TEXT_DIM),
                );
            });
            let height = STRIPS_HEIGHT;
            ui.horizontal_top(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("mixer-strips")
                    .max_width((ui.available_width() - EFFECTS_WIDTH - 12.0).max(160.0))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 3.0;
                            for (index, track) in project.mixer.iter().enumerate() {
                                strip(ui, state, project, index, track, height, actions);
                                if index == 0 {
                                    ui.add_space(6.0);
                                }
                            }
                        });
                    });
                ui.separator();
                effects(ui, state, project, frame.plugins, actions);
            });
        });
    state.windows.mixer = open;
}

const EFFECTS_WIDTH: f32 = 270.0;

fn led(
    ui: &mut egui::Ui,
    on: bool,
    color: egui::Color32,
    text: &str,
    hint: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(22.0, 16.0), egui::Sense::click());
    let fill = if on { color } else { theme::RAISED };
    ui.painter().rect_filled(rect, 3.0, fill);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(10.0),
        if on {
            theme::BACKGROUND
        } else {
            theme::TEXT_DIM
        },
    );
    response.hint(hint)
}

#[allow(clippy::too_many_arguments)]
fn strip(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    index: usize,
    track: &MixerTrack,
    height: f32,
    actions: &mut Vec<Action>,
) {
    let number = index.to_string();
    let selected = state.selected_track == index;
    let color = if index == 0 {
        theme::HIGHLIGHT
    } else {
        theme::palette(index + 3)
    };
    let frame = egui::Frame::new()
        .fill(if selected {
            theme::RAISED
        } else {
            theme::PANEL_DARK
        })
        .stroke(egui::Stroke::new(
            1.0,
            if selected { color } else { theme::PANEL_DARK },
        ))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(3, 4));
    let response = frame.show(ui, |ui| {
        ui.set_width(STRIP_WIDTH);
        ui.set_height((height - 10.0).max(200.0));
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            let label = if index == 0 {
                "M".to_owned()
            } else {
                index.to_string()
            };
            ui.label(RichText::new(label).strong().size(13.0).color(color));
            let name = ui
                .add(
                    egui::Label::new(RichText::new(&track.name).small())
                        .truncate()
                        .sense(egui::Sense::click()),
                )
                .hint(format!(
                    "{} — click to select, right-click for options",
                    track.name
                ));
            if name.clicked() {
                state.selected_track = index;
            }
            if name.double_clicked() {
                state.open_dialog(Dialog::rename_track(index, &track.name));
            }
            name.context_menu(|ui| {
                if ui.button("Rename…").clicked() {
                    state.open_dialog(Dialog::rename_track(index, &track.name));
                    ui.close();
                }
                if index > 0
                    && ui
                        .button(RichText::new("Delete").color(theme::DANGER))
                        .clicked()
                {
                    run(actions, "mixer.remove_track", [number.clone()]);
                    ui.close();
                }
            });
            let pan = widgets::knob(ui, track.pan, -1.0, 1.0, 0.0, true);
            crate::automation::menu(&pan.response, &format!("mixer:{index}:pan"), actions);
            if pan.started {
                state.begin_gesture();
            }
            if let Some(value) = pan.changed {
                actions.push(Action::Coalesced(
                    Invocation::new("mixer.set_pan", [number.clone(), format!("{value:.3}")]),
                    state.gesture(),
                ));
            }
            pan.response.hint(format!("Pan {:+.0}", track.pan * 100.0));
            let fader_height = (ui.available_height() - 72.0).max(80.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(2.0);
                let levels = state.meters.get(index).copied().unwrap_or_default();
                widgets::meter(ui, levels, vec2(14.0, fader_height));
                let fader = widgets::fader(
                    ui,
                    track.volume,
                    MAX_VOLUME,
                    vec2(STRIP_WIDTH - 22.0, fader_height),
                );
                crate::automation::menu(&fader.response, &format!("mixer:{index}:volume"), actions);
                if fader.started {
                    state.begin_gesture();
                }
                if let Some(gain) = fader.changed {
                    actions.push(Action::Coalesced(
                        Invocation::new("mixer.set_volume", [number.clone(), format!("{gain:.4}")]),
                        state.gesture(),
                    ));
                }
            });
            let db = 20.0 * track.volume.max(1e-6).log10();
            let readout = if track.volume <= 1e-6 {
                "-inf".to_owned()
            } else {
                format!("{db:+.1} dB")
            };
            ui.label(
                RichText::new(readout)
                    .monospace()
                    .size(10.0)
                    .color(theme::TEXT_DIM),
            );
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                ui.add_space((STRIP_WIDTH - 48.0) / 2.0);
                if led(ui, !track.mute, theme::ACCENT, "ON", "Enable / mute").clicked() {
                    run(actions, "mixer.toggle_mute", [number.clone()]);
                }
                if index > 0 && led(ui, track.solo, theme::HIGHLIGHT, "S", "Solo").clicked() {
                    run(actions, "mixer.toggle_solo", [number.clone()]);
                }
            });
            let fx = track.effects.iter().filter(|e| e.enabled).count();
            ui.label(
                RichText::new(if fx > 0 {
                    format!("{fx} fx")
                } else {
                    " ".to_owned()
                })
                .small()
                .color(theme::ACCENT),
            );
            route_arrow(ui, state, project, index, actions);
        });
    });
    if response.response.interact(egui::Sense::click()).clicked() {
        state.selected_track = index;
    }
}

/// The arrow under a strip shows whether the selected insert sends here;
/// clicking it toggles that route.
fn route_arrow(
    ui: &mut egui::Ui,
    state: &UiState,
    project: &Project,
    target: usize,
    actions: &mut Vec<Action>,
) {
    let source = state.selected_track;
    let (rect, response) = ui.allocate_exact_size(vec2(STRIP_WIDTH, 14.0), egui::Sense::click());
    let Some(from) = project.mixer.get(source) else {
        return;
    };
    let possible = source > 0 && (target == 0 || target > source);
    if !possible {
        return;
    }
    let routed = if target == 0 {
        from.to_master
    } else {
        from.sends.iter().any(|s| s.target == target)
    };
    let c = rect.center();
    let color = if routed {
        theme::HIGHLIGHT
    } else if response.hovered() {
        theme::TEXT_DIM
    } else {
        theme::RAISED_HOVER
    };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            c + vec2(-7.0, -5.0),
            c + vec2(7.0, -5.0),
            c + vec2(0.0, 5.0),
        ],
        color,
        egui::Stroke::NONE,
    ));
    let hint = if routed {
        "Routed here — click to remove"
    } else {
        "Click to route the selected insert here"
    };
    if response.hint(hint).clicked() {
        let invocation = if target == 0 {
            Invocation::new(
                "mixer.route_to_master",
                [
                    source.to_string(),
                    if routed { "off" } else { "on" }.to_owned(),
                ],
            )
        } else {
            Invocation::new(
                "mixer.set_send",
                [
                    source.to_string(),
                    target.to_string(),
                    if routed { "0" } else { "1" }.to_owned(),
                ],
            )
        };
        actions.push(Action::Run(invocation));
    }
}

fn effects(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    plugins: &crate::Plugins,
    actions: &mut Vec<Action>,
) {
    let index = state.selected_track;
    let Some(track) = project.mixer.get(index) else {
        return;
    };
    let number = index.to_string();
    ui.vertical(|ui| {
        ui.set_width(EFFECTS_WIDTH);
        let title = if index == 0 {
            "Master".to_owned()
        } else {
            format!("Insert {index}")
        };
        let heading = if track.name == title {
            title
        } else {
            format!("{title} — {}", track.name)
        };
        ui.label(RichText::new(heading).strong().size(14.0));
        egui::ScrollArea::vertical()
            .id_salt("effect-chain")
            .show(ui, |ui| {
                for slot in 0..MAX_EFFECT_SLOTS {
                    slot_row(ui, state, track, &number, slot, plugins, actions);
                }
                ui.label(
                    RichText::new("Click an effect's name to open its window")
                        .small()
                        .color(theme::TEXT_DIM),
                );
                if index > 0 {
                    routing(ui, state, project, index, track, actions);
                }
            });
    });
}

fn track_index(number: &str) -> usize {
    number.parse().unwrap_or(0)
}

fn slot_row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: &MixerTrack,
    number: &str,
    slot: usize,
    plugins: &crate::Plugins,
    actions: &mut Vec<Action>,
) {
    let slot_no = (slot + 1).to_string();
    let selected = state.mixer_slot == slot;
    let effect = track.effects.get(slot);
    egui::Frame::new()
        .fill(if selected && effect.is_some() {
            theme::RAISED
        } else {
            theme::PANEL_DARK
        })
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(4, 1))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&slot_no)
                        .small()
                        .monospace()
                        .color(theme::TEXT_DIM),
                );
                match effect {
                    Some(effect) => {
                        if led(ui, effect.enabled, theme::ACCENT, "", "Enable / bypass").clicked() {
                            run(
                                actions,
                                "mixer.toggle_effect",
                                [number.to_owned(), slot_no.clone()],
                            );
                        }
                        let name = RichText::new(effect.label()).color(if effect.enabled {
                            theme::TEXT
                        } else {
                            theme::TEXT_DIM
                        });
                        if ui
                            .add(egui::Label::new(name).sense(egui::Sense::click()))
                            .hint("Open or close this effect's window")
                            .clicked()
                        {
                            state.mixer_slot = slot;
                            crate::effect_window::toggle(state, track_index(number), slot);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("×").hint("Remove").clicked() {
                                run(
                                    actions,
                                    "mixer.remove_effect",
                                    [number.to_owned(), slot_no.clone()],
                                );
                            }
                            if slot + 1 < track.effects.len()
                                && ui
                                    .small_button(RichText::new("▼").monospace())
                                    .hint("Move down")
                                    .clicked()
                            {
                                run(
                                    actions,
                                    "mixer.move_effect",
                                    [number.to_owned(), slot_no.clone(), (slot + 2).to_string()],
                                );
                            }
                            if slot > 0
                                && ui
                                    .small_button(RichText::new("▲").monospace())
                                    .hint("Move up")
                                    .clicked()
                            {
                                run(
                                    actions,
                                    "mixer.move_effect",
                                    [number.to_owned(), slot_no.clone(), slot.to_string()],
                                );
                            }
                        });
                    }
                    None if slot == track.effects.len() => {
                        ui.menu_button(
                            RichText::new("+ add effect").color(theme::TEXT_DIM),
                            |ui| {
                                for kind in EFFECT_KINDS {
                                    if ui.button(kind.name()).clicked() {
                                        run(
                                            actions,
                                            "mixer.add_effect",
                                            [number.to_owned(), kind.name().to_owned()],
                                        );
                                        state.mixer_slot = slot;
                                        state.effect_windows.push((track_index(number), slot));
                                        ui.close();
                                    }
                                }
                                ui.separator();
                                ui.menu_button("Plugin effect", |ui| {
                                    let chosen = crate::plugins::add_menu(
                                        ui,
                                        plugins,
                                        false,
                                        actions,
                                        |p| {
                                            Invocation::new(
                                                "mixer.add_plugin",
                                                [number.to_owned(), p.id.clone(), p.name.clone()],
                                            )
                                        },
                                    );
                                    if chosen {
                                        state.mixer_slot = slot;
                                    }
                                });
                            },
                        );
                    }
                    None => {
                        ui.label(RichText::new("empty").small().color(theme::GRID_BAR));
                    }
                }
            });
        });
}

fn routing(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    index: usize,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    let number = index.to_string();
    ui.separator();
    ui.horizontal(|ui| {
        let mut to_master = track.to_master;
        if ui.checkbox(&mut to_master, "To master").changed() {
            run(
                actions,
                "mixer.route_to_master",
                [
                    number.clone(),
                    if to_master { "on" } else { "off" }.to_owned(),
                ],
            );
        }
        ui.menu_button("+ Send", |ui| {
            for (target, other) in project.mixer.iter().enumerate().skip(index + 1) {
                if track.sends.iter().any(|s| s.target == target) {
                    continue;
                }
                if ui.button(format!("{target} {}", other.name)).clicked() {
                    run(
                        actions,
                        "mixer.set_send",
                        [number.clone(), target.to_string(), "1".to_owned()],
                    );
                    ui.close();
                }
            }
        });
    });
    for send in &track.sends {
        ui.horizontal(|ui| {
            let name = project
                .mixer
                .get(send.target)
                .map_or("?", |t| t.name.as_str());
            ui.add_sized(
                vec2(72.0, 16.0),
                egui::Label::new(
                    RichText::new(format!("→ {} {name}", send.target))
                        .monospace()
                        .small(),
                ),
            );
            let mut level = send.level;
            let response =
                ui.add(egui::Slider::new(&mut level, 0.0..=MAX_VOLUME).show_value(false));
            if response.drag_started() {
                state.begin_gesture();
            }
            if response.changed() && level > 0.0 {
                actions.push(Action::Coalesced(
                    Invocation::new(
                        "mixer.set_send",
                        [
                            number.clone(),
                            send.target.to_string(),
                            format!("{level:.3}"),
                        ],
                    ),
                    state.gesture(),
                ));
            }
            if ui.small_button("×").hint("Remove send").clicked() {
                run(
                    actions,
                    "mixer.set_send",
                    [number.clone(), send.target.to_string(), "0".to_owned()],
                );
            }
        });
    }
}
