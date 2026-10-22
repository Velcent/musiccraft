//! Menu bar, transport bar and status bar.

use crate::hint::Hint;
use egui::{RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::{MAX_TEMPO_BPM, MIN_TEMPO_BPM, PlayMode};

use crate::icons::{self, Icon};
use crate::{Action, Frame, UiState, format, geometry::position_label, signature, theme, widgets};

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
                ("Channel stack", "channel.add_layer"),
                ("MIDI output", "channel.add_midi_out"),
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
        let text = RichText::new(format!("{}  {}", i + 1, pattern.name))
            .color(crate::pattern_text(pattern));
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
    window_toggle(ui, state, "Spectrum analyser", "spectrum");
    ui.separator();
    let mut hints = frame.settings.show_hints;
    if ui.checkbox(&mut hints, "Show hint bar").changed() {
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
    ui.checkbox(
        &mut state.typing_keyboard,
        "Play notes from the computer keyboard",
    );
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

/// The hint field: the project name over what the hovered control does,
/// otherwise the last message.
fn hint_field(ui: &mut egui::Ui, state: &UiState, frame: &Frame<'_>, width: f32) {
    let hint = if frame.settings.show_hints {
        state.hint.as_deref()
    } else {
        None
    };
    let (text, color) = match hint {
        Some(hint) => (hint, theme::TEXT),
        None if state.status.error => (state.status.text.as_str(), theme::DANGER),
        None => (state.status.text.as_str(), theme::TEXT_DIM),
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width.max(60.0), icons::ROW), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 3.0, theme::PANEL_DARK);
    painter.rect_stroke(
        rect,
        3.0,
        egui::Stroke::new(1.0, theme::LCD_EDGE),
        egui::StrokeKind::Inside,
    );
    let font = egui::FontId::proportional(theme::FONT_CAPTION);
    let line = |text: &str, color, y| {
        let mut job =
            egui::text::LayoutJob::simple_singleline(text.to_owned(), font.clone(), color);
        job.wrap = egui::text::TextWrapping::truncate_at_width(rect.width() - 10.0);
        let galley = painter.layout_job(job);
        painter.galley(egui::pos2(rect.left() + 5.0, y), galley, color);
    };
    line(frame.project.name(), theme::TEXT_DIM, rect.top() + 1.0);
    line(text, color, rect.center().y);
}

/// Width the one-band toolbar needs: the menu column and both racks.
const ONE_BAND_WIDTH: f32 = 1000.0;
/// Width of the menu and hint column.
const MENU_COLUMN: f32 = 300.0;

pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    if ui.available_width() >= ONE_BAND_WIDTH {
        one_band(ui, state, frame, actions);
    } else {
        two_bands(ui, state, frame, actions);
    }
}

fn band_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::PANEL)
        .inner_margin(egui::Margin::symmetric(6, 3))
}

/// The menu and hint column at the left, then the upper and lower racks
/// (see `docs/spec/window-layout.md`, "Toolbar").
fn one_band(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::top("toolbar")
        .frame(band_frame())
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(MENU_COLUMN);
                    ui.spacing_mut().item_spacing.y = 2.0;
                    menu_bar(ui, state, frame, actions);
                    hint_field(ui, state, frame, MENU_COLUMN);
                });
                ui.add_space(8.0);
                racks(ui, state, frame, actions, true);
            });
        });
}

/// The narrow fallback: menus, hint field and window buttons, then the racks.
fn two_bands(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::top("menu").frame(band_frame()).show(ui, |ui| {
        ui.horizontal(|ui| {
            menu_bar(ui, state, frame, actions);
            ui.add_space(6.0);
            let width = ui.available_width() - 160.0;
            if width > 80.0 {
                hint_field(ui, state, frame, width);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                window_buttons(ui, state, actions);
            });
        });
    });
    egui::Panel::top("transport")
        .frame(band_frame())
        .show(ui, |ui| racks(ui, state, frame, actions, false));
}

