//! The mixer: one strip per track plus the selected track's effect chain.

use crate::hint::Hint;
use egui::{Color32, RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{EFFECT_KINDS, MAX_EFFECT_SLOTS};
use musiccraft_project::{MAX_VOLUME, MixerTrack, Project};

use crate::dialogs::Dialog;
use crate::icons::{self, Icon};
use crate::{Action, Frame, UiState, format, theme, widgets};

const STRIP_WIDTH: f32 = 40.0;

fn run(actions: &mut Vec<Action>, name: &str, args: impl IntoIterator<Item = String>) {
    actions.push(Action::Run(Invocation::new(name, args)));
}

/// The mixer window: the strips, then the selected track's slot panel.
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
    let id = egui::Id::new("mixer-window");
    if !open {
        return;
    }
    // At the top, right of the channel rack, up to the right edge, a little
    // under half the height.
    let rack_right = ctx
        .memory(|m| m.area_rect(egui::Id::new("channel-rack")))
        .map_or(area.left(), |r| r.right());
    let left = rack_right.max(area.left() + area.width() / 3.0) + 4.0;
    let default = egui::Rect::from_min_max(
        egui::pos2(left.min(area.right() - 500.0), area.top() + 4.0),
        egui::pos2(area.right() - 4.0, area.top() + area.height() * 0.42 - 4.0),
    );
    // The rack takes a few frames to reach its width, so the mixer follows
    // its default slot for its first frames.
    let settle = egui::Id::new("mixer-settle");
    let frames = ctx.data(|d| d.get_temp::<u8>(settle)).unwrap_or(0);
    let mut window = crate::chrome::window(ctx, id, "Mixer", area, default, true);
    if frames < 4 {
        ctx.data_mut(|d| d.insert_temp(settle, frames + 1));
        window = window.current_pos(default.min);
    }
    let window = window
        .default_height(default.height().max(STRIP_MIN_HEIGHT))
        .min_width(320.0)
        .show(ctx, |ui| {
            let folded = crate::chrome::Header::new(id, &title).title_at_end().show(
                ui,
                &mut open,
                &mut (&mut *state, &mut *actions),
                |ui, (state, actions)| window_menu(ui, state, project, actions),
                |ui, (state, actions)| header_tools(ui, state, actions),
            );
            if folded {
                return;
            }
            let height = (ui.available_height() - 16.0).max(STRIP_MIN_HEIGHT);
            ui.horizontal_top(|ui| {
                // The panel, the separator and the gaps around it, so the
                // row never outgrows the window.
                let panel = if state.mixer_panel {
                    EFFECTS_WIDTH + 2.0 * ui.spacing().item_spacing.x + 8.0
                } else {
                    0.0
                };
                // The wheel over a strip's name steps whole strips
                // (`vertical_name`); elsewhere it is left to the controls.
                let strip_bars = egui::Id::new(STRIP_BARS);
                let area = egui::ScrollArea::horizontal()
                    .scroll_source(egui::scroll_area::ScrollSource {
                        mouse_wheel: false,
                        ..egui::scroll_area::ScrollSource::ALL
                    })
                    .id_salt("mixer-strips")
                    .max_width((ui.available_width() - panel).max(160.0))
                    .auto_shrink([!state.mixer_panel, true]);
                // The strips' bar sits under them, inside the 16 points the
                // strip height leaves free.
                ui.vertical(|ui| {
                    let bars = (strip_bars, 42.0);
                    crate::scrollbars::plain_area(ui, area, bars, |ui| {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for (index, track) in project.mixer.iter().enumerate() {
                                strip(ui, state, project, index, track, height, actions);
                                if index == 0 {
                                    ui.add_space(6.0);
                                }
                            }
                        });
                    });
                });
                if state.mixer_panel {
                    ui.separator();
                    effects(ui, state, frame, actions);
                }
            });
        });
    state.windows.mixer = open;
    crate::chrome::remember(ctx, id, window.as_ref().map(|w| &w.response));
}

/// The shortest the strips get; the fader gives up travel first.
const STRIP_MIN_HEIGHT: f32 = 300.0;

