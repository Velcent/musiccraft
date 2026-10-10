//! The mixer: one strip per track plus the selected track's effect chain.

use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{EFFECT_KINDS, MAX_EFFECT_SLOTS};
use musiccraft_project::{MAX_VOLUME, MixerTrack, Project};

use crate::dialogs::Dialog;
use crate::{Action, Frame, UiState, theme, widgets};

const STRIP_WIDTH: f32 = 64.0;

fn run(actions: &mut Vec<Action>, name: &str, args: impl IntoIterator<Item = String>) {
    actions.push(Action::Run(Invocation::new(name, args)));
}

/// Draws the mixer docked at the bottom of the window.
pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    egui::Panel::bottom("mixer")
        .resizable(true)
        .default_size(300.0)
        .min_size(220.0)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mixer").strong().size(15.0));
                if ui
                    .small_button("+ Insert")
                    .on_hover_text("Add an insert track")
                    .clicked()
                {
                    run(actions, "mixer.add_track", []);
                }
            });
            ui.horizontal_top(|ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("mixer-strips")
                    .max_width((ui.available_width() - 300.0).max(200.0))
                    .show(ui, |ui| {
                        ui.horizontal_top(|ui| {
                            for (index, track) in project.mixer.iter().enumerate() {
                                strip(ui, state, index, track, actions);
                            }
                        });
                    });
                ui.separator();
                effects(ui, state, project, actions);
            });
        });
}

