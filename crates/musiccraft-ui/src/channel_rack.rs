//! The channel rack: one row per channel with mute, pan, volume, routing, name
//! and step buttons for the selected pattern.

use egui::{RichText, Sense, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{BUILTIN_PREFIX, BUILTIN_SAMPLES, TICKS_PER_STEP};
use musiccraft_project::{Channel, DEFAULT_CHANNEL_VOLUME, Instrument, MAX_VOLUME, Project};

use crate::dialogs::Dialog;
use crate::{Action, Frame, UiState, theme, widgets};

const STEP_SIZE: egui::Vec2 = egui::Vec2::new(16.0, 22.0);

fn run(actions: &mut Vec<Action>, name: &str, args: impl IntoIterator<Item = String>) {
    actions.push(Action::Run(Invocation::new(name, args)));
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.channel_rack;
    let project = frame.project;
    egui::Window::new("Channel rack")
        .id(egui::Id::new("channel-rack"))
        .open(&mut open)
        .default_pos([200.0, 110.0])
        .default_size([640.0, 260.0])
        .show(ctx, |ui| {
            header(ui, project, actions);
            ui.separator();
            egui::ScrollArea::both()
                .id_salt("rack-scroll")
                .auto_shrink([false, true])
                .max_height(420.0)
                .show(ui, |ui| {
                    for (index, channel) in project.channels.iter().enumerate() {
                        row(ui, state, project, index, channel, actions);
                    }
                    if project.channels.is_empty() {
                        ui.label(
                            RichText::new(
                                "No channels yet. Add a sound from the browser or below.",
                            )
                            .color(theme::TEXT_DIM),
                        );
                    }
                });
            ui.separator();
            add_menu(ui, actions);
        });
    state.windows.channel_rack = open;
}

fn header(ui: &mut egui::Ui, project: &Project, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let pattern = project.current_pattern();
        ui.label(
            RichText::new(&pattern.name)
                .strong()
                .color(theme::HIGHLIGHT),
        );
        ui.label(RichText::new("steps").color(theme::TEXT_DIM));
        let mut steps = pattern.length_steps;
        let response = ui.add(egui::DragValue::new(&mut steps).range(1..=256).speed(0.2));
        if response.changed() && steps != pattern.length_steps {
            run(
                actions,
                "pattern.set_length",
                [(project.current_pattern + 1).to_string(), steps.to_string()],
            );
        }
    });
}

fn row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let number = (index + 1).to_string();
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        if widgets::dot(ui, !channel.mute, theme::ACCENT)
            .on_hover_text("Mute")
            .clicked()
        {
            run(actions, "channel.toggle_mute", [number.clone()]);
        }
        let pan = widgets::knob(ui, channel.pan, -1.0, 1.0, 0.0, true);
        if pan.started {
            state.begin_gesture();
        }
        if let Some(value) = pan.changed {
            actions.push(Action::Coalesced(
                Invocation::new("channel.set_pan", [number.clone(), format!("{value:.3}")]),
                state.gesture(),
            ));
        }
        pan.response
            .on_hover_text(format!("Pan {:+.0}%", channel.pan * 100.0));
        let volume = widgets::knob(
            ui,
            channel.volume,
            0.0,
            MAX_VOLUME,
            DEFAULT_CHANNEL_VOLUME,
            false,
        );
        if volume.started {
            state.begin_gesture();
        }
        if let Some(value) = volume.changed {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_volume",
                    [number.clone(), format!("{value:.3}")],
                ),
                state.gesture(),
            ));
        }
        volume
            .response
            .on_hover_text(format!("Volume {:.0}%", channel.volume * 100.0));

        let mut track = channel.mixer_track;
        let route = ui
            .add(
                egui::DragValue::new(&mut track)
                    .range(0..=project.mixer.len().saturating_sub(1))
                    .speed(0.1)
                    .custom_formatter(|v, _| {
                        if v < 0.5 {
                            "M".to_owned()
                        } else {
                            format!("{v:.0}")
                        }
                    }),
            )
            .on_hover_text("Mixer track (M = master)");
        if route.changed() && track != channel.mixer_track {
            run(
                actions,
                "channel.route",
                [number.clone(), track.to_string()],
            );
        }

        let selected = state.selected_channel == index;
        let name = egui::Button::new(RichText::new(&channel.name).color(if selected {
            theme::BACKGROUND
        } else {
            theme::TEXT
        }))
        .fill(if selected {
            theme::palette(index)
        } else {
            theme::RAISED
        })
        .min_size(vec2(120.0, STEP_SIZE.y));
        let response = ui.add(name).on_hover_text(match &channel.instrument {
            Instrument::Sampler { sample } => format!("Sampler: {sample}"),
            Instrument::Synth => "Synth".to_owned(),
        });
        if response.clicked() {
            state.selected_channel = index;
        }
        if response.double_clicked() {
            state.selected_channel = index;
            state.windows.piano_roll = true;
        }
        response.context_menu(|ui| context_menu(ui, state, project, index, channel, actions));

        steps(ui, project, index, channel, actions);
    });
}