/// The header's tools: strip details, slot panel, add insert.
fn header_tools(ui: &mut egui::Ui, state: &mut UiState, actions: &mut Vec<Action>) {
    if icons::header_button(
        ui,
        Icon::Details,
        state.mixer_details,
        false,
        "Strip details: show or hide phase, swap, separation and effects",
    )
    .clicked()
    {
        state.mixer_details = !state.mixer_details;
    }
    if icons::header_button(
        ui,
        Icon::SidePanel,
        state.mixer_panel,
        false,
        "Slot panel: show or hide the selected track's effects",
    )
    .clicked()
    {
        state.mixer_panel = !state.mixer_panel;
    }
    if icons::header_button(ui, Icon::Plus, false, false, "Add an insert track").clicked() {
        run(actions, "mixer.add_track", []);
    }
}

/// The mixer's own window-menu items.
fn window_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    ui.menu_button("View", |ui| {
        ui.checkbox(&mut state.mixer_details, "Strip details");
        ui.checkbox(&mut state.mixer_panel, "Slot panel");
    });
    let index = state.selected_track;
    let Some(track) = project.mixer.get(index) else {
        return;
    };
    ui.menu_button("Tracks", |ui| {
        if ui.button("Rename…").clicked() {
            state.open_dialog(Dialog::rename_track(index, &track.name));
            ui.close();
        }
        if index > 0
            && ui
                .button(RichText::new("Delete").color(theme::DANGER))
                .clicked()
        {
            run(actions, "mixer.remove_track", [index.to_string()]);
            ui.close();
        }
    });
}

const EFFECTS_WIDTH: f32 = 270.0;

/// Id of the strips' scroll bars.
const STRIP_BARS: &str = "mixer-strip-bars";

/// Width of the name beside the meter in a strip's upper block.
const NAME_WIDTH: f32 = 22.0;

/// A strip's name, written bottom to top so narrow strips fit long names.
fn vertical_name(ui: &mut egui::Ui, name: &str, selected: bool, height: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(NAME_WIDTH, height), egui::Sense::click());
    let mut job = egui::text::LayoutJob::simple_singleline(
        name.to_owned(),
        egui::FontId::proportional(theme::FONT_LABEL),
        if selected {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    );
    job.wrap = egui::text::TextWrapping::truncate_at_width(height - 4.0);
    let galley = ui.painter().layout_job(job);
    let pos = egui::pos2(rect.center().x - galley.size().y / 2.0, rect.bottom() - 2.0);
    ui.painter().add(
        egui::epaint::TextShape::new(pos, galley, theme::TEXT)
            .with_angle(-std::f32::consts::FRAC_PI_2),
    );
    step_strips_on_wheel(ui, &response);
    response
}

/// The wheel over `response` (a strip's name or number) scrolls the strips by
/// whole strips.
fn step_strips_on_wheel(ui: &egui::Ui, response: &egui::Response) {
    let notches = if response.hovered() {
        crate::scrollbars::wheel_notches(ui)
    } else {
        0.0
    };
    if notches != 0.0 {
        let step = vec2(-notches * (STRIP_WIDTH + 2.0), 0.0);
        crate::scrollbars::plain_step(ui.ctx(), egui::Id::new(STRIP_BARS), step);
    }
}

/// A round light of `diameter`: filled in `color` when on, a dim ring when
/// off.
fn light(
    ui: &mut egui::Ui,
    diameter: f32,
    on: bool,
    color: egui::Color32,
    hint: &str,
) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(vec2(diameter + 6.0, diameter + 2.0), egui::Sense::click());
    let center = rect.center();
    let radius = diameter / 2.0;
    if on {
        ui.painter().circle_filled(center, radius, color);
    } else {
        ui.painter()
            .circle_filled(center, radius, theme::PANEL_DARK);
        ui.painter().circle_stroke(
            center,
            radius - 0.5,
            egui::Stroke::new(
                1.0,
                if response.hovered() {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                },
            ),
        );
    }
    response.hint(hint)
}

/// A tiny square switch showing `icon`: dim when off, the highlight tone on
/// a raised square when on.
fn small_switch(ui: &mut egui::Ui, icon: Icon, on: bool, hint: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(16.0, 14.0), egui::Sense::click());
    if on || response.hovered() {
        let fill = if response.hovered() {
            theme::RAISED_HOVER
        } else {
            theme::RAISED
        };
        ui.painter().rect_filled(rect, 2.0, fill);
    }
    let color = if on {
        theme::HIGHLIGHT
    } else {
        theme::TEXT_DIM
    };
    icons::paint(ui.painter(), rect, icon, color);
    response.hint(hint)
}

