//! Menu bar, transport bar and status bar.

use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::PlayMode;

use crate::icons::{self, Icon};
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
                ui.checkbox(&mut state.record_audio, "Record audio input")
                    .on_hover_text(
                        "With REC armed in song mode, capture the input as an audio clip",
                    );
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
            window_buttons(ui, state);
        });
    });

    egui::Panel::top("transport")
        .frame(
            egui::Frame::new()
                .fill(theme::PANEL_DARK)
                .inner_margin(egui::Margin::symmetric(8, 6)),
        )
        .show(ui, |ui| {
            ui.horizontal(|ui| transport(ui, state, frame, actions));
        });
}

/// Window buttons at the right end of the menu bar.
fn window_buttons(ui: &mut egui::Ui, state: &mut UiState) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 3.0;
        let w = &mut state.windows;
        for (icon, open, hint) in [
            (Icon::Mixer, &mut w.mixer, "Mixer (F9)"),
            (Icon::PianoRoll, &mut w.piano_roll, "Piano roll (F7)"),
            (Icon::ChannelRack, &mut w.channel_rack, "Channel rack (F6)"),
            (Icon::Playlist, &mut w.playlist, "Playlist (F5)"),
            (Icon::Browser, &mut w.browser, "Browser (Alt+F8)"),
        ] {
            if icons::toggle(ui, icon, *open, hint).clicked() {
                *open = !*open;
            }
        }
    });
}

fn group(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.spacing_mut().item_spacing.x = 3.0;
    add(ui);
    ui.spacing_mut().item_spacing.x = 8.0;
    ui.add_space(6.0);
}

fn transport(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let playing = project.transport().is_playing();

    let mode = usize::from(project.play_mode == PlayMode::Song);
    if let Some(choice) =
        icons::segmented(ui, &["PAT", "SONG"], mode, "Pattern or song playback (L)")
    {
        if choice != mode {
            let label = if choice == 1 { "song" } else { "pattern" };
            run(actions, &format!("transport.set_mode {label}"));
        }
    }
    ui.add_space(6.0);

    group(ui, |ui| {
        if icons::button(
            ui,
            Icon::Play,
            playing,
            theme::ACCENT,
            "Play / stop (Space)",
        )
        .clicked()
        {
            run(actions, "transport.toggle");
        }
        if icons::button(
            ui,
            Icon::Stop,
            false,
            theme::TEXT,
            "Stop and return to the start",
        )
        .clicked()
        {
            run(actions, "transport.stop");
        }
        let record_hint = format!(
            "Record live notes into the pattern while playing ({})",
            shortcut(state, "ui.toggle_record")
        );
        if icons::button(ui, Icon::Record, state.record, theme::DANGER, &record_hint).clicked() {
            state.record = !state.record;
        }
    });

    let ticks = frame.engine.position_ticks.max(0.0);
    let seconds = ticks / f64::from(musiccraft_project::params::PPQ) * 60.0 / project.tempo_bpm();
    let time = format!("{}:{:05.2}", (seconds / 60.0) as u32, seconds % 60.0);
    icons::lcd(
        ui,
        92.0,
        position_label(ticks).trim(),
        &time,
        egui::Sense::hover(),
    )
    .on_hover_text("Song position: bar . beat . step, and time");

    let tempo = project.tempo_bpm();
    let tempo_lcd = icons::lcd(
        ui,
        76.0,
        &format!("{tempo:.2}"),
        "BPM",
        egui::Sense::click_and_drag(),
    )
    .on_hover_text("Tempo: drag up/down or scroll (Shift for fine steps)");
    let fine = ui.input(|i| i.modifiers.shift);
    let scroll = if tempo_lcd.hovered() {
        ui.input(|i| i.smooth_scroll_delta.y)
    } else {
        0.0
    };
    if tempo_lcd.drag_started() || (scroll != 0.0 && !tempo_lcd.dragged()) {
        state.begin_gesture();
    }
    let delta = -tempo_lcd.drag_delta().y * if fine { 0.02 } else { 0.25 }
        + scroll * if fine { 0.01 } else { 0.05 };
    if delta != 0.0 {
        let value = (tempo + f64::from(delta)).clamp(20.0, 999.0);
        actions.push(Action::Coalesced(
            Invocation::new("project.set_tempo", [format!("{value:.3}")]),
            state.gesture(),
        ));
    }
    ui.add_space(4.0);

    let current = project.current_pattern;
    egui::ComboBox::from_id_salt("pattern-select")
        .width(118.0)
        .selected_text(
            RichText::new(format!(
                "{}  {}",
                current + 1,
                project.current_pattern().name
            ))
            .color(theme::palette(current)),
        )
        .show_ui(ui, |ui| {
            for (i, pattern) in project.patterns.iter().enumerate() {
                let text =
                    RichText::new(format!("{}  {}", i + 1, pattern.name)).color(theme::palette(i));
                if ui.selectable_label(i == current, text).clicked() {
                    run(actions, &format!("pattern.select {}", i + 1));
                }
            }
        })
        .response
        .on_hover_text("Selected pattern (Plus / Minus)");
    if ui
        .small_button("+")
        .on_hover_text("New pattern (F4)")
        .clicked()
    {
        run(actions, "pattern.add");
    }
    ui.add_space(4.0);

    group(ui, |ui| {
        let metronome_hint = format!(
            "Metronome ({})",
            shortcut(state, "transport.toggle_metronome")
        );
        if icons::button(
            ui,
            Icon::Metronome,
            project.metronome,
            theme::HIGHLIGHT,
            &metronome_hint,
        )
        .clicked()
        {
            run(actions, "transport.toggle_metronome");
        }
        let typing_hint = format!(
            "Typing keyboard plays notes: Z–M and Q–P rows ({})",
            shortcut(state, "ui.typing_keyboard")
        );
        if icons::button(
            ui,
            Icon::Keyboard,
            state.typing_keyboard,
            theme::ACCENT,
            &typing_hint,
        )
        .clicked()
        {
            state.typing_keyboard = !state.typing_keyboard;
        }
    });

    if let Some(master) = project.mixer.first() {
        let knob = widgets::knob(
            ui,
            master.volume,
            0.0,
            musiccraft_project::MAX_VOLUME,
            1.0,
            false,
        );
        if knob.started {
            state.begin_gesture();
        }
        if let Some(value) = knob.changed {
            actions.push(Action::Coalesced(
                Invocation::new("mixer.set_volume", ["0".to_owned(), format!("{value:.3}")]),
                state.gesture(),
            ));
        }
        knob.response.on_hover_text(format!(
            "Master volume {:.1} dB",
            20.0 * master.volume.max(1e-6).log10()
        ));
    }
    let master = state.meters.first().copied().unwrap_or_default();
    widgets::meter(ui, master, vec2(10.0, 32.0)).on_hover_text("Master level");
    if ui.available_width() > 150.0 {
        icons::scope(ui, &frame.engine.scope, vec2(96.0, 32.0)).on_hover_text("Master output");
    }
    if ui.available_width() > 50.0 {
        icons::load_bar(ui, frame.engine.load);
    }
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