/// The menu titles in their recessed field.
fn menu_bar(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Frame::new()
        .fill(theme::PANEL_DARK)
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(2, 0))
        .show(ui, |ui| {
            ui.set_min_height(icons::ROW - 2.0);
            egui::MenuBar::new().ui(ui, |ui| menus(ui, state, frame, actions));
        });
}

/// The upper rack (option toggles and displays) over the lower rack
/// (transport, tempo, pattern and window buttons).
fn racks(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
    with_windows: bool,
) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        ui.horizontal(|ui| upper_rack(ui, state, frame, actions));
        ui.horizontal(|ui| {
            lower_rack(ui, state, frame, actions);
            if with_windows {
                divider(ui);
                window_buttons(ui, state, actions);
            }
        });
    });
}

fn menus(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let menu = |label: &str| RichText::new(label).size(theme::FONT_LABEL);
    ui.menu_button(menu("File"), |ui| file_menu(ui, state, actions));
    ui.menu_button(menu("Edit"), |ui| {
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
    ui.menu_button(menu("Add"), |ui| add_menu(ui, frame, actions));
    ui.menu_button(menu("Patterns"), |ui| {
        patterns_menu(ui, state, frame, actions)
    });
    ui.menu_button(menu("View"), |ui| view_menu(ui, state, frame, actions));
    ui.menu_button(menu("Options"), |ui| {
        options_menu(ui, state, frame, actions)
    });
    ui.menu_button(menu("Tools"), |ui| tools_menu(ui, frame, state, actions));
    ui.menu_button(menu("Help"), |ui| help_menu(ui, state));
}

/// The window buttons: playlist, channel rack, piano roll, browser, mixer.
fn window_buttons(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<Action>) {
    ui.spacing_mut().item_spacing.x = 1.0;
    let dragging = egui::DragAndDrop::has_payload_of_type::<crate::browser::Dragged>(ui.ctx());
    let w = &mut state.windows;
    for (icon, open, hint, which, window) in [
        (
            Icon::Playlist,
            &mut w.playlist,
            "Playlist (F5)",
            "playlist",
            "playlist-window",
        ),
        (
            Icon::ChannelRack,
            &mut w.channel_rack,
            "Channel rack (F6)",
            "channel_rack",
            "channel-rack",
        ),
        (
            Icon::PianoRoll,
            &mut w.piano_roll,
            "Piano roll (F7)",
            "piano_roll",
            "piano-roll",
        ),
        (
            Icon::Browser,
            &mut w.browser,
            "Browser (Alt+F8)",
            "browser",
            "",
        ),
        (
            Icon::Mixer,
            &mut w.mixer,
            "Mixer (F9)",
            "mixer",
            "mixer-window",
        ),
    ] {
        let response = icons::toggle(ui, icon, *open, hint);
        if response.clicked() {
            run(actions, &format!("view.toggle {which}"));
        }
        // A browser item dragged over the button opens and raises its window,
        // so the drag can go on into it.
        if dragging && !window.is_empty() && response.contains_pointer() {
            *open = true;
            raise(ui.ctx(), egui::Id::new(window));
        }
    }
}

/// Puts a workspace window on top of the others.
fn raise(ctx: &egui::Context, id: egui::Id) {
    ctx.move_to_top(egui::LayerId::new(egui::Order::Middle, id));
}

/// A thin vertical line between groups.
fn divider(ui: &mut egui::Ui) {
    ui.add_space(3.0);
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, icons::ROW - 4.0), egui::Sense::hover());
    ui.painter().rect_filled(rect, 0.0, theme::SEPARATOR);
    ui.add_space(3.0);
}

/// Buttons in a group nearly touch.
fn group(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui.spacing_mut().item_spacing.x = 1.0;
    add(ui);
    ui.spacing_mut().item_spacing.x = 4.0;
}