/// The number cell: the number (M for the master) on a cell tinted with the
/// track's colour.
fn number_cell(
    ui: &mut egui::Ui,
    index: usize,
    color: egui::Color32,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(STRIP_WIDTH, 16.0), egui::Sense::click());
    let tint = theme::mix(theme::RAISED, color, if selected { 0.6 } else { 0.35 });
    ui.painter().rect_filled(rect, 2.0, tint);
    let label = if index == 0 {
        "M".to_owned()
    } else {
        index.to_string()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(theme::FONT_LABEL + 1.0),
        theme::text_on(tint),
    );
    response
}

/// The track menu (right-click on the number or name).
fn track_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    let number = index.to_string();
    if ui.button("Rename…").clicked() {
        state.open_dialog(Dialog::rename_track(index, &track.name));
        ui.close();
    }
    ui.horizontal(|ui| {
        ui.label("Stereo separation");
        let mut separation = track.separation;
        let drag = ui.add(
            egui::DragValue::new(&mut separation)
                .range(-1.0..=1.0)
                .speed(0.01),
        );
        if drag.drag_started() || drag.gained_focus() {
            state.begin_gesture();
        }
        if drag.changed() {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "mixer.set_separation",
                    [number.clone(), format!("{separation:.3}")],
                ),
                state.gesture(),
            ));
        }
    })
    .response
    .on_hover_text("-1 mono, 0 as is, 1 wider");
    for (label, on, name) in [
        ("Invert phase", track.phase_invert, "mixer.set_phase_invert"),
        (
            "Swap left and right",
            track.swap_sides,
            "mixer.set_swap_sides",
        ),
    ] {
        if ui.selectable_label(on, label).clicked() {
            run(
                actions,
                name,
                [number.clone(), if on { "off" } else { "on" }.to_owned()],
            );
            ui.close();
        }
    }
    if index > 0
        && ui
            .button(RichText::new("Delete").color(theme::DANGER))
            .clicked()
    {
        run(actions, "mixer.remove_track", [number.clone()]);
        ui.close();
    }
}

/// Runs a knob gesture as one coalesced undo step of `name <number> <value>`.
fn knob_command(
    state: &mut UiState,
    out: &widgets::KnobOutput,
    name: &str,
    number: &str,
    actions: &mut Vec<Action>,
) {
    if out.started {
        state.begin_gesture();
    }
    if let Some(value) = out.changed {
        actions.push(Action::Coalesced(
            Invocation::new(name, [number.to_owned(), format!("{value:.4}")]),
            state.gesture(),
        ));
    }
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
        .corner_radius(2.0)
        .inner_margin(egui::Margin::symmetric(0, 2));
    let response = frame.show(ui, |ui| {
        ui.set_width(STRIP_WIDTH);
        ui.set_height(height - 4.0);
        ui.vertical_centered(|ui| {
            ui.spacing_mut().item_spacing.y = 3.0;
            let hint = format!("{} — click to select, right-click for options", track.name);
            let cell = number_cell(ui, index, color, selected).hint(&hint);
            step_strips_on_wheel(ui, &cell);
            let block = (height * 0.22).clamp(56.0, 120.0);
            let name = ui
                .horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(3.0);
                    let name = vertical_name(ui, &track.name, selected, block).hint(&hint);
                    let levels = state.meters.get(index).copied().unwrap_or_default();
                    let meter = widgets::meter(ui, levels, vec2(10.0, block));
                    if selected {
                        ui.painter().rect_stroke(
                            meter.rect.expand(1.0),
                            2.0,
                            egui::Stroke::new(1.0, theme::ACCENT),
                            egui::StrokeKind::Outside,
                        );
                    }
                    name.union(meter.interact(egui::Sense::click()))
                })
                .inner
                .union(cell);
            if name.clicked() {
                state.selected_track = index;
            }
            if name.double_clicked() {
                // The name reads upward; the field is a plain row at its top.
                let field = egui::Rect::from_min_size(name.rect.left_top(), vec2(120.0, 20.0));
                state.rename_in_place(Dialog::rename_track(index, &track.name), field);
            }
            name.context_menu(|ui| track_menu(ui, state, index, track, actions));
            let heard = if track.mute { "muted" } else { "on" };
            let enable = light(
                ui,
                10.0,
                !track.mute,
                theme::ACCENT,
                &if index > 0 {
                    format!("Enable ({heard}) · click to mute or unmute · Ctrl+click to solo")
                } else {
                    format!("Enable ({heard}) · click to mute or unmute")
                },
            );
            let solo_key = ui.input(|i| i.modifiers.command);
            if enable.clicked() {
                if solo_key && index > 0 {
                    run(actions, "mixer.toggle_solo", [number.clone()]);
                } else {
                    run(actions, "mixer.toggle_mute", [number.clone()]);
                }
            }
            enable.context_menu(|ui| {
                let mute = if track.mute { "Unmute" } else { "Mute" };
                if ui.button(mute).clicked() {
                    run(actions, "mixer.toggle_mute", [number.clone()]);
                    ui.close();
                }
                if index > 0 {
                    let solo = if track.solo { "Unsolo" } else { "Solo" };
                    if ui.button(solo).clicked() {
                        run(actions, "mixer.toggle_solo", [number.clone()]);
                        ui.close();
                    }
                }
            });
            if index > 0 {
                if light(ui, 7.0, track.solo, theme::HIGHLIGHT, "Solo").clicked() {
                    run(actions, "mixer.toggle_solo", [number.clone()]);
                }
            } else {
                ui.add_space(9.0);
            }
            let pan = widgets::knob(ui, track.pan, -1.0, 1.0, 0.0, true);
            crate::automation::menu(&pan.response, &format!("mixer:{index}:pan"), actions);
            knob_command(state, &pan, "mixer.set_pan", &number, actions);
            pan.response.hint(format!("Pan {}", format::pan(track.pan)));
            let below = if state.mixer_details { 84.0 } else { 0.0 } + 20.0;
            let fader_height = (ui.available_height() - below).max(60.0);
            let fader = widgets::fader(
                ui,
                track.volume,
                MAX_VOLUME,
                vec2(STRIP_WIDTH - 12.0, fader_height),
            );
            crate::automation::menu(&fader.response, &format!("mixer:{index}:volume"), actions);
            knob_command(state, &fader, "mixer.set_volume", &number, actions);
            fader
                .response
                .hint(format!("Level {}", format::level(track.volume)));
            if state.mixer_details {
                details(ui, state, &number, track, actions);
            }
            route_arrow(ui, state, project, index, actions);
        });
    });
    // A click anywhere on the strip selects it. This reads the pointer
    // instead of adding a strip-wide click area, which would sit on top of
    // the lights, switches and name and take their clicks and hovers.
    let clicked_here = ui.input(|i| {
        i.pointer.primary_clicked()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| response.response.rect.contains(p))
    });
    if clicked_here && ui.is_rect_visible(response.response.rect) {
        state.selected_track = index;
    }
}

