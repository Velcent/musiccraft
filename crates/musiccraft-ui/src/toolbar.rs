//! Menu bar, transport bar and status bar.

use crate::hint::Hint;
use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::{MAX_TEMPO_BPM, MIN_TEMPO_BPM, PlayMode};

use crate::icons::{self, Icon};
use crate::{Action, Frame, UiState, geometry::position_label, signature, theme, widgets};

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
        "spectrum" => &mut state.windows.spectrum,
        _ => &mut state.windows.shortcuts,
    };
    ui.checkbox(open, label);
}

fn settings_item(
    ui: &mut egui::Ui,
    state: &UiState,
    label: &str,
    tab: &str,
    actions: &mut Vec<Action>,
) {
    let invocation = format!("ui.settings {tab}");
    let shortcut = if tab == "audio" {
        shortcut(state, "ui.settings")
    } else {
        String::new()
    };
    if ui
        .add(egui::Button::new(label).shortcut_text(shortcut))
        .clicked()
    {
        run(actions, &invocation);
        ui.close();
    }
}

fn file_menu(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<Action>) {
    menu_item(ui, state, "New", "ui.new", actions);
    menu_item(ui, state, "Open…", "ui.open", actions);
    menu_item(ui, state, "Save", "ui.save", actions);
    menu_item(ui, state, "Save as…", "ui.save_as", actions);
    menu_item(
        ui,
        state,
        "Save new version",
        "ui.save_new_version",
        actions,
    );
    ui.separator();
    menu_item(ui, state, "Export audio (WAV, FLAC)…", "ui.export", actions);
    menu_item(ui, state, "Export stems…", "ui.export_stems", actions);
    menu_item(ui, state, "Export MIDI…", "ui.export_midi", actions);
    if let Some(path) = state.autosave_path.clone() {
        if ui
            .button("Recover unsaved work")
            .hint("Restore the project from the session that did not close properly")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new("project.recover", [path])));
            ui.close();
        }
    }
    ui.separator();
    ui.checkbox(&mut state.record_audio, "Record audio input")
        .hint("With REC armed in song mode, capture the audio input as a clip");
    ui.separator();
    if ui.button("Exit").clicked() {
        actions.push(Action::Quit);
    }
}

fn add_menu(ui: &mut egui::Ui, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    ui.menu_button("Channel", |ui| {
        for name in musiccraft_project::params::BUILTIN_SAMPLES {
            if ui.button(name.replace('_', " ")).clicked() {
                actions.push(Action::Run(Invocation::new(
                    "channel.add_sampler",
                    [format!(
                        "{}{name}",
                        musiccraft_project::params::BUILTIN_PREFIX
                    )],
                )));
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Synth").clicked() {
            run(actions, "channel.add_synth");
            ui.close();
        }
        ui.menu_button("Drum synth", |ui| {
            for kind in musiccraft_project::params::DRUM_TYPES {
                if ui.button(kind).clicked() {
                    run(actions, &format!("channel.add_drum {kind}"));
                    ui.close();
                }
            }
        });
        if ui.button("Pluck").clicked() {
            run(actions, "channel.add_pluck");
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
    });
    ui.menu_button("Plugin instrument", |ui| {
        crate::plugins::add_menu(ui, frame.plugins, true, actions, |p| {
            Invocation::new("channel.add_plugin", [p.id.clone(), p.name.clone()])
        });
    });
    ui.separator();
    if ui.button("Pattern").clicked() {
        run(actions, "pattern.add");
        ui.close();
    }
    if ui.button("Mixer insert").clicked() {
        run(actions, "mixer.add_track");
        ui.close();
    }
}

fn patterns_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    let number = project.current_pattern + 1;
    menu_item(ui, state, "New pattern", "pattern.add", actions);
    if ui.button("Clone pattern").clicked() {
        run(actions, &format!("pattern.clone {number}"));
        ui.close();
    }
    if ui.button("Rename pattern…").clicked() {
        state.open_dialog(crate::dialogs::Dialog::rename_pattern(
            project.current_pattern,
            &project.current_pattern().name,
        ));
        ui.close();
    }
    ui.add_enabled_ui(project.patterns.len() > 1, |ui| {
        if ui.button("Delete pattern").clicked() {
            run(actions, &format!("pattern.remove {number}"));
            ui.close();
        }
    });
    ui.separator();
    menu_item(ui, state, "Next pattern", "pattern.next", actions);
    menu_item(ui, state, "Previous pattern", "pattern.prev", actions);
    ui.separator();
    for (i, pattern) in project.patterns.iter().enumerate() {
        let text = RichText::new(format!("{}  {}", i + 1, pattern.name)).color(theme::palette(i));
        if ui
            .selectable_label(i == project.current_pattern, text)
            .clicked()
        {
            run(actions, &format!("pattern.select {}", i + 1));
            ui.close();
        }
    }
}

fn view_menu(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    window_toggle(ui, state, "Playlist (F5)", "playlist");
    window_toggle(ui, state, "Channel rack (F6)", "channel_rack");
    window_toggle(ui, state, "Piano roll (F7)", "piano_roll");
    window_toggle(ui, state, "Browser (Alt+F8)", "browser");
    window_toggle(ui, state, "Mixer (F9)", "mixer");
    window_toggle(ui, state, "Spectrum analyzer", "spectrum");
    ui.separator();
    let mut hints = frame.settings.show_hints;
    if ui.checkbox(&mut hints, "Hint bar").changed() {
        actions.push(Action::Settings(Box::new(crate::Settings {
            show_hints: hints,
            ..frame.settings.clone()
        })));
    }
    window_toggle(ui, state, "Keyboard shortcuts", "shortcuts");
}

fn options_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    settings_item(ui, state, "Audio settings", "audio", actions);
    settings_item(ui, state, "MIDI settings", "midi", actions);
    settings_item(ui, state, "General settings", "general", actions);
    settings_item(ui, state, "Folders and plugins", "folders", actions);
    settings_item(ui, state, "Project info", "project", actions);
    ui.separator();
    let mut metronome = frame.project.metronome;
    if ui.checkbox(&mut metronome, "Metronome").changed() {
        run(actions, "transport.toggle_metronome");
    }
    ui.checkbox(&mut state.typing_keyboard, "Typing keyboard to piano");
}

