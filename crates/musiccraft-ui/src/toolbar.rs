//! Menu bar, transport bar and status bar.

use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::PlayMode;

use crate::{Action, Frame, UiState, geometry::position_label, theme, widgets};

fn run(actions: &mut Vec<Action>, line: &str) {
    if let Ok(invocation) = Invocation::parse(line) {
        actions.push(Action::Run(invocation));
    }
}

fn shortcut(state: &UiState, invocation: &str) -> String {
    state
        .keymap
        .chord_for(invocation)
        .map(ToString::to_string)
        .unwrap_or_default()
}

fn menu_item(
    ui: &mut egui::Ui,
    state: &UiState,
    label: &str,
    invocation: &str,
    actions: &mut Vec<Action>,
) {
    let button = egui::Button::new(label).shortcut_text(shortcut(state, invocation));
    if ui.add(button).clicked() {
        run(actions, invocation);
        ui.close();
    }
}

fn window_toggle(ui: &mut egui::Ui, state: &mut UiState, label: &str, which: &str) {
    let open = match which {
        "playlist" => &mut state.windows.playlist,
        "channel_rack" => &mut state.windows.channel_rack,
        "piano_roll" => &mut state.windows.piano_roll,
        "mixer" => &mut state.windows.mixer,
        "browser" => &mut state.windows.browser,
        _ => &mut state.windows.shortcuts,
    };
    ui.checkbox(open, label);
}

pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                menu_item(ui, state, "New", "ui.new", actions);
                menu_item(ui, state, "Open…", "ui.open", actions);
                menu_item(ui, state, "Save", "ui.save", actions);
                menu_item(ui, state, "Save as…", "ui.save_as", actions);
                ui.separator();
                menu_item(ui, state, "Export WAV…", "ui.export", actions);
                if let Some(path) = state.autosave_path.clone() {
                    if ui.button("Recover autosave").clicked() {
                        actions.push(Action::Run(Invocation::new("project.load", [path])));
                        ui.close();
                    }
                }
                if ui.button("Reload samples").clicked() {
                    actions.push(Action::ReloadSamples);
                    ui.close();
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                    actions.push(Action::Quit);
                }
            });
            ui.menu_button("Edit", |ui| {
                let undo = match &frame.history.undo {
                    Some(label) => format!("Undo {label}"),
                    None => "Undo".to_owned(),
                };
                ui.add_enabled_ui(frame.history.undo.is_some(), |ui| {
                    menu_item(ui, state, &undo, "edit.undo", actions);
                });
                ui.add_enabled_ui(frame.history.can_redo, |ui| {
                    menu_item(ui, state, "Redo", "edit.redo", actions);
                });
            });
            ui.menu_button("View", |ui| {
                window_toggle(ui, state, "Channel rack (F6)", "channel_rack");
                window_toggle(ui, state, "Piano roll (F7)", "piano_roll");
                window_toggle(ui, state, "Playlist (F5)", "playlist");
                window_toggle(ui, state, "Mixer (F9)", "mixer");
                window_toggle(ui, state, "Browser (Alt+F8)", "browser");
                ui.separator();
                window_toggle(ui, state, "Keyboard shortcuts", "shortcuts");
            });
        });
    });

    egui::Panel::top("transport")
        .frame(egui::Frame::new().fill(theme::PANEL_DARK).inner_margin(6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| transport(ui, state, frame, actions));
        });
}