/// The track switches: invert phase, swap, separation and the effect count.
fn details(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: &str,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    ui.spacing_mut().item_spacing.y = 1.0;
    for (icon, on, name, label) in [
        (
            Icon::Phase,
            track.phase_invert,
            "mixer.set_phase_invert",
            "Invert phase",
        ),
        (
            Icon::Swap,
            track.swap_sides,
            "mixer.set_swap_sides",
            "Swap left and right",
        ),
    ] {
        let state_word = if on { "on" } else { "off" };
        if small_switch(ui, icon, on, &format!("{label}: {state_word}")).clicked() {
            run(
                actions,
                name,
                [number.to_owned(), if on { "off" } else { "on" }.to_owned()],
            );
        }
    }
    let sep = widgets::knob(ui, track.separation, -1.0, 1.0, 0.0, true);
    knob_command(state, &sep, "mixer.set_separation", number, actions);
    sep.response.hint(format!(
        "Stereo separation {:+.0} (left: mono, right: wider)",
        track.separation * 100.0
    ));
    effects_switch(ui, track, number, actions);
}

/// The effect count at the foot of a strip, lit while any effect runs.
/// Clicking it bypasses every effect of the track, or brings them all back.
fn effects_switch(ui: &mut egui::Ui, track: &MixerTrack, number: &str, actions: &mut Vec<Action>) {
    let count = track.effects.len();
    let running = track.effects.iter().filter(|e| e.enabled).count();
    let names: Vec<String> = track.effects.iter().map(|e| e.label()).collect();
    let text = if count > 0 {
        format!("{count} fx")
    } else {
        " ".to_owned()
    };
    let color = if running > 0 {
        theme::ACCENT
    } else {
        theme::TEXT_FAINT
    };
    let response = ui
        .add(
            egui::Label::new(
                RichText::new(text)
                    .size(theme::FONT_CAPTION + 0.5)
                    .color(color),
            )
            .sense(egui::Sense::click()),
        )
        .hint(if names.is_empty() {
            "No effects".to_owned()
        } else if running > 0 {
            format!(
                "Effects: {} · click to bypass them all · right-click to open their windows",
                names.join(", ")
            )
        } else {
            format!(
                "Effects (all bypassed): {} · click to enable them · right-click to open their windows",
                names.join(", ")
            )
        });
    if count > 0 {
        response.context_menu(|ui| {
            if ui.button("Open all effect windows").clicked() {
                run(actions, "ui.open_effects", [number.to_owned()]);
            }
        });
    }
    if count > 0 && response.clicked() {
        let on = if running > 0 { "off" } else { "on" };
        run(
            actions,
            "mixer.set_effects_enabled",
            [number.to_owned(), on.to_owned()],
        );
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
    let c = rect.center();
    let down = vec![
        c + vec2(-6.0, -4.0),
        c + vec2(6.0, -4.0),
        c + vec2(0.0, 4.0),
    ];
    if target == source && source > 0 {
        // The selected insert shows a source arrow; its hint says where it goes.
        ui.painter().add(egui::Shape::convex_polygon(
            vec![c + vec2(-6.0, 4.0), c + vec2(6.0, 4.0), c + vec2(0.0, -4.0)],
            theme::HIGHLIGHT,
            egui::Stroke::NONE,
        ));
        response.hint(format!("This insert goes to {}", route_summary(from)));
        return;
    }
    let possible = source > 0 && (target == 0 || target > source);
    if !possible {
        // Inert: a faint outline where no route can go.
        if source > 0 {
            ui.painter().add(egui::Shape::convex_polygon(
                down,
                Color32::TRANSPARENT,
                egui::Stroke::new(1.0, theme::RAISED_HOVER),
            ));
        }
        return;
    }
    let routed = if target == 0 {
        from.to_master
    } else {
        from.sends.iter().any(|s| s.target == target)
    };
    // Routed: filled in the highlight tone. Possible: an outline that
    // brightens on hover.
    let (fill, edge) = if routed {
        (theme::HIGHLIGHT, theme::HIGHLIGHT)
    } else if response.hovered() {
        (Color32::TRANSPARENT, theme::TEXT)
    } else {
        (Color32::TRANSPARENT, theme::TEXT_DIM)
    };
    ui.painter().add(egui::Shape::convex_polygon(
        down,
        fill,
        egui::Stroke::new(1.0, edge),
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

/// Where an insert's output goes: `M`, `3`, `M, 3, 5` or `none` (drawn
/// after an [`Icon::Out`] glyph).
fn route_summary(track: &MixerTrack) -> String {
    let mut to: Vec<String> = Vec::new();
    if track.to_master {
        to.push("M".to_owned());
    }
    to.extend(track.sends.iter().map(|s| s.target.to_string()));
    if to.is_empty() {
        "none".to_owned()
    } else {
        to.join(", ")
    }
}

/// A device picker; `None` is the system default. Returns the choice made.
fn device_picker(
    ui: &mut egui::Ui,
    id: &str,
    current: Option<&String>,
    devices: &[String],
) -> Option<Option<String>> {
    let mut picked = None;
    egui::ComboBox::from_id_salt(id)
        .width(EFFECTS_WIDTH - 50.0)
        .selected_text(current.map_or("System default", String::as_str))
        .show_ui(ui, |ui| {
            let choices = std::iter::once(None).chain(devices.iter().cloned().map(Some));
            for choice in choices {
                let label = choice
                    .clone()
                    .unwrap_or_else(|| "System default".to_owned());
                if ui
                    .selectable_label(current == choice.as_ref(), label)
                    .clicked()
                {
                    picked = Some(choice);
                }
            }
        });
    picked
}

/// The selected track's input: the audio input device recorded from.
fn input_row(ui: &mut egui::Ui, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        icons::glyph(ui, Icon::In, 12.0, theme::TEXT_DIM).hint(
            "Audio input recorded from: the one input device for every track (Settings → Audio)",
        );
        let settings = frame.settings;
        let inputs = &frame.settings_info.inputs;
        if let Some(choice) =
            device_picker(ui, "mixer-input", settings.audio_input.as_ref(), inputs)
        {
            actions.push(Action::Settings(Box::new(crate::Settings {
                audio_input: choice,
                ..settings.clone()
            })));
        }
    });
}

/// The selected track's output: where an insert routes, or the master's
/// audio device.
fn output_row(
    ui: &mut egui::Ui,
    frame: &Frame<'_>,
    index: usize,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    ui.horizontal(|ui| {
        if index > 0 {
            icons::glyph(ui, Icon::Out, 12.0, theme::TEXT_DIM);
            ui.label(RichText::new(route_summary(track)).monospace())
                .hint("Set with the arrows under the strips, or To master / Send below");
            return;
        }
        icons::glyph(ui, Icon::Out, 12.0, theme::TEXT_DIM)
            .hint("Audio output device the master plays through");
        let settings = frame.settings;
        let outputs = &frame.settings_info.outputs;
        if let Some(choice) =
            device_picker(ui, "mixer-output", settings.audio_device.as_ref(), outputs)
        {
            actions.push(Action::Settings(Box::new(crate::Settings {
                audio_device: choice,
                ..settings.clone()
            })));
        }
    });
}

/// The track's built-in EQ: a gain knob per band; right-click for its
/// frequency.
fn eq_row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    index: usize,
    track: &MixerTrack,
    actions: &mut Vec<Action>,
) {
    use musiccraft_project::TrackEq;
    let number = index.to_string();
    ui.horizontal(|ui| {
        ui.label(RichText::new("EQ").small().color(theme::TEXT_DIM));
        for (band, name) in TrackEq::BANDS.iter().enumerate() {
            let (gain, freq) = track.eq.bands[band];
            ui.vertical(|ui| {
                let max = TrackEq::MAX_GAIN;
                let knob = widgets::knob(ui, gain, -max, max, 0.0, true);
                if knob.started {
                    state.begin_gesture();
                }
                if let Some(value) = knob.changed {
                    actions.push(Action::Coalesced(
                        Invocation::new(
                            "mixer.set_eq",
                            [number.clone(), (*name).to_owned(), format!("{value:.2}")],
                        ),
                        state.gesture(),
                    ));
                }
                let response = knob.response.hint(format!(
                    "{name}: {gain:+.1} dB at {freq:.0} Hz (right-click: frequency)"
                ));
                response.context_menu(|ui| {
                    let (lo, hi) = TrackEq::RANGES[band];
                    let mut f = freq;
                    ui.horizontal(|ui| {
                        ui.label("Frequency");
                        let drag = ui.add(
                            egui::DragValue::new(&mut f)
                                .range(lo..=hi)
                                .speed(f64::from(freq) * 0.01)
                                .suffix(" Hz"),
                        );
                        if drag.drag_started() || drag.gained_focus() {
                            state.begin_gesture();
                        }
                        if drag.changed() {
                            actions.push(Action::Coalesced(
                                Invocation::new(
                                    "mixer.set_eq",
                                    [
                                        number.clone(),
                                        (*name).to_owned(),
                                        format!("{gain:.2}"),
                                        format!("{f:.1}"),
                                    ],
                                ),
                                state.gesture(),
                            ));
                        }
                    });
                });
                ui.label(RichText::new(*name).small().color(theme::TEXT_DIM));
            });
        }
    });
}

fn effects(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let plugins = frame.plugins;
    let index = state.selected_track;
    let Some(track) = project.mixer.get(index) else {
        return;
    };
    let number = index.to_string();
    ui.vertical(|ui| {
        ui.set_width(EFFECTS_WIDTH);
        input_row(ui, frame, actions);
        let area = egui::ScrollArea::vertical().id_salt("effect-chain");
        let bars = (egui::Id::new("mixer-slot-bars"), 25.0);
        crate::scrollbars::plain_area(ui, area, bars, |ui| {
            let mut rows = Vec::with_capacity(MAX_EFFECT_SLOTS);
            for slot in 0..MAX_EFFECT_SLOTS {
                rows.push(slot_row(ui, state, track, &number, slot, plugins, actions));
            }
            ui.ctx().data_mut(|d| d.insert_temp(slot_rows_id(), rows));
            ui.separator();
            eq_row(ui, state, index, track, actions);
            output_row(ui, frame, index, track, actions);
            if index > 0 {
                routing(ui, state, project, index, track, actions);
            }
        });
    });
}

fn track_index(number: &str) -> usize {
    number.parse().unwrap_or(0)
}

/// One effect slot: chevron (slot menu), name, mix knob and a light that
/// bypasses or restores the effect. An empty slot shows a dim "Slot n" and
/// opens the add list.
/// Where last frame's slot rows were, for dropping a dragged effect.
fn slot_rows_id() -> egui::Id {
    egui::Id::new("mixer-slot-rows")
}

/// The loaded slot under `pos` in last frame's rows.
fn slot_at(ui: &egui::Ui, pos: egui::Pos2, loaded: usize) -> Option<usize> {
    let rows: Vec<egui::Rect> = ui.ctx().data(|d| d.get_temp(slot_rows_id()))?;
    rows.iter()
        .position(|r| r.y_range().contains(pos.y))
        .map(|slot| slot.min(loaded.saturating_sub(1)))
}

/// One slot row; returns its rectangle.
fn slot_row(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: &MixerTrack,
    number: &str,
    slot: usize,
    plugins: &crate::Plugins,
    actions: &mut Vec<Action>,
) -> egui::Rect {
    let slot_no = (slot + 1).to_string();
    let selected = state.mixer_slot == slot;
    let effect = track.effects.get(slot);
    let loaded = track.effects.len();
    egui::Frame::new()
        .fill(if selected && effect.is_some() {
            theme::RAISED
        } else {
            theme::PANEL_DARK
        })
        .corner_radius(2.0)
        .inner_margin(egui::Margin::symmetric(2, 1))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let popup = egui::Id::new(("mixer-slot-menu", slot));
                let open = egui::Popup::is_id_open(ui.ctx(), popup);
                let (rect, chevron) =
                    ui.allocate_exact_size(vec2(14.0, 20.0), egui::Sense::click());
                let glyph = if open || chevron.hovered() {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                };
                icons::paint(ui.painter(), rect, Icon::ChevronRight, glyph);
                let Some(effect) = effect else {
                    let name = ui
                        .add(
                            egui::Label::new(
                                RichText::new(format!("Slot {slot_no}"))
                                    .italics()
                                    .color(theme::TEXT_FAINT),
                            )
                            .sense(egui::Sense::click()),
                        )
                        .hint("Add an effect");
                    let anchor = chevron.union(name);
                    let toggle = anchor.clicked().then_some(egui::SetOpenCommand::Toggle);
                    egui::Popup::menu(&anchor)
                        .id(popup)
                        .open_memory(toggle)
                        .show(|ui| {
                            add_list(ui, state, number, track.effects.len(), plugins, actions)
                        });
                    return;
                };
                let chevron = chevron.hint("Slot menu");
                let toggle = chevron.clicked().then_some(egui::SetOpenCommand::Toggle);
                egui::Popup::menu(&chevron)
                    .id(popup)
                    .open_memory(toggle)
                    .show(|ui| slot_menu(ui, state, track, number, slot, true, actions));
                let name = RichText::new(effect.label()).color(if effect.enabled {
                    theme::TEXT
                } else {
                    theme::TEXT_DIM
                });
                let name = ui
                    .add(
                        egui::Label::new(name)
                            .truncate()
                            .sense(egui::Sense::click_and_drag()),
                    )
                    .hint("Open or close this effect's window · drag to reorder");
                if name.clicked() {
                    state.mixer_slot = slot;
                    crate::effect_window::toggle(state, track_index(number), slot);
                }
                drag_to_reorder(ui, &name, number, slot, loaded, actions);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let word = if effect.enabled { "on" } else { "bypassed" };
                    if light(
                        ui,
                        8.0,
                        effect.enabled,
                        theme::ACCENT,
                        &format!("Effect {word} · click to bypass or restore"),
                    )
                    .clicked()
                    {
                        run(
                            actions,
                            "mixer.toggle_effect",
                            [number.to_owned(), slot_no.clone()],
                        );
                    }
                    let mix = widgets::knob(ui, effect.mix, 0.0, 1.0, 1.0, false);
                    if mix.started {
                        state.begin_gesture();
                    }
                    if let Some(value) = mix.changed {
                        actions.push(Action::Coalesced(
                            Invocation::new(
                                "mixer.set_effect_mix",
                                [number.to_owned(), slot_no.clone(), format!("{value:.3}")],
                            ),
                            state.gesture(),
                        ));
                    }
                    mix.response.hint(format!("Mix {:.0}%", effect.mix * 100.0));
                });
            });
        })
        .response
        .rect
}