fn context_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let number = (index + 1).to_string();
    if ui.button("Open in piano roll").clicked() {
        state.selected_channel = index;
        state.windows.piano_roll = true;
        ui.close();
    }
    if ui.button("Rename…").clicked() {
        state.open_dialog(Dialog::rename_channel(index, &channel.name));
        ui.close();
    }
    if let Instrument::Sampler { sample } = &channel.instrument {
        if ui.button("Change sample…").clicked() {
            state.open_dialog(Dialog::set_sample(index, sample.clone()));
            ui.close();
        }
    }
    if ui.button("Solo").clicked() {
        run(actions, "channel.solo", [number.clone()]);
        ui.close();
    }
    if ui.button("Clone").clicked() {
        run(actions, "channel.clone", [number.clone()]);
        ui.close();
    }
    ui.add_enabled_ui(index > 0, |ui| {
        if ui.button("Move up").clicked() {
            run(actions, "channel.move", [number.clone(), index.to_string()]);
            ui.close();
        }
    });
    ui.add_enabled_ui(index + 1 < project.channels.len(), |ui| {
        if ui.button("Move down").clicked() {
            run(
                actions,
                "channel.move",
                [number.clone(), (index + 2).to_string()],
            );
            ui.close();
        }
    });
    if ui.button("Clear notes in pattern").clicked() {
        run(actions, "note.clear", [number.clone()]);
        ui.close();
    }
    ui.separator();
    if ui
        .button(RichText::new("Delete").color(theme::DANGER))
        .clicked()
    {
        run(actions, "channel.remove", [number]);
        ui.close();
    }
}

fn steps(
    ui: &mut egui::Ui,
    project: &Project,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let pattern = project.current_pattern();
    let count = pattern.length_ticks().div_ceil(TICKS_PER_STEP);
    let mut on = vec![false; count as usize];
    for note in pattern.notes_of(channel.id) {
        if let Some(slot) = on.get_mut((note.start / TICKS_PER_STEP) as usize) {
            *slot = true;
        }
    }
    ui.add_space(6.0);
    for (step, lit) in on.iter().enumerate() {
        let (rect, response) = ui.allocate_exact_size(STEP_SIZE, Sense::click());
        let base = if (step / 4) % 2 == 0 {
            theme::STEP_A
        } else {
            theme::STEP_B
        };
        let fill = if *lit { theme::palette(index) } else { base };
        let fill = if response.hovered() && !lit {
            theme::RAISED
        } else {
            fill
        };
        ui.painter().rect_filled(rect.shrink(1.0), 3.0, fill);
        if response.clicked() {
            actions.push(Action::Run(Invocation::new(
                "step.toggle",
                [(index + 1).to_string(), step.to_string()],
            )));
            if !lit {
                actions.push(Action::PreviewShort {
                    channel: index,
                    key: 60,
                });
            }
        }
    }
}

fn add_menu(ui: &mut egui::Ui, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.menu_button("+ Add channel", |ui| {
            for name in BUILTIN_SAMPLES {
                if ui.button(name.replace('_', " ")).clicked() {
                    run(
                        actions,
                        "channel.add_sampler",
                        [format!("{BUILTIN_PREFIX}{name}")],
                    );
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Synth").clicked() {
                actions.push(Action::Run(Invocation::bare("channel.add_synth")));
                ui.close();
            }
        });
    });
}