fn transport(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let playing = project.transport().is_playing();
    let (label, color) = if playing {
        ("■", theme::HIGHLIGHT)
    } else {
        ("▶", theme::ACCENT)
    };
    let play =
        egui::Button::new(RichText::new(label).size(18.0).color(color)).min_size(vec2(36.0, 28.0));
    if ui.add(play).on_hover_text("Play / stop (Space)").clicked() {
        run(actions, "transport.toggle");
    }
    ui.add_space(6.0);
    for (mode, text) in [(PlayMode::Pattern, "PAT"), (PlayMode::Song, "SONG")] {
        let selected = project.play_mode == mode;
        if ui
            .selectable_label(selected, RichText::new(text).strong())
            .on_hover_text("Pattern or song playback (L)")
            .clicked()
            && !selected
        {
            run(actions, &format!("transport.set_mode {}", mode.label()));
        }
    }
    let record = egui::Button::new(
        RichText::new("REC")
            .small()
            .strong()
            .color(if state.record {
                theme::BACKGROUND
            } else {
                theme::DANGER
            }),
    )
    .fill(if state.record {
        theme::DANGER
    } else {
        theme::RAISED
    });
    if ui
        .add(record)
        .on_hover_text(format!(
            "Record live notes into the pattern while playing ({})",
            shortcut(state, "ui.toggle_record")
        ))
        .clicked()
    {
        state.record = !state.record;
    }
    ui.add_space(6.0);
    let metronome = egui::Button::new(RichText::new("♩").size(16.0).color(if project.metronome {
        theme::BACKGROUND
    } else {
        theme::TEXT_DIM
    }))
    .fill(if project.metronome {
        theme::HIGHLIGHT
    } else {
        theme::RAISED
    });
    if ui
        .add(metronome)
        .on_hover_text(format!(
            "Metronome ({})",
            shortcut(state, "transport.toggle_metronome")
        ))
        .clicked()
    {
        run(actions, "transport.toggle_metronome");
    }
    let typing = egui::Button::new(
        RichText::new("⌨")
            .size(15.0)
            .color(if state.typing_keyboard {
                theme::BACKGROUND
            } else {
                theme::TEXT_DIM
            }),
    )
    .fill(if state.typing_keyboard {
        theme::ACCENT
    } else {
        theme::RAISED
    });
    if ui
        .add(typing)
        .on_hover_text(format!(
            "Typing keyboard plays notes: Z–M and Q–P rows ({})",
            shortcut(state, "ui.typing_keyboard")
        ))
        .clicked()
    {
        state.typing_keyboard = !state.typing_keyboard;
    }
    ui.add_space(10.0);

    let mut tempo = project.tempo_bpm();
    let response = ui.add(
        egui::DragValue::new(&mut tempo)
            .range(20.0..=999.0)
            .speed(0.25)
            .fixed_decimals(1)
            .suffix(" BPM"),
    );
    if response.drag_started() || response.gained_focus() {
        state.begin_gesture();
    }
    if response.changed() {
        actions.push(Action::Coalesced(
            Invocation::new("project.set_tempo", [format!("{tempo:.3}")]),
            state.gesture(),
        ));
    }
    ui.add_space(10.0);

    let current = project.current_pattern;
    egui::ComboBox::from_id_salt("pattern-select")
        .width(140.0)
        .selected_text(format!(
            "{}  {}",
            current + 1,
            project.current_pattern().name
        ))
        .show_ui(ui, |ui| {
            for (i, pattern) in project.patterns.iter().enumerate() {
                if ui
                    .selectable_label(i == current, format!("{}  {}", i + 1, pattern.name))
                    .clicked()
                {
                    run(actions, &format!("pattern.select {}", i + 1));
                }
            }
        });
    if ui.button("+").on_hover_text("New pattern (F4)").clicked() {
        run(actions, "pattern.add");
    }
    ui.add_space(10.0);

    let position = if frame.engine.playing {
        frame.engine.position_ticks
    } else {
        0.0
    };
    ui.label(
        RichText::new(position_label(position))
            .monospace()
            .size(16.0)
            .color(theme::PLAYHEAD),
    )
    .on_hover_text("Bar . beat . step");
    ui.add_space(10.0);
    let master = state.meters.first().copied().unwrap_or_default();
    widgets::meter(ui, master, vec2(10.0, 26.0)).on_hover_text("Master level");
    ui.label(
        RichText::new(format!("{} voices", frame.engine.voices))
            .small()
            .color(theme::TEXT_DIM),
    );
}

pub fn status_bar(ui: &mut egui::Ui, state: &UiState, frame: &Frame<'_>) {
    egui::Panel::bottom("status")
        .frame(egui::Frame::new().fill(theme::PANEL_DARK).inner_margin(4.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(&frame.engine.midi)
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(&frame.engine.audio)
                            .small()
                            .color(theme::TEXT_DIM),
                    );
                    ui.separator();
                    let file = frame
                        .project
                        .path()
                        .and_then(|p| p.file_name())
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "unsaved".to_owned());
                    ui.label(RichText::new(format!("{} · {file}", frame.project.name())).small())
                        .on_hover_text(
                            frame
                                .project
                                .path()
                                .map(|p| p.display().to_string())
                                .unwrap_or_default(),
                        );
                    ui.separator();
                    let color = if state.status.error {
                        theme::DANGER
                    } else {
                        theme::TEXT_DIM
                    };
                    // Takes the remaining space on the left and truncates.
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add(
                            egui::Label::new(RichText::new(&state.status.text).color(color))
                                .truncate(),
                        );
                    });
                });
            });
        });
}