/// Dragging a loaded slot's name moves its effect to the slot it is dropped
/// on (`mixer.move_effect`); a line marks where it will land.
fn drag_to_reorder(
    ui: &egui::Ui,
    name: &egui::Response,
    number: &str,
    slot: usize,
    loaded: usize,
    actions: &mut Vec<Action>,
) {
    let Some(pos) = ui.ctx().pointer_interact_pos() else {
        return;
    };
    let target = slot_at(ui, pos, loaded).filter(|t| *t != slot);
    if name.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
        let rows: Option<Vec<egui::Rect>> = ui.ctx().data(|d| d.get_temp(slot_rows_id()));
        if let Some(rect) = target.and_then(|t| rows?.get(t).copied()) {
            let y = if target > Some(slot) {
                rect.bottom()
            } else {
                rect.top()
            };
            ui.ctx()
                .layer_painter(egui::LayerId::new(egui::Order::Tooltip, name.id))
                .hline(rect.x_range(), y, egui::Stroke::new(2.0, theme::ACCENT));
        }
    }
    if let Some(target) = target.filter(|_| name.drag_stopped()) {
        run(
            actions,
            "mixer.move_effect",
            [
                number.to_owned(),
                (slot + 1).to_string(),
                (target + 1).to_string(),
            ],
        );
    }
}