/// A latching option toggle: an accent fill with a dark glyph while on.
fn option_toggle(
    ui: &mut egui::Ui,
    state: &UiState,
    (icon, on, name): (Icon, bool, &str),
    invocation: &str,
) -> bool {
    let key = shortcut(state, invocation);
    let word = if on { "on" } else { "off" };
    let hint = if key.is_empty() {
        format!("{name}: {word}")
    } else {
        format!("{name}: {word} ({key})")
    };
    icons::button(ui, icon, on, theme::ACCENT, &hint).clicked()
}

/// Option toggles, the time display, the output monitor and the performance
/// panel.
fn upper_rack(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    let record = project.record;
    group(ui, |ui| {
        for (icon, on, name, invocation) in [
            (
                Icon::Metronome,
                project.metronome,
                "Metronome",
                "transport.toggle_metronome",
            ),
            (
                Icon::Wait,
                record.wait_for_input,
                "Wait for input",
                "record.toggle_wait",
            ),
            (
                Icon::CountIn,
                record.count_in,
                "Count-in",
                "record.toggle_count_in",
            ),
            (
                Icon::Blend,
                record.mode == musiccraft_project::RecordMode::Blend,
                "Blend recording (off: replace)",
                "",
            ),
            (
                Icon::LoopRecord,
                record.loop_record,
                "Loop recording",
                "record.toggle_loop",
            ),
            (
                Icon::Keyboard,
                state.typing_keyboard,
                "Computer keyboard as piano",
                "ui.typing_keyboard",
            ),
            (
                Icon::StepEdit,
                project.step_edit(),
                "Step edit",
                "record.toggle_step_edit",
            ),
        ] {
            if !option_toggle(ui, state, (icon, on, name), invocation) {
                continue;
            }
            match icon {
                Icon::Keyboard => state.typing_keyboard = !state.typing_keyboard,
                Icon::Blend => {
                    let mode = if on { "replace" } else { "blend" };
                    run(actions, &format!("record.set_mode {mode}"));
                }
                _ => run(actions, invocation),
            }
        }
    });
    divider(ui);
    time_display(ui, state, frame);
    divider(ui);
    let recent = frame.engine.scope.len().saturating_sub(256);
    let scope = frame.engine.scope.get(recent..).unwrap_or_default();
    let levels = state.meters.first().copied().unwrap_or_default();
    if icons::output_monitor(ui, scope, levels, 124.0)
        .hint("Master output · click for the spectrum analyser")
        .clicked()
    {
        state.windows.spectrum = !state.windows.spectrum;
    }
    ui.add_space(2.0);
    icons::performance(ui, frame.engine.load, frame.engine.voices).hint(format!(
        "Audio load {:.0}% · {} voices · {} · {}",
        frame.engine.load * 100.0,
        frame.engine.voices,
        frame.engine.audio,
        frame.engine.midi
    ));
}

