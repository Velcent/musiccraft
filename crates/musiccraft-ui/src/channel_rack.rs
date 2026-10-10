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
    let area = state.workspace;
    let window = egui::Window::new("Channel rack")
        .id(egui::Id::new("channel-rack"))
        .open(&mut open)
        .default_pos(area.left_top() + egui::vec2(150.0, 40.0))
        .default_width((area.width() - 200.0).clamp(420.0, 760.0))
        .constrain_to(area)
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
    // Sounds dragged from the browser and released over the rack become channels.
    if let Some(window) = window {
        let over = ctx
            .pointer_hover_pos()
            .is_some_and(|p| window.response.rect.contains(p));
        if over && egui::DragAndDrop::has_payload_of_type::<crate::browser::Dragged>(ctx) {
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("rack-drop"),
            ))
            .rect_stroke(
                window.response.rect,
                6.0,
                egui::Stroke::new(2.0, theme::ACCENT),
                egui::StrokeKind::Inside,
            );
            if ctx.input(|i| i.pointer.any_released()) {
                if let Some(dragged) =
                    egui::DragAndDrop::take_payload::<crate::browser::Dragged>(ctx)
                {
                    actions.push(Action::Run(dragged.add_channel()));
                }
            }
        }
    }
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
        ui.label(RichText::new("swing").color(theme::TEXT_DIM));
        let mut swing = (pattern.swing * 100.0).round();
        let response = ui.add(
            egui::DragValue::new(&mut swing)
                .range(0.0..=100.0)
                .speed(0.5)
                .suffix("%"),
        );
        if response.changed() {
            run(
                actions,
                "pattern.set_swing",
                [
                    (project.current_pattern + 1).to_string(),
                    format!("{swing}"),
                ],
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
        crate::automation::menu(&pan.response, &format!("channel:{number}:pan"), actions);
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
        crate::automation::menu(
            &volume.response,
            &format!("channel:{number}:volume"),
            actions,
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
            // Clicking the selected channel again closes its settings.
            let reopen = state.selected_channel != index || !state.windows.channel_settings;
            state.selected_channel = index;
            state.windows.channel_settings = reopen;
        }
        response.context_menu(|ui| context_menu(ui, state, project, index, channel, actions));

        steps(ui, state, project, index, channel, actions);
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
    if ui.button("Channel settings").clicked() {
        state.selected_channel = index;
        state.windows.channel_settings = true;
        ui.close();
    }
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

/// Whether a channel's notes are all plain steps (key C5, one step long, on
/// the step grid), so the step buttons show them fully.
fn is_step_only(notes: &[&musiccraft_project::Note]) -> bool {
    notes.iter().all(|n| {
        n.key == musiccraft_project::STEP_KEY
            && n.length == TICKS_PER_STEP
            && n.start % TICKS_PER_STEP == 0
    })
}

fn steps(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let pattern = project.current_pattern();
    let count = pattern.length_ticks().div_ceil(TICKS_PER_STEP);
    let notes: Vec<&musiccraft_project::Note> = pattern.notes_of(channel.id).collect();
    ui.add_space(6.0);
    if !is_step_only(&notes) {
        mini_roll(ui, state, index, &notes, count);
        return;
    }
    let mut on = vec![false; count as usize];
    for note in &notes {
        if let Some(slot) = on.get_mut((note.start / TICKS_PER_STEP) as usize) {
            *slot = true;
        }
    }
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

/// A miniature piano roll in place of the step buttons, for melodic channels.
fn mini_roll(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    notes: &[&musiccraft_project::Note],
    steps: u32,
) {
    let size = vec2(STEP_SIZE.x * steps as f32, STEP_SIZE.y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect.shrink(1.0), 3.0, theme::PANEL_DARK);
    for bar in 0..steps.div_ceil(4) {
        let x = rect.left() + bar as f32 * 4.0 * STEP_SIZE.x;
        if bar % 2 == 1 {
            let band = egui::Rect::from_min_max(
                egui::pos2(x, rect.top() + 1.0),
                egui::pos2(
                    (x + 4.0 * STEP_SIZE.x).min(rect.right()),
                    rect.bottom() - 1.0,
                ),
            );
            painter.rect_filled(band, 0.0, theme::BLACK_KEY_ROW);
        }
    }
    let (lo, hi) = notes
        .iter()
        .fold((127_u8, 0_u8), |(lo, hi), n| (lo.min(n.key), hi.max(n.key)));
    let span = f32::from(hi.saturating_sub(lo).max(1));
    let total = (steps * TICKS_PER_STEP) as f32;
    let color = theme::palette(index);
    for note in notes {
        let x0 = rect.left() + note.start as f32 / total * rect.width();
        let x1 = rect.left() + (note.start + note.length) as f32 / total * rect.width();
        let t = if hi == lo {
            0.5
        } else {
            f32::from(note.key - lo) / span
        };
        let y = rect.bottom() - 4.0 - t * (rect.height() - 8.0);
        painter.line_segment(
            [
                egui::pos2(x0 + 1.0, y),
                egui::pos2(x1.max(x0 + 3.0) - 1.0, y),
            ],
            egui::Stroke::new(2.5, color),
        );
    }
    if response.hovered() {
        painter.rect_stroke(
            rect.shrink(1.0),
            3.0,
            egui::Stroke::new(1.0, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
    if response
        .on_hover_text("Melodic pattern: click to open the piano roll")
        .clicked()
    {
        state.selected_channel = index;
        state.windows.piano_roll = true;
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