/// The menu of a loaded slot, also the effect window's menu (`with_open`:
/// whether to offer "Open window").
pub(crate) fn slot_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: &MixerTrack,
    number: &str,
    slot: usize,
    with_open: bool,
    actions: &mut Vec<Action>,
) {
    let slot_no = (slot + 1).to_string();
    let enabled = track.effects.get(slot).is_some_and(|e| e.enabled);
    if with_open && ui.button("Open window").clicked() {
        state.mixer_slot = slot;
        crate::effect_window::toggle(state, track_index(number), slot);
        ui.close();
    }
    if ui
        .button(if enabled { "Bypass" } else { "Enable" })
        .clicked()
    {
        run(
            actions,
            "mixer.toggle_effect",
            [number.to_owned(), slot_no.clone()],
        );
        ui.close();
    }
    if ui
        .add_enabled(slot > 0, egui::Button::new("Move up"))
        .clicked()
    {
        run(
            actions,
            "mixer.move_effect",
            [number.to_owned(), slot_no.clone(), slot.to_string()],
        );
        ui.close();
    }
    if ui
        .add_enabled(
            slot + 1 < track.effects.len(),
            egui::Button::new("Move down"),
        )
        .clicked()
    {
        run(
            actions,
            "mixer.move_effect",
            [number.to_owned(), slot_no.clone(), (slot + 2).to_string()],
        );
        ui.close();
    }
    ui.separator();
    if ui
        .button(RichText::new("Remove").color(theme::DANGER))
        .clicked()
    {
        run(actions, "mixer.remove_effect", [number.to_owned(), slot_no]);
        ui.close();
    }
}

