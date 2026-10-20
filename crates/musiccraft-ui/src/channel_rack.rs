//! The channel rack: one row per channel with mute, pan, volume, routing, name
//! and step buttons for the selected pattern.

use crate::hint::Hint;
use egui::{PointerButton, RichText, Sense, vec2};
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
    state.step_layout.clear();
    let window = egui::Window::new("Channel rack")
        .id(egui::Id::new("channel-rack"))
        .open(&mut open)
        .default_pos(area.left_top() + egui::vec2(150.0, 40.0))
        .default_width((area.width() - 200.0).clamp(420.0, 760.0))
        .constrain_to(area)
        .show(ctx, |ui| {
            header(ui, state, project, actions);
            ui.separator();
            egui::ScrollArea::both()
                .id_salt("rack-scroll")
                .auto_shrink([false, true])
                .max_height(420.0)
                .show(ui, |ui| {
                    for (index, channel) in project.channels.iter().enumerate() {
                        let shown = state
                            .rack_group
                            .as_ref()
                            .is_none_or(|group| *group == channel.group);
                        if shown {
                            row(ui, state, project, index, channel, actions);
                        }
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
            if state.graph.is_some() {
                ui.separator();
                graph_editor(ui, state, project, actions);
            }
            ui.separator();
            add_menu(ui, frame.plugins, actions);
        });
    state.windows.channel_rack = open;
    // A stroke whose row was not drawn when the button was let go is dropped.
    if !ctx.input(|i| i.pointer.any_down()) {
        state.step_paint = None;
    }
    // Sounds dragged from the browser and released over the rack become channels.
    if let Some(window) = window {
        let over = ctx
            .pointer_hover_pos()
            .is_some_and(|p| window.response.rect.contains(p));
        if open && over && !state.typing_keyboard && !ctx.egui_wants_keyboard_input() {
            keys(ctx, state, project, actions);
        }
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

fn header(ui: &mut egui::Ui, state: &mut UiState, project: &Project, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        let mut groups: Vec<&str> = project
            .channels
            .iter()
            .map(|c| c.group.as_str())
            .filter(|g| !g.is_empty())
            .collect();
        groups.sort_unstable();
        groups.dedup();
        if state
            .rack_group
            .as_ref()
            .is_some_and(|g| !g.is_empty() && !groups.contains(&g.as_str()))
        {
            state.rack_group = None;
        }
        // Always shown, as a fixed part of the header, even before any
        // channel has a group.
        {
            egui::ComboBox::from_id_salt("rack-group")
                .width(90.0)
                .selected_text(match &state.rack_group {
                    None => "All".to_owned(),
                    Some(g) if g.is_empty() => "Unsorted".to_owned(),
                    Some(g) => g.clone(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut state.rack_group, None, "All");
                    for group in &groups {
                        ui.selectable_value(
                            &mut state.rack_group,
                            Some((*group).to_owned()),
                            *group,
                        );
                    }
                    ui.selectable_value(&mut state.rack_group, Some(String::new()), "Unsorted");
                })
                .response
                .on_hover_text("Show one display filter group (right-click a channel → Group…)");
        }
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
        let mut open = state.graph.is_some();
        if ui
            .toggle_value(&mut open, "Graph")
            .on_hover_text(
                "Edit velocity, pan, fine pitch and more per step of the selected channel",
            )
            .changed()
        {
            state.graph = open.then_some(crate::piano_roll::LaneMode::Velocity);
        }
        ui.label(RichText::new("swing").color(theme::TEXT_DIM))
            .on_hover_text("Project swing; each channel's swing mix scales it");
        let mut swing = (project.swing * 100.0).round();
        let response = ui.add(
            egui::DragValue::new(&mut swing)
                .range(0.0..=100.0)
                .speed(0.5)
                .suffix("%"),
        );
        if response.changed() {
            run(actions, "project.set_swing", [format!("{swing}")]);
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
            .hint("Mute")
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
            .hint(format!("Pan {:+.0}%", channel.pan * 100.0));
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
            .hint(format!("Volume {:.0}%", channel.volume * 100.0));

        let mut track = channel.mixer_track;
        let route = ui
            .add_sized(
                vec2(28.0, 18.0),
                egui::DragValue::new(&mut track)
                    .range(0..=project.mixer.len().saturating_sub(1))
                    .speed(0.1)
                    .custom_formatter(|v, _| {
                        if v < 0.5 {
                            "M".to_owned()
                        } else {
                            format!("{v:.0}")
                        }
                    })
                    .custom_parser(|text| {
                        let text = text.trim().trim_start_matches('→').trim();
                        if text.eq_ignore_ascii_case("mst") || text.eq_ignore_ascii_case("m") {
                            Some(0.0)
                        } else {
                            text.parse().ok()
                        }
                    }),
            )
            .hint(format!(
                "Plays through mixer track: {}. Drag up or down to change",
                route_name(project, channel.mixer_track)
            ));
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
        let response = ui.add(name).hint(match &channel.instrument {
            Instrument::Sampler { sample } => format!("Sampler: {sample}"),
            Instrument::Synth => "Synth".to_owned(),
            Instrument::Antiserum { patch } => format!("Antiserum: {}", patch.name),
            Instrument::DrumSynth => "Drum synth".to_owned(),
            Instrument::Pluck => "Pluck".to_owned(),
            Instrument::Fm => "FM synth".to_owned(),
            Instrument::Keys => "Keys".to_owned(),
            Instrument::Additive => "Additive synth".to_owned(),
            Instrument::Granular { sample } => format!("Granular: {sample}"),
            Instrument::Resynth { sample } => format!("Resynth: {sample}"),
            Instrument::Kit { .. } => "Drum kit (16 pads)".to_owned(),
            Instrument::Zones { zones } => format!("Zone sampler ({} zones)", zones.len()),
            Instrument::Layer { channels } => format!("Layer of {} channels", channels.len()),
            Instrument::Plugin { plugin } => format!("Plugin: {} ({})", plugin.name, plugin.id),
            Instrument::MidiOut { midi_channel } => {
                format!("MIDI Out: external gear on MIDI channel {midi_channel}")
            }
        });
        if response.clicked() {
            // Clicking the selected channel again closes its settings.
            let reopen = state.selected_channel != index || !state.windows.channel_settings;
            state.selected_channel = index;
            state.windows.channel_settings = reopen;
        }
        response.context_menu(|ui| context_menu(ui, state, project, index, channel, actions));

        steps(ui, state, project, index, channel, actions);
        // The selection switch at the end of the row.
        let (rect, switch) = ui.allocate_exact_size(vec2(8.0, STEP_SIZE.y), egui::Sense::click());
        let fill = if selected {
            theme::ACCENT
        } else {
            theme::RAISED
        };
        ui.painter()
            .rect_filled(rect.shrink2(vec2(1.0, 2.0)), 2.0, fill);
        if switch.hint("Select this channel").clicked() {
            state.selected_channel = index;
        }
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
    if ui.button("Group…").clicked() {
        state.open_dialog(crate::dialogs::Dialog::channel_group(index, &channel.group));
        ui.close();
    }
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
    ui.menu_button("Steps", |ui| {
        for every in [2, 4, 8] {
            if ui.button(format!("Fill every {every} steps")).clicked() {
                run(actions, "step.fill", [number.clone(), every.to_string()]);
                ui.close();
            }
        }
        if ui.button("Shift left").clicked() {
            run(actions, "step.shift", [number.clone(), "-1".to_owned()]);
            ui.close();
        }
        if ui.button("Shift right").clicked() {
            run(actions, "step.shift", [number.clone(), "1".to_owned()]);
            ui.close();
        }
        if ui.button("Clear").clicked() {
            run(actions, "note.clear", [number.clone()]);
            ui.close();
        }
    });
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
    // Alternate beats of the pattern's signature are shaded.
    let beat_steps = (pattern.time_signature.beat_ticks() / TICKS_PER_STEP).max(1) as usize;
    let notes: Vec<&musiccraft_project::Note> = pattern.notes_of(channel.id).collect();
    ui.add_space(6.0);
    if !is_step_only(&notes) {
        mini_roll(ui, state, (index, beat_steps as u32), &notes, count);
        return;
    }
    let mut on = vec![false; count as usize];
    for note in &notes {
        if let Some(slot) = on.get_mut((note.start / TICKS_PER_STEP) as usize) {
            *slot = true;
        }
    }
    let pointer = ui.input(|i| {
        (
            i.pointer.latest_pos(),
            i.pointer.button_pressed(PointerButton::Primary),
            i.pointer.button_pressed(PointerButton::Secondary),
            i.pointer.any_down(),
        )
    });
    let (pos, left_pressed, right_pressed, down) = pointer;
    // The stroke ends when the button is let go: every step it passed
    // changes in one undo step.
    if !down
        && state
            .step_paint
            .as_ref()
            .is_some_and(|s| s.channel == index)
    {
        if let Some(stroke) = state.step_paint.take() {
            let changes = stroke
                .steps
                .iter()
                .filter(|step| on.get(**step).copied() != Some(stroke.on))
                .map(|step| {
                    Invocation::new(
                        "step.set",
                        [
                            (index + 1).to_string(),
                            step.to_string(),
                            if stroke.on { "on" } else { "off" }.to_owned(),
                        ],
                    )
                })
                .collect();
            actions.extend(batch(changes).map(Action::Run));
        }
    }
    for (step, lit) in on.iter().enumerate() {
        let (rect, response) = ui.allocate_exact_size(STEP_SIZE, Sense::click_and_drag());
        if step < 2 {
            state.step_layout.push((index, rect));
        }
        let under = pos.is_some_and(|p| rect.contains(p));
        // A press starts a stroke: the left button sets the opposite of
        // this step, the right button clears.
        if under && (left_pressed || right_pressed) && response.hovered() {
            let paint = left_pressed && !lit;
            state.step_paint = Some(StepStroke {
                channel: index,
                on: paint,
                steps: Vec::new(),
            });
            if paint {
                actions.push(Action::PreviewShort {
                    channel: index,
                    key: 60,
                });
            }
        }
        let mut shown = *lit;
        if let Some(stroke) = state.step_paint.as_mut().filter(|s| s.channel == index) {
            if under && down && !stroke.steps.contains(&step) {
                stroke.steps.push(step);
            }
            if stroke.steps.contains(&step) {
                shown = stroke.on;
            }
        }
        let base = if (step / beat_steps) % 2 == 0 {
            theme::STEP_A
        } else {
            theme::STEP_B
        };
        let fill = if shown { theme::palette(index) } else { base };
        let fill = if response.hovered() && !shown {
            theme::RAISED
        } else {
            fill
        };
        ui.painter().rect_filled(rect.shrink(1.0), 3.0, fill);
    }
}

/// Wraps several invocations into one undo step.
fn batch(invocations: Vec<Invocation>) -> Option<Invocation> {
    match invocations.len() {
        0 => None,
        1 => invocations.into_iter().next(),
        _ => Some(Invocation::new(
            musiccraft_commands::BATCH,
            invocations.iter().map(ToString::to_string),
        )),
    }
}

/// A drag across a channel's step buttons.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StepStroke {
    /// The channel row.
    pub channel: usize,
    /// Whether the steps passed turn on (left button from an unlit step) or
    /// off.
    pub on: bool,
    /// The steps passed so far.
    pub steps: Vec<usize>,
}

/// A miniature piano roll in place of the step buttons, for melodic channels.
fn mini_roll(
    ui: &mut egui::Ui,
    state: &mut UiState,
    (index, beat_steps): (usize, u32),
    notes: &[&musiccraft_project::Note],
    steps: u32,
) {
    let size = vec2(STEP_SIZE.x * steps as f32, STEP_SIZE.y);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect.shrink(1.0), 3.0, theme::PANEL_DARK);
    let beat = beat_steps as f32 * STEP_SIZE.x;
    for k in 0..steps.div_ceil(beat_steps) {
        let x = rect.left() + k as f32 * beat;
        if k % 2 == 1 {
            let band = egui::Rect::from_min_max(
                egui::pos2(x, rect.top() + 1.0),
                egui::pos2((x + beat).min(rect.right()), rect.bottom() - 1.0),
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
        .hint("Melodic pattern: click to open the piano roll")
        .clicked()
    {
        state.selected_channel = index;
        state.windows.piano_roll = true;
    }
}

fn add_menu(ui: &mut egui::Ui, plugins: &crate::Plugins, actions: &mut Vec<Action>) {
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
            ui.menu_button("Drum synth", |ui| {
                for kind in musiccraft_project::params::DRUM_TYPES {
                    if ui.button(kind).clicked() {
                        actions.push(Action::Run(Invocation::new("channel.add_drum", [kind])));
                        ui.close();
                    }
                }
            });
            if ui.button("Pluck").clicked() {
                actions.push(Action::Run(Invocation::bare("channel.add_pluck")));
                ui.close();
            }
            ui.menu_button("More instruments", |ui| {
                for (label, line) in [
                    ("FM synth", "channel.add_fm"),
                    ("Piano", "channel.add_keys piano"),
                    ("Electric piano", "channel.add_keys epiano"),
                    ("Organ", "channel.add_keys organ"),
                    ("Additive", "channel.add_additive"),
                    ("Drum kit (16 pads)", "channel.add_kit"),
                    ("Zone sampler", "channel.add_zones"),
                    ("Layer", "channel.add_layer"),
                    ("MIDI Out", "channel.add_midi_out"),
                ] {
                    if ui.button(label).clicked() {
                        if let Ok(invocation) = Invocation::parse(line) {
                            actions.push(Action::Run(invocation));
                        }
                        ui.close();
                    }
                }
            });
            ui.menu_button("Plugin instrument", |ui| {
                crate::plugins::add_menu(ui, plugins, true, actions, |p| {
                    Invocation::new("channel.add_plugin", [p.id.clone(), p.name.clone()])
                });
            });
        });
    });
}

/// Keys while the pointer is over the rack: 1–0 mute channels 1–10 (Ctrl:
/// solo), and for the selected channel Alt+C clone, Alt+Delete delete,
/// Alt+Up/Down move, Ctrl+L route to a free mixer track and
/// Shift+Ctrl+Left/Right shift its steps.
fn keys(ctx: &egui::Context, state: &mut UiState, project: &Project, actions: &mut Vec<Action>) {
    use egui::{Key, Modifiers};
    let pressed = |modifiers: Modifiers, key: Key| ctx.input_mut(|i| i.consume_key(modifiers, key));
    let digits = [
        Key::Num1,
        Key::Num2,
        Key::Num3,
        Key::Num4,
        Key::Num5,
        Key::Num6,
        Key::Num7,
        Key::Num8,
        Key::Num9,
        Key::Num0,
    ];
    for (i, key) in digits.into_iter().enumerate() {
        let number = (i + 1).to_string();
        if i >= project.channels.len() {
            continue;
        }
        if pressed(Modifiers::COMMAND, key) {
            run(actions, "channel.solo", [number]);
        } else if pressed(Modifiers::NONE, key) {
            run(actions, "channel.toggle_mute", [number]);
        }
    }
    let selected = state.selected_channel;
    if selected >= project.channels.len() {
        return;
    }
    let number = (selected + 1).to_string();
    if pressed(Modifiers::ALT, Key::C) {
        run(actions, "channel.clone", [number.clone()]);
    }
    if pressed(Modifiers::ALT, Key::Delete) {
        run(actions, "channel.remove", [number.clone()]);
    }
    if pressed(Modifiers::ALT, Key::ArrowUp) && selected > 0 {
        run(
            actions,
            "channel.move",
            [number.clone(), selected.to_string()],
        );
        state.selected_channel -= 1;
    }
    if pressed(Modifiers::ALT, Key::ArrowDown) && selected + 1 < project.channels.len() {
        run(
            actions,
            "channel.move",
            [number.clone(), (selected + 2).to_string()],
        );
        state.selected_channel += 1;
    }
    if pressed(Modifiers::COMMAND, Key::L) {
        run(actions, "channel.route_free", [number.clone()]);
    }
    let shift_ctrl = Modifiers::COMMAND | Modifiers::SHIFT;
    if pressed(shift_ctrl, Key::ArrowLeft) {
        run(actions, "step.shift", [number.clone(), "-1".to_owned()]);
    }
    if pressed(shift_ctrl, Key::ArrowRight) {
        run(actions, "step.shift", [number, "1".to_owned()]);
    }
}

/// The step graph editor: one bar per step of the selected channel, showing
/// the property of the note that starts on that step. Dragging a bar sets it;
/// one drag is one undo step.
fn graph_editor(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    use crate::piano_roll::LaneMode;
    let Some(mode) = state.graph else {
        return;
    };
    let channel = state.selected_channel;
    let Some(id) = project.channels.get(channel).map(|c| c.id) else {
        ui.label(RichText::new("Select a channel").color(theme::TEXT_DIM));
        return;
    };
    let pattern = project.current_pattern();
    let steps = (pattern.length_ticks() / TICKS_PER_STEP).max(1);
    let beat_steps = (pattern.time_signature.beat_ticks() / TICKS_PER_STEP).max(1);
    ui.horizontal(|ui| {
        ui.label(RichText::new(&project.channels[channel].name).strong());
        for m in LaneMode::ALL {
            if ui.selectable_label(mode == m, m.label()).clicked() {
                state.graph = Some(m);
            }
        }
    });
    let size = egui::vec2(ui.available_width().max(120.0), 70.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    state.graph_view = Some((rect, steps));
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::PANEL_DARK);
    let width = rect.width() / steps as f32;
    let note_at = |step: u32| {
        pattern
            .notes_of(id)
            .filter(|n| n.start == step * TICKS_PER_STEP)
            .max_by_key(|n| n.key)
            .copied()
    };
    let step_at = |x: f32| ((x - rect.left()) / width).floor().max(0.0) as u32;
    let unit = |y: f32| ((rect.bottom() - 3.0 - y) / (rect.height() - 6.0)).clamp(0.0, 1.0);
    if response.drag_started() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            state.graph_drag = note_at(step_at(pos.x).min(steps - 1));
            state.begin_gesture();
        }
    }
    let mut shown = None;
    if let (Some(note), Some(pos)) = (state.graph_drag, response.interact_pointer_pos()) {
        if response.dragged() || response.clicked() {
            let t = unit(pos.y);
            shown = Some((note.start, t));
            if let Some(invocation) = mode.invocation(channel, &note, t) {
                actions.push(Action::Coalesced(invocation, state.gesture()));
            }
        }
    }
    if response.drag_stopped() || (!response.dragged() && !response.clicked()) {
        state.graph_drag = None;
    }
    let color = theme::palette(channel);
    let base = if mode.centred() {
        rect.center().y
    } else {
        rect.bottom() - 3.0
    };
    for step in 0..steps {
        let x0 = rect.left() + width * step as f32;
        if step % beat_steps == 0 {
            painter.line_segment(
                [egui::pos2(x0, rect.top()), egui::pos2(x0, rect.bottom())],
                egui::Stroke::new(1.0, theme::GRID_BEAT),
            );
        }
        let Some(note) = note_at(step) else {
            continue;
        };
        let t = match shown {
            Some((start, t)) if start == note.start => t,
            _ => mode.value(&note),
        };
        let tip = rect.bottom() - 3.0 - t * (rect.height() - 6.0);
        let bar = egui::Rect::from_two_pos(
            egui::pos2(x0 + 2.0, base),
            egui::pos2(x0 + (width - 2.0).max(3.0), tip),
        );
        painter.rect_filled(bar, 1.0, color);
    }
}

/// The name of the mixer track a channel plays through, for tooltips.
fn route_name(project: &Project, track: usize) -> String {
    match project.mixer.get(track) {
        Some(_) if track == 0 => "Master".to_owned(),
        Some(t) => format!("{track} ({})", t.name),
        None => track.to_string(),
    }
}