fn strip(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    let number = index.to_string();
    let selected = state.selected_track == index;
    let frame = egui::Frame::new()
        .fill(theme::PANEL_DARK)
        .stroke(egui::Stroke::new(
            1.0,
            if selected {
                theme::ACCENT
            } else {
                theme::PANEL_DARK
            },
        ))
        .corner_radius(4.0)
        .inner_margin(4.0);
    frame.show(ui, |ui| {
        ui.set_width(STRIP_WIDTH);
        ui.vertical_centered(|ui| {
            let label = if index == 0 {
                "Master".to_owned()
            } else {
                format!("{index}")
            };
            let title = ui.selectable_label(selected, RichText::new(label).strong());
            if title.clicked() {
                state.selected_track = index;
            }
            ui.label(RichText::new(&track.name).small().color(theme::TEXT_DIM));
            title.context_menu(|ui| {
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
            ui.horizontal(|ui| {
                let levels = state.meters.get(index).copied().unwrap_or_default();
                widgets::meter(ui, levels, vec2(12.0, 120.0));
                let mut volume = track.volume;
                let fader = ui.add(
                    egui::Slider::new(&mut volume, 0.0..=MAX_VOLUME)
                        .vertical()
                        .show_value(false),
                );
                if fader.drag_started() {
                    state.begin_gesture();
                }
                if fader.changed() {
                    actions.push(Action::Coalesced(
                        Invocation::new(
                            "mixer.set_volume",
                            [number.clone(), format!("{volume:.3}")],
                        ),
                        state.gesture(),
                    ));
                }
                fader.on_hover_text(format!("{:.1} dB", 20.0 * track.volume.max(1e-6).log10()));
            });
            let pan = widgets::knob(ui, track.pan, -1.0, 1.0, 0.0, true);
            if pan.started {
                state.begin_gesture();
            }
            if let Some(value) = pan.changed {
                actions.push(Action::Coalesced(
                    Invocation::new("mixer.set_pan", [number.clone(), format!("{value:.3}")]),
                    state.gesture(),
                ));
            }
            ui.horizontal(|ui| {
                let mute = egui::Button::new(RichText::new("M").small()).fill(if track.mute {
                    theme::DANGER
                } else {
                    theme::RAISED
                });
                if ui.add(mute).on_hover_text("Mute").clicked() {
                    run(actions, "mixer.toggle_mute", [number.clone()]);
                }
                if index > 0 {
                    let solo = egui::Button::new(RichText::new("S").small()).fill(if track.solo {
                        theme::HIGHLIGHT
                    } else {
                        theme::RAISED
                    });
                    if ui.add(solo).on_hover_text("Solo").clicked() {
                        run(actions, "mixer.toggle_solo", [number.clone()]);
                    }
                }
            });
            let fx = track.effects.len();
            ui.label(RichText::new(format!("{fx} fx")).small().color(if fx > 0 {
                theme::ACCENT
            } else {
                theme::TEXT_DIM
            }));
        });
    });
}

fn effects(ui: &mut egui::Ui, state: &mut UiState, project: &Project, actions: &mut Vec<Action>) {
    let index = state.selected_track;
    let Some(track) = project.mixer.get(index) else {
        return;
    };
    let number = index.to_string();
    ui.vertical(|ui| {
        ui.set_width(290.0);
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
        ui.label(RichText::new(heading).strong());
        egui::ScrollArea::vertical()
            .id_salt("effect-chain")
            .max_height(260.0)
            .show(ui, |ui| {
                for (slot, effect) in track.effects.iter().enumerate() {
                    let slot_no = (slot + 1).to_string();
                    egui::Frame::new()
                        .fill(theme::PANEL_DARK)
                        .corner_radius(4.0)
                        .inner_margin(6.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let mut enabled = effect.enabled;
                                if ui
                                    .checkbox(&mut enabled, "")
                                    .on_hover_text("Enable / bypass")
                                    .changed()
                                {
                                    run(
                                        actions,
                                        "mixer.toggle_effect",
                                        [number.clone(), slot_no.clone()],
                                    );
                                }
                                ui.label(
                                    RichText::new(effect.kind.name())
                                        .strong()
                                        .color(theme::ACCENT),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("×").on_hover_text("Remove").clicked() {
                                            run(
                                                actions,
                                                "mixer.remove_effect",
                                                [number.clone(), slot_no.clone()],
                                            );
                                        }
                                        if slot + 1 < track.effects.len()
                                            && ui.small_button("▼").clicked()
                                        {
                                            run(
                                                actions,
                                                "mixer.move_effect",
                                                [
                                                    number.clone(),
                                                    slot_no.clone(),
                                                    (slot + 2).to_string(),
                                                ],
                                            );
                                        }
                                        if slot > 0 && ui.small_button("▲").clicked() {
                                            run(
                                                actions,
                                                "mixer.move_effect",
                                                [number.clone(), slot_no.clone(), slot.to_string()],
                                            );
                                        }
                                    },
                                );
                            });
                            for (p, spec) in effect.kind.params().iter().enumerate() {
                                let mut value = effect.values[p];
                                ui.horizontal(|ui| {
                                    ui.add_sized(
                                        vec2(72.0, 16.0),
                                        egui::Label::new(RichText::new(spec.name).small()),
                                    );
                                    let mut slider =
                                        egui::Slider::new(&mut value, spec.min..=spec.max)
                                            .suffix(format!(" {}", spec.unit));
                                    if spec.integer {
                                        slider = slider.integer();
                                    }
                                    if spec.max / spec.min.max(1e-3) > 100.0 && spec.min > 0.0 {
                                        slider = slider.logarithmic(true);
                                    }
                                    let response = ui.add(slider);
                                    if response.drag_started() {
                                        state.begin_gesture();
                                    }
                                    if response.changed() {
                                        actions.push(Action::Coalesced(
                                            Invocation::new(
                                                "mixer.set_effect_param",
                                                [
                                                    number.clone(),
                                                    slot_no.clone(),
                                                    spec.name.to_owned(),
                                                    format!("{value}"),
                                                ],
                                            ),
                                            state.gesture(),
                                        ));
                                    }
                                });
                            }
                        });
                    ui.add_space(4.0);
                }
            });
        if index > 0 {
            routing(ui, state, project, index, track, actions);
        }
        ui.add_enabled_ui(track.effects.len() < MAX_EFFECT_SLOTS, |ui| {
            ui.menu_button("+ Add effect", |ui| {
                for kind in EFFECT_KINDS {
                    if ui.button(kind.name()).clicked() {
                        run(
                            actions,
                            "mixer.add_effect",
                            [number.clone(), kind.name().to_owned()],
                        );
                        ui.close();
                    }
                }
            });
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
                egui::Label::new(RichText::new(format!("to {} {name}", send.target)).small()),
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
            if ui.small_button("×").on_hover_text("Remove send").clicked() {
                run(
                    actions,
                    "mixer.set_send",
                    [number.clone(), send.target.to_string(), "0".to_owned()],
                );
            }
        });
    }
}