/// The built-in effects, then the plugin effects; the choice goes into
/// `slot`, the first free one.
fn add_list(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: &str,
    slot: usize,
    plugins: &crate::Plugins,
    actions: &mut Vec<Action>,
) {
    if slot >= MAX_EFFECT_SLOTS {
        ui.label(RichText::new("Every slot is in use").color(theme::TEXT_DIM));
        return;
    }
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
    ui.menu_button("Plugin effects", |ui| {
        let chosen = crate::plugins::add_menu(ui, plugins, false, actions, |p| {
            Invocation::new(
                "mixer.add_plugin",
                [number.to_owned(), p.id.clone(), p.name.clone()],
            )
        });
        if chosen {
            state.mixer_slot = slot;
        }
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
        if crate::widgets::led_switch(ui, &mut to_master, "To master").changed() {
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
            icons::glyph(ui, Icon::Out, 12.0, theme::TEXT_DIM);
            ui.add_sized(
                vec2(58.0, 16.0),
                egui::Label::new(
                    RichText::new(format!("{} {name}", send.target))
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
            let reset = crate::widgets::wants_reset(&response);
            if reset {
                run(
                    actions,
                    "mixer.set_send",
                    [number.clone(), send.target.to_string(), "1".to_owned()],
                );
            } else if response.changed() && level > 0.0 {
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
            if icons::header_button(ui, Icon::Close, false, true, "Remove send").clicked() {
                run(
                    actions,
                    "mixer.set_send",
                    [number.clone(), send.target.to_string(), "0".to_owned()],
                );
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_summaries_name_every_destination() {
        let mut track = MixerTrack::new("A");
        assert_eq!(route_summary(&track), "M");
        track.sends.push(musiccraft_project::MixerSend {
            target: 5,
            level: 1.0,
        });
        assert_eq!(route_summary(&track), "M, 5");
        track.to_master = false;
        track.sends.clear();
        assert_eq!(route_summary(&track), "none");
    }
}