/// The song position: bar.beat.step (last field smaller) or minutes and
/// seconds; COUNT-IN or WAITING while recording waits to start.
fn time_display(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>) {
    use crate::TimeMode;
    let project = frame.project;
    let ticks = frame.engine.position_ticks.max(0.0);
    let seconds = ticks / f64::from(musiccraft_project::params::PPQ) * 60.0 / project.tempo_bpm();
    let time = format!("{}:{:05.2}", (seconds / 60.0) as u32, seconds % 60.0);
    let meter = project.playing_meter();
    let bars = position_label(ticks, &meter);
    let mode = state.time_mode;
    let (text, caption, sep) = match mode {
        TimeMode::Bars => (bars.clone(), "BARS", '.'),
        TimeMode::Time => (time.clone(), "TIME", '.'),
        TimeMode::Ticks => (crate::geometry::tick_label(ticks, &meter), "TICKS", '.'),
    };
    // The least-significant field, drawn smaller.
    let (digits, tail, caption) = if frame.engine.counting_in {
        ("COUNT-IN".to_owned(), String::new(), "REC")
    } else if frame.engine.waiting {
        ("WAITING".to_owned(), String::new(), "REC")
    } else {
        let (head, last) = text.rsplit_once(sep).unwrap_or((text.as_str(), ""));
        (head.to_owned(), format!("{sep}{last}"), caption)
    };
    let other = if mode == TimeMode::Time { &bars } else { &time };
    let position = icons::display(ui, 100.0, (&digits, &tail), caption, egui::Sense::click()).hint(
        format!("Song position ({other}) · click to switch bars and time · right-click for modes"),
    );
    if position.clicked() {
        state.time_mode = if mode == TimeMode::Time {
            TimeMode::Bars
        } else {
            TimeMode::Time
        };
    }
    position.context_menu(|ui| {
        for (choice, label) in [
            (TimeMode::Bars, "Bars, beats and steps"),
            (TimeMode::Time, "Minutes and seconds"),
            (TimeMode::Ticks, "Bars, beats and ticks"),
        ] {
            if ui.radio(state.time_mode == choice, label).clicked() {
                state.time_mode = choice;
                ui.close();
            }
        }
    });
}

/// The master knob, the mode switch and transport, tempo, signature and the
/// pattern panel.
fn lower_rack(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    let playing = project.transport().is_playing();
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
        knob.response
            .hint(format!("Master volume: {}", format::level(master.volume)));
    }
    divider(ui);
    group(ui, |ui| {
        let song = project.play_mode == PlayMode::Song;
        let (switch, over) = icons::mode_switch(
            ui,
            song,
            "Pattern or song playback (L) · right-click a half to open the channel rack or playlist",
        );
        if switch.clicked() {
            let label = if song { "pattern" } else { "song" };
            run(actions, &format!("transport.set_mode {label}"));
        }
        if switch.secondary_clicked() {
            match over {
                Some(0) => state.windows.channel_rack = true,
                Some(_) => state.windows.playlist = true,
                None => {}
            }
        }
        ui.add_space(3.0);
        let (icon, hint) = if playing {
            (Icon::Pause, "Stop (Space)")
        } else {
            (Icon::Play, "Play (Space)")
        };
        if icons::button(ui, icon, playing, theme::ACCENT, hint).clicked() {
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
        ui.add_space(3.0);
        let record_hint = format!(
            "Record ({}) · right-click for options",
            shortcut(state, "record.toggle")
        );
        let armed = project.is_armed();
        let taking = armed && (playing || frame.engine.counting_in || frame.engine.waiting);
        let rec = icons::button(
            ui,
            Icon::Record,
            armed,
            record_tone(ui, taking),
            &record_hint,
        );
        if rec.clicked() {
            run(actions, "record.toggle");
        }
        rec.context_menu(|ui| record_options(ui, state, project, actions));
    });
    divider(ui);
    tempo(ui, state, project, actions);
    group(ui, |ui| {
        ui.add_space(2.0);
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
    });
    divider(ui);
    pattern_panel(ui, project, actions);
}

/// Record's fill: the danger tone, pulsing while a take runs.
fn record_tone(ui: &egui::Ui, taking: bool) -> egui::Color32 {
    if !taking {
        return theme::DANGER;
    }
    ui.ctx().request_repaint();
    let t = ui.input(|i| i.time);
    let strength = 0.6 + 0.4 * (t * std::f64::consts::PI).sin().abs() as f32;
    theme::mix(theme::PANEL, theme::DANGER, strength)
}

/// Round tempos offered on the tempo display's menu.
const ROUND_TEMPOS: [f64; 5] = [60.0, 90.0, 120.0, 140.0, 174.0];