fn tools_menu(ui: &mut egui::Ui, frame: &Frame<'_>, state: &UiState, actions: &mut Vec<Action>) {
    if ui.button("Rescan plugins").clicked() {
        actions.push(Action::RescanPlugins);
        ui.close();
    }
    if ui.button("Reload samples").clicked() {
        actions.push(Action::ReloadSamples);
        ui.close();
    }
    let channel = state.selected_channel + 1;
    ui.add_enabled_ui(
        state.selected_channel < frame.project.channels.len(),
        |ui| {
            if ui
                .button("Quantize selected channel")
                .hint("Snap the selected channel's notes in this pattern to the step grid")
                .clicked()
            {
                run(actions, &format!("note.quantize {channel} 1"));
                ui.close();
            }
        },
    );
}

fn help_menu(ui: &mut egui::Ui, state: &mut UiState) {
    if ui.button("Keyboard shortcuts").clicked() {
        state.windows.shortcuts = true;
        ui.close();
    }
    if ui.button("About MusicCraft").clicked() {
        state.set_status(
            format!(
                "MusicCraft {} — a clean-room pattern-based DAW written in Rust",
                env!("CARGO_PKG_VERSION")
            ),
            false,
        );
        ui.close();
    }
}

/// The hint bar: what the hovered control does, otherwise the last message.
fn hint_bar(ui: &mut egui::Ui, state: &UiState, width: f32) {
    let (text, color) = match &state.hint {
        Some(hint) => (hint.as_str(), theme::TEXT),
        None if state.status.error => (state.status.text.as_str(), theme::DANGER),
        None => (state.status.text.as_str(), theme::TEXT_DIM),
    };
    egui::Frame::new()
        .fill(theme::LCD_BG)
        .stroke(egui::Stroke::new(1.0, theme::LCD_EDGE))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(6, 2))
        .show(ui, |ui| {
            ui.set_width(width.max(60.0));
            ui.add(egui::Label::new(RichText::new(text).small().color(color)).truncate());
        });
}

pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::top("menu").show(ui, |ui| {
        egui::MenuBar::new().ui(ui, |ui| {
            let menu = |label: &str| RichText::new(label).small().strong();
            ui.menu_button(menu("FILE"), |ui| file_menu(ui, state, actions));
            ui.menu_button(menu("EDIT"), |ui| {
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
            ui.menu_button(menu("ADD"), |ui| add_menu(ui, frame, actions));
            ui.menu_button(menu("PATTERNS"), |ui| {
                patterns_menu(ui, state, frame, actions)
            });
            ui.menu_button(menu("VIEW"), |ui| view_menu(ui, state, frame, actions));
            ui.menu_button(menu("OPTIONS"), |ui| {
                options_menu(ui, state, frame, actions)
            });
            ui.menu_button(menu("TOOLS"), |ui| tools_menu(ui, frame, state, actions));
            ui.menu_button(menu("HELP"), |ui| help_menu(ui, state));
            ui.add_space(6.0);
            let width = ui.available_width() - 150.0;
            if frame.settings.show_hints && width > 80.0 {
                hint_bar(ui, state, width);
            } else if width > 80.0 {
                let color = if state.status.error {
                    theme::DANGER
                } else {
                    theme::TEXT_DIM
                };
                ui.add_sized(
                    [width, 18.0],
                    egui::Label::new(RichText::new(&state.status.text).small().color(color))
                        .truncate(),
                );
            }
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
            "Record live notes into the pattern while playing ({}); right-click for options",
            shortcut(state, "record.toggle")
        );
        let armed = project.is_armed();
        let rec = icons::button(ui, Icon::Record, armed, theme::DANGER, &record_hint);
        if rec.clicked() {
            run(actions, "record.toggle");
        }
        rec.context_menu(|ui| record_options(ui, state, project, actions));
        if frame.engine.counting_in {
            ui.label(RichText::new("COUNT-IN").strong().color(theme::DANGER));
        } else if frame.engine.waiting {
            ui.label(RichText::new("WAITING").strong().color(theme::DANGER))
                .on_hover_text("Playback starts with the first note you play");
        } else if project.step_edit() {
            ui.label(RichText::new("STEP").strong().color(theme::HIGHLIGHT))
                .on_hover_text("Step edit: played notes go in at the playhead");
        }
    });

    let ticks = frame.engine.position_ticks.max(0.0);
    let seconds = ticks / f64::from(musiccraft_project::params::PPQ) * 60.0 / project.tempo_bpm();
    let time = format!("{}:{:05.2}", (seconds / 60.0) as u32, seconds % 60.0);
    icons::lcd(
        ui,
        92.0,
        position_label(ticks, &project.playing_meter()).trim(),
        &time,
        egui::Sense::hover(),
    )
    .hint("Song position: bar . beat . step, and time");

    let tempo = project.tempo_bpm();
    let tempo_lcd = icons::lcd(
        ui,
        76.0,
        &format!("{tempo:.2}"),
        "BPM",
        egui::Sense::click_and_drag(),
    )
    .hint("Tempo: drag up/down or scroll (Shift for fine steps)");
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
        let value = (tempo + f64::from(delta)).clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM);
        actions.push(Action::Coalesced(
            Invocation::new("project.set_tempo", [format!("{value:.3}")]),
            state.gesture(),
        ));
    }
    if ui
        .small_button("TAP")
        .hint("Tap the beat a few times to set the tempo")
        .clicked()
    {
        let now = ui.input(|i| i.time);
        if state.taps.is_empty() || state.taps.last().is_some_and(|t| now - t > 2.0) {
            state.begin_gesture();
        }
        if let Some(bpm) = musiccraft_commands::tap_tempo::tap(&mut state.taps, now) {
            actions.push(Action::Coalesced(
                Invocation::new("project.set_tempo", [format!("{bpm:.2}")]),
                state.gesture(),
            ));
        }
    }
    let sig = project.time_signature;
    ui.menu_button(RichText::new(sig.to_string()).monospace(), |ui| {
        if let Some(new) = signature::picker(ui, Some(sig), &mut state.sig_draft) {
            run(actions, &format!("project.set_time_signature {new}"));
        }
    })
    .response
    .hint(
        "Time signature of the song and of new patterns (changes: right-click the playlist ruler)",
    );
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
        .hint("Selected pattern (Plus / Minus)");
    if ui
        .small_button("+")
        .hint("New pattern (F4: next empty pattern)")
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
        knob.response.hint(format!(
            "Master volume {:.1} dB",
            20.0 * master.volume.max(1e-6).log10()
        ));
    }
    let master = state.meters.first().copied().unwrap_or_default();
    widgets::meter(ui, master, vec2(10.0, 32.0)).hint("Master level");
    if ui.available_width() > 150.0 {
        let recent = frame.engine.scope.len().saturating_sub(256);
        let scope = frame.engine.scope.get(recent..).unwrap_or_default();
        if icons::scope(ui, scope, vec2(96.0, 32.0))
            .hint("Master output; click for the spectrum analyzer")
            .clicked()
        {
            state.windows.spectrum = !state.windows.spectrum;
        }
    }
    if ui.available_width() > 50.0 {
        icons::load_bar(ui, frame.engine.load).hint(format!(
            "Audio load {:.0}% · {} · {}",
            frame.engine.load * 100.0,
            frame.engine.audio,
            frame.engine.midi
        ));
    }
}

/// The REC button's menu: count-in, wait for input, loop, blend/replace and
/// step edit.
fn record_options(
    ui: &mut egui::Ui,
    state: &UiState,
    project: &musiccraft_project::Project,
    actions: &mut Vec<Action>,
) {
    let options = project.record;
    let mut toggle = |ui: &mut egui::Ui, on: bool, label: &str, name: &str| {
        let text = match shortcut(state, name) {
            s if s.is_empty() => label.to_owned(),
            s => format!("{label} ({s})"),
        };
        if ui.selectable_label(on, text).clicked() {
            run(actions, name);
            ui.close();
        }
    };
    toggle(
        ui,
        options.count_in,
        "Count-in (one bar)",
        "record.toggle_count_in",
    );
    toggle(
        ui,
        options.wait_for_input,
        "Wait for input",
        "record.toggle_wait",
    );
    toggle(
        ui,
        options.loop_record,
        "Loop recording",
        "record.toggle_loop",
    );
    toggle(
        ui,
        project.step_edit(),
        "Step edit",
        "record.toggle_step_edit",
    );
    ui.separator();
    for mode in [
        musiccraft_project::RecordMode::Blend,
        musiccraft_project::RecordMode::Replace,
    ] {
        if ui
            .selectable_label(options.mode == mode, mode.label())
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "record.set_mode",
                [mode.label().to_owned()],
            )));
            ui.close();
        }
    }
}