/// The tempo a drag on the display reaches: `distance` points upwards from
/// where it started at `start` BPM, on the whole digits or the decimals.
fn dragged_tempo(start: f64, on_whole: bool, distance: f32, fine: bool) -> f64 {
    let distance = f64::from(distance);
    let change = match (on_whole, fine) {
        (true, false) => (distance / 6.0).trunc(),
        (true, true) => (distance / 6.0).trunc() * 0.1,
        (false, false) => distance * 0.01,
        (false, true) => distance * 0.001,
    };
    let value = start + change;
    // Whole-digit drags keep the decimals; others land on the display's grid.
    let value = if on_whole {
        value
    } else {
        (value * 1000.0).round() / 1000.0
    };
    value.clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM)
}

/// The tempo display: whole BPM large, decimals small. Drag either part,
/// scroll, double-click to type, right-click for the menu.
fn tempo(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &musiccraft_project::Project,
    actions: &mut Vec<Action>,
) {
    let tempo = project.tempo_bpm();
    let text = format!("{tempo:.3}");
    let (whole, decimals) = text.split_once('.').unwrap_or((text.as_str(), "000"));
    let id = egui::Id::new("tempo-display");
    let typing = ui.data(|d| d.get_temp::<String>(id.with("typing")));
    if let Some(typed) = typing {
        tempo_entry(ui, state, id, typed, actions);
        return;
    }
    let (lcd, split) = icons::display_split(
        ui,
        74.0,
        (whole, &format!(".{decimals}")),
        "BPM",
        egui::Sense::click_and_drag(),
    );
    let lcd = lcd.hint(format!(
        "Tempo {tempo:.3} BPM · drag the whole digits or the decimals (Shift: fine) · double-click to type"
    ));
    let set = |actions: &mut Vec<Action>, state: &UiState, value: f64| {
        actions.push(Action::Coalesced(
            Invocation::new("project.set_tempo", [format!("{value:.3}")]),
            state.gesture(),
        ));
    };
    let fine = ui.input(|i| i.modifiers.shift);
    if lcd.drag_started() {
        state.begin_gesture();
        let on_whole = ui
            .input(|i| i.pointer.press_origin())
            .is_some_and(|p| p.x < split);
        ui.data_mut(|d| d.insert_temp(id.with("drag"), (tempo, on_whole)));
    }
    if lcd.dragged() {
        let start = ui.data(|d| d.get_temp::<(f64, bool)>(id.with("drag")));
        let origin = ui.input(|i| i.pointer.press_origin());
        if let (Some((start, on_whole)), Some(origin), Some(now)) =
            (start, origin, lcd.interact_pointer_pos())
        {
            let value = dragged_tempo(start, on_whole, origin.y - now.y, fine);
            if (value - tempo).abs() > 1e-9 {
                set(actions, state, value);
            }
        }
    }
    let notch = if lcd.hovered() {
        crate::scrollbars::wheel_notches(ui)
    } else {
        0.0
    };
    if notch != 0.0 {
        state.begin_gesture();
        let step = if fine { 0.01 } else { 1.0 };
        let value = (tempo + f64::from(notch) * step).clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM);
        set(actions, state, value);
    }
    if lcd.double_clicked() {
        ui.data_mut(|d| d.insert_temp(id.with("typing"), format!("{tempo:.3}")));
    }
    lcd.context_menu(|ui| {
        if ui.button("Enter a tempo…").clicked() {
            ui.data_mut(|d| d.insert_temp(id.with("typing"), format!("{tempo:.3}")));
            ui.close();
        }
        ui.separator();
        for round in ROUND_TEMPOS {
            if ui.button(format!("{round:.0} BPM")).clicked() {
                state.begin_gesture();
                set(actions, state, round);
                ui.close();
            }
        }
        ui.separator();
        if ui.button("Copy").clicked() {
            ui.ctx().copy_text(format!("{tempo:.3}"));
            ui.close();
        }
        let reset = musiccraft_project::DEFAULT_TEMPO_BPM;
        if ui.button(format!("Reset to {reset:.0}")).clicked() {
            state.begin_gesture();
            set(actions, state, reset);
            ui.close();
        }
    });
}

/// The tempo display's type-in field: Enter applies, Esc cancels.
fn tempo_entry(
    ui: &mut egui::Ui,
    state: &mut UiState,
    id: egui::Id,
    mut typed: String,
    actions: &mut Vec<Action>,
) {
    let field = ui.add(
        egui::TextEdit::singleline(&mut typed)
            .id(id.with("field"))
            .desired_width(74.0 - 8.0)
            .font(egui::FontId::monospace(theme::FONT_VALUE)),
    );
    if !field.has_focus() && !field.lost_focus() {
        field.request_focus();
    }
    // Taken here so neither key also reaches the window shortcuts.
    let (enter, escape) = ui.input_mut(|i| {
        (
            i.consume_key(egui::Modifiers::NONE, egui::Key::Enter),
            i.consume_key(egui::Modifiers::NONE, egui::Key::Escape),
        )
    });
    if escape || (field.lost_focus() && !enter) {
        ui.data_mut(|d| d.remove::<String>(id.with("typing")));
    } else if enter {
        ui.data_mut(|d| d.remove::<String>(id.with("typing")));
        if let Ok(value) = typed.trim().parse::<f64>() {
            state.begin_gesture();
            actions.push(Action::Coalesced(
                Invocation::new(
                    "project.set_tempo",
                    [format!("{:.3}", value.clamp(MIN_TEMPO_BPM, MAX_TEMPO_BPM))],
                ),
                state.gesture(),
            ));
        }
    } else {
        ui.data_mut(|d| d.insert_temp(id.with("typing"), typed));
    }
}

/// The pattern list and the plus that adds the next empty pattern.
fn pattern_panel(
    ui: &mut egui::Ui,
    project: &musiccraft_project::Project,
    actions: &mut Vec<Action>,
) {
    let current = project.current_pattern;
    egui::ComboBox::from_id_salt("pattern-select")
        .width(118.0)
        .selected_text(
            RichText::new(format!(
                "{}  {}",
                current + 1,
                project.current_pattern().name
            ))
            .color(crate::pattern_text(project.current_pattern())),
        )
        .show_ui(ui, |ui| {
            for (i, pattern) in project.patterns.iter().enumerate() {
                let text = RichText::new(format!("{}  {}", i + 1, pattern.name))
                    .color(crate::pattern_text(pattern));
                if ui.selectable_label(i == current, text).clicked() {
                    run(actions, &format!("pattern.select {}", i + 1));
                }
            }
        })
        .response
        .hint("Selected pattern (Plus / Minus)");
    if icons::button(
        ui,
        Icon::Plus,
        false,
        theme::ACCENT,
        "New pattern (F4: next empty pattern)",
    )
    .clicked()
    {
        run(actions, "pattern.add");
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

#[cfg(test)]
mod tests {
    use super::dragged_tempo;

    #[test]
    fn tempo_drags_step_whole_digits_or_decimals() {
        // Whole digits: 1 BPM per 6 points, keeping the decimals.
        assert!((dragged_tempo(120.5, true, 13.0, false) - 122.5).abs() < 1e-9);
        assert!((dragged_tempo(120.5, true, -5.0, false) - 120.5).abs() < 1e-9);
        assert!((dragged_tempo(120.0, true, 12.0, true) - 120.2).abs() < 1e-9);
        // Decimals: 0.01 BPM per point, 0.001 with Shift.
        assert!((dragged_tempo(120.0, false, 10.0, false) - 120.1).abs() < 1e-9);
        assert!((dragged_tempo(120.0, false, -3.0, true) - 119.997).abs() < 1e-9);
        // The range holds.
        assert!(
            (dragged_tempo(15.0, true, -600.0, false) - musiccraft_project::MIN_TEMPO_BPM).abs()
                < 1e-9
        );
    }
}
