//! The playlist: arrange patterns as clips on track rows.

use crate::hint::Hint;
use std::collections::BTreeSet;

use egui::{
    Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Sense, Stroke, Vec2, pos2, vec2,
};
use musiccraft_commands::Invocation;
use musiccraft_project::params::PPQ;
use musiccraft_project::{
    AutomationId, Clip, ClipSource, MarkerAction, Meter, PLAYLIST_TRACKS, PlayMode, Project,
};

use crate::dialogs::Dialog;
use crate::geometry::{Grid, Hit, Snap, hit_test};
use crate::icons::{self, Icon};
use crate::piano_roll::Tool;
use crate::signature;
use crate::{Action, Frame, UiState, theme, widgets};

const HEADER_WIDTH: f32 = 124.0;
const RULER_HEIGHT: f32 = 20.0;
const LIST_WIDTH: f32 = 130.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Move,
    Resize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct ClipDrag {
    kind: DragKind,
    original: Clip,
    grab: f32,
    track: u32,
    start: u32,
    length: u32,
}

/// Ticks the zoom (`px_per_bar`) measures: a 4/4 bar, whatever the song's
/// signature.
const ZOOM_TICKS: u32 = 4 * PPQ;
/// Horizontal zoom range, in points per 4/4 bar.
const BAR_WIDTH_RANGE: (f32, f32) = (16.0, 400.0);
/// Track height: the default and the vertical zoom range, in points.
const TRACK_HEIGHT: f32 = 36.0;
const TRACK_HEIGHT_RANGE: (f32, f32) = (18.0, 96.0);

#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistState {
    px_per_bar: f32,
    row_height: f32,
    /// Scroll offset of the last frame, in points.
    offset: Vec2,
    /// Scroll offset asked for by the scroll bars, applied next frame.
    jump: Option<Vec2>,
    snap: Snap,
    drag: Option<ClipDrag>,
    /// Grid and visible area from the last frame (used by tests and tools).
    pub(crate) last_view: Option<(Grid, Rect)>,
    /// Selected clips as (track, start).
    pub(crate) selection: BTreeSet<(u32, u32)>,
    /// Rubber-band selection in progress.
    marquee: Option<(Pos2, Pos2)>,
    /// When set, clicking the playlist places this automation instead of the
    /// selected pattern.
    pub(crate) paint_automation: Option<AutomationId>,
    /// When set, clicking the playlist places an audio clip of this channel.
    pub(crate) paint_audio: Option<musiccraft_project::ChannelId>,
    /// What the picker beside the playlist lists.
    pub(crate) picker: Picker,
    /// Each pattern's miniature in the picker on the last frame, in pattern
    /// order (used by tests).
    pub(crate) picker_items: Vec<Rect>,
    /// Loop range being drawn on the ruler.
    loop_drag: Option<(u32, u32)>,
    /// The active tool.
    pub(crate) tool: Tool,
    /// Clips painted in the stroke in progress: (track, start, length).
    painted: Vec<(u32, u32, u32)>,
    /// Clips touched by the delete stroke in progress.
    erased: Vec<Clip>,
    /// The song's bars and beats, refreshed every frame.
    pub(crate) meter: Meter,
}

impl Default for PlaylistState {
    fn default() -> Self {
        Self {
            px_per_bar: 72.0,
            row_height: TRACK_HEIGHT,
            offset: Vec2::ZERO,
            jump: None,
            snap: Snap::Bar,
            drag: None,
            last_view: None,
            selection: BTreeSet::new(),
            marquee: None,
            paint_automation: None,
            paint_audio: None,
            picker: Picker::All,
            picker_items: Vec::new(),
            loop_drag: None,
            tool: Tool::Draw,
            painted: Vec::new(),
            erased: Vec::new(),
            meter: Meter::default(),
        }
    }
}

#[cfg(test)]
fn finish(drag: &ClipDrag) -> Option<Invocation> {
    let o = drag.original;
    let t = |ticks: u32| format!("{ticks}t");
    match drag.kind {
        DragKind::Move if (drag.track, drag.start) != (o.track, o.start) => Some(Invocation::new(
            "playlist.move_clip",
            [
                (o.track + 1).to_string(),
                t(o.start),
                (drag.track + 1).to_string(),
                t(drag.start),
            ],
        )),
        DragKind::Resize if drag.length != o.length => Some(Invocation::new(
            "playlist.resize_clip",
            [(o.track + 1).to_string(), t(o.start), t(drag.length)],
        )),
        _ => None,
    }
}

/// The playlist tools, in header order, with their hints.
const TOOLS: [(Tool, Icon, &str); 8] = [
    (
        Tool::Draw,
        Icon::Pencil,
        "Draw (P): click places the selected item, drag moves, edge resizes",
    ),
    (
        Tool::Paint,
        Icon::Brush,
        "Paint (B): drag to lay the selected item end to end",
    ),
    (
        Tool::Delete,
        Icon::Eraser,
        "Delete (D): click or drag over clips to delete them",
    ),
    (
        Tool::Mute,
        Icon::Mute,
        "Mute (T): click a clip to mute or unmute it",
    ),
    (
        Tool::Slice,
        Icon::Slice,
        "Slice (C): click a clip to cut it in two there",
    ),
    (
        Tool::Select,
        Icon::Select,
        "Select (E): drag a box around clips; click empty space to clear",
    ),
    (
        Tool::Zoom,
        Icon::Zoom,
        "Zoom (Z): click to zoom in, Shift+click to zoom out",
    ),
    (
        Tool::Play,
        Icon::PlayFrom,
        "Play from here (Y): click to play from there",
    ),
];

/// The playlist window: arrange patterns, audio and automation as clips.
pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.playlist;
    if !open {
        return;
    }
    state.playlist.meter = frame.project.song_meter();
    let area = state.workspace;
    let id = egui::Id::new("playlist-window");
    // Across the whole lower part of the workspace, a little over half its
    // height.
    let top = area.top() + area.height() * 0.42;
    let default = Rect::from_min_max(
        pos2(area.left() + 4.0, top),
        pos2(area.right() - 4.0, area.bottom() - 4.0),
    );
    let window = crate::chrome::window(ctx, id, "Playlist", area, default, true)
        .default_height(default.height())
        .min_width(360.0)
        .min_height(140.0)
        .show(ctx, |ui| {
            let mut data = (&mut *state, &mut *actions);
            let folded = crate::chrome::Header::new(id, "Playlist — Arrangement").show(
                ui,
                &mut open,
                &mut data,
                |ui, (state, actions)| window_menu(ui, state, frame, actions),
                |ui, (state, _)| tools(ui, state),
            );
            if folded {
                return;
            }
            ui.horizontal_top(|ui| {
                pattern_list(ui, state, frame.project, actions);
                canvas(ui, state, frame, actions);
            });
        });
    crate::chrome::remember(ctx, id, window.as_ref().map(|w| &w.response));
    state.windows.playlist = open;
}

/// The header's tools: the snap button, then the editing tools.
fn tools(ui: &mut egui::Ui, state: &mut UiState) {
    snap_button(ui, state);
    ui.add_space(6.0);
    for (tool, icon, hint) in TOOLS {
        if icons::button(ui, icon, state.playlist.tool == tool, theme::ACCENT, hint).clicked() {
            state.playlist.tool = tool;
        }
    }
}

/// The snap glyph and its current value; a click lists the values.
fn snap_button(ui: &mut egui::Ui, state: &mut UiState) {
    let popup = egui::Id::new("playlist-snap");
    let open = egui::Popup::is_id_open(ui.ctx(), popup);
    let hint = format!("Snap: {} · click to choose", state.playlist.snap.label());
    let response = ui
        .horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            let glyph = icons::button(ui, Icon::Snap, open, theme::ACCENT, &hint);
            let value = ui.add(
                egui::Label::new(
                    RichText::new(state.playlist.snap.label())
                        .size(theme::FONT_LABEL)
                        .color(theme::TEXT_DIM),
                )
                .sense(Sense::click()),
            );
            glyph.union(value)
        })
        .inner;
    let toggle = response.clicked().then_some(egui::SetOpenCommand::Toggle);
    egui::Popup::menu(&response)
        .id(popup)
        .open_memory(toggle)
        .show(|ui| snap_list(ui, state));
}

/// The snap values, the current one ticked.
fn snap_list(ui: &mut egui::Ui, state: &mut UiState) {
    for snap in Snap::ALL {
        if ui
            .selectable_label(state.playlist.snap == snap, snap.label())
            .clicked()
        {
            state.playlist.snap = snap;
            ui.close();
        }
    }
}

/// The playlist's own window-menu items.
fn window_menu(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    let selected = selected_clips(state, project);
    ui.menu_button("Edit", |ui| {
        menu_item(
            ui,
            actions,
            "Duplicate (Ctrl+B)",
            "Copy the selected clips right after their end",
            duplication(project, &selected),
        );
        let removal = removal(&selected);
        if removal.is_some() && ui.button("Delete (Del)").clicked() {
            actions.extend(removal.map(Action::Run));
            state.playlist.selection.clear();
            ui.close();
        }
        ui.separator();
        time_items(ui, project, actions);
    });
    ui.menu_button("Clip", |ui| clip_menu(ui, state, frame, actions));
    ui.menu_button("Select", |ui| {
        if ui.button("All (Ctrl+A)").clicked() {
            state.playlist.selection = project
                .playlist
                .clips
                .iter()
                .map(|c| (c.track, c.start))
                .collect();
            ui.close();
        }
        if ui.button("None (Esc)").clicked() {
            state.playlist.selection.clear();
            ui.close();
        }
    });
    ui.menu_button("Snap", |ui| snap_list(ui, state));
    ui.menu_button("Zoom", |ui| {
        if ui.button("Zoom in").hint("Ctrl+wheel also zooms").clicked() {
            state.playlist.px_per_bar = (state.playlist.px_per_bar * 1.25).min(BAR_WIDTH_RANGE.1);
        }
        if ui.button("Zoom out").clicked() {
            state.playlist.px_per_bar = (state.playlist.px_per_bar / 1.25).max(BAR_WIDTH_RANGE.0);
        }
        if ui.button("Fit the song").clicked() {
            if let Some((_, view)) = state.playlist.last_view {
                let bars = project
                    .song_meter()
                    .bar(project.playlist.length_ticks())
                    .index
                    .max(1);
                let width = view.width() - HEADER_WIDTH - 12.0;
                state.playlist.px_per_bar =
                    (width / bars as f32).clamp(BAR_WIDTH_RANGE.0, BAR_WIDTH_RANGE.1);
            }
            ui.close();
        }
    });
    ui.menu_button("Markers", |ui| {
        if ui.button("Add marker at the playhead (Alt+T)").clicked() {
            actions.push(Action::Run(Invocation::bare("ui.add_marker")));
            ui.close();
        }
    });
}

/// The clips in the playlist selection.
fn selected_clips(state: &UiState, project: &Project) -> Vec<Clip> {
    project
        .playlist
        .clips
        .iter()
        .filter(|c| state.playlist.selection.contains(&(c.track, c.start)))
        .copied()
        .collect()
}

/// Removes `selected` as one undo step.
fn removal(selected: &[Clip]) -> Option<Invocation> {
    batch(
        selected
            .iter()
            .map(|c| {
                Invocation::new(
                    "playlist.remove_clip",
                    [(c.track + 1).to_string(), format!("{}t", c.start)],
                )
            })
            .collect(),
    )
}

/// How far a duplicate of `selected` moves: from its first start to its end.
fn duplicate_span(selected: &[Clip]) -> u32 {
    let first = selected.iter().map(|c| c.start).min().unwrap_or(0);
    let end = selected.iter().map(Clip::end).max().unwrap_or(first);
    end - first
}

/// Copies `selected` right after its end, as one undo step.
fn duplication(project: &Project, selected: &[Clip]) -> Option<Invocation> {
    let span = duplicate_span(selected);
    batch(
        selected
            .iter()
            .map(|c| add_clip_invocation(project, c, c.track, c.start + span))
            .collect(),
    )
}

fn pattern_list(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    state.playlist.picker_items.clear();
    ui.vertical(|ui| {
        ui.set_width(LIST_WIDTH);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (picker, label, hint) in [
                (Picker::All, "All", "Patterns, audio and automation clips"),
                (Picker::Patterns, "Pat", "Patterns"),
                (Picker::Audio, "Aud", "Audio clips"),
                (Picker::Automation, "Aut", "Automation clips"),
            ] {
                ui.selectable_value(
                    &mut state.playlist.picker,
                    picker,
                    RichText::new(label).small(),
                )
                .hint(hint);
            }
        });
        let picker = state.playlist.picker;
        if matches!(picker, Picker::All | Picker::Patterns) {
            pattern_section(ui, state, project, actions);
        }
        if matches!(picker, Picker::All | Picker::Audio) {
            audio_section(ui, state, project);
        }
        if matches!(picker, Picker::All | Picker::Automation) {
            automation_section(ui, state, project, actions);
        }
    });
}

/// What the picker lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Picker {
    All,
    Patterns,
    Audio,
    Automation,
}

/// Channels that have audio clips; clicking one paints its clips.
fn audio_section(ui: &mut egui::Ui, state: &mut UiState, project: &Project) {
    let mut sources: Vec<musiccraft_project::ChannelId> = Vec::new();
    for clip in &project.playlist.clips {
        if let ClipSource::Audio(id) = clip.source {
            if !sources.contains(&id) {
                sources.push(id);
            }
        }
    }
    ui.add_space(8.0);
    ui.label(
        RichText::new("AUDIO")
            .small()
            .strong()
            .color(theme::TEXT_DIM),
    );
    if sources.is_empty() {
        ui.label(
            RichText::new("Drop a sound on the playlist")
                .small()
                .color(theme::TEXT_DIM),
        );
    }
    for id in sources {
        let Some(index) = project.channel_position(id) else {
            continue;
        };
        let selected = state.playlist.paint_audio == Some(id);
        let text = RichText::new(format!("♪ {}", project.channels[index].name))
            .color(theme::palette(index));
        let response = ui
            .selectable_label(selected, text)
            .hint("Click to paint this audio clip on the playlist");
        if response.clicked() {
            state.playlist.paint_audio = Some(id);
            state.playlist.paint_automation = None;
        }
    }
}

fn pattern_section(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    {
        ui.label(
            RichText::new("PATTERNS")
                .small()
                .strong()
                .color(theme::TEXT_DIM),
        );
        // A solid scroll bar takes its own strip: a floating one would lie
        // over the right edge of the miniatures and catch their clicks.
        ui.scope(|ui| {
            ui.spacing_mut().scroll = egui::style::ScrollStyle::solid();
            egui::ScrollArea::vertical()
                .id_salt("pattern-list")
                .max_height(260.0)
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    for (i, pattern) in project.patterns.iter().enumerate() {
                        let selected = i == project.current_pattern;
                        let painting = state.playlist.paint_automation.is_none()
                            && state.playlist.paint_audio.is_none();
                        let response = miniature(ui, project, pattern, selected && painting)
                            .hint("Click to paint this pattern; right-click for options");
                        state.playlist.picker_items.push(response.rect);
                        if response.clicked() {
                            state.playlist.paint_automation = None;
                            state.playlist.paint_audio = None;
                            if !selected {
                                actions.push(Action::Run(Invocation::new(
                                    "pattern.select",
                                    [(i + 1).to_string()],
                                )));
                            }
                        }
                        response.context_menu(|ui| {
                            if ui.button("Rename…").clicked() {
                                state.open_dialog(Dialog::rename_pattern(i, &pattern.name));
                                ui.close();
                            }
                            if ui.button("Clone").clicked() {
                                actions.push(Action::Run(Invocation::new(
                                    "pattern.clone",
                                    [(i + 1).to_string()],
                                )));
                                ui.close();
                            }
                            ui.menu_button("Colour", |ui| {
                                let mut pick = |ui: &mut egui::Ui, text: RichText, arg: String| {
                                    if ui.button(text).clicked() {
                                        actions.push(Action::Run(Invocation::new(
                                            "pattern.set_color",
                                            [(i + 1).to_string(), arg],
                                        )));
                                        ui.close();
                                    }
                                };
                                pick(ui, RichText::new("None"), "none".to_owned());
                                for c in 0..16u8 {
                                    let text = RichText::new(format!("■ Colour {}", c + 1))
                                        .color(theme::track_color(c));
                                    pick(ui, text, (c + 1).to_string());
                                }
                            });
                            let sig = pattern.time_signature;
                            ui.menu_button(format!("Time signature ({sig})…"), |ui| {
                                if let Some(new) =
                                    signature::picker(ui, Some(sig), &mut state.sig_draft)
                                {
                                    actions.push(Action::Run(Invocation::new(
                                        "pattern.set_time_signature",
                                        [(i + 1).to_string(), new.to_string()],
                                    )));
                                }
                            });
                            if !pattern.automation.is_empty()
                                && ui
                                    .button(format!(
                                        "Clear automation ({} lanes)",
                                        pattern.automation.len()
                                    ))
                                    .on_hover_text(
                                        "Remove the knob moves recorded into this pattern",
                                    )
                                    .clicked()
                            {
                                actions.push(Action::Run(Invocation::new(
                                    "pattern.clear_events",
                                    [(i + 1).to_string()],
                                )));
                                ui.close();
                            }
                            ui.add_enabled_ui(project.patterns.len() > 1, |ui| {
                                if ui
                                    .button(RichText::new("Delete").color(theme::DANGER))
                                    .clicked()
                                {
                                    actions.push(Action::Run(Invocation::new(
                                        "pattern.remove",
                                        [(i + 1).to_string()],
                                    )));
                                    ui.close();
                                }
                            });
                        });
                    }
                });
        });
        if ui.button("+ New pattern").clicked() {
            state.playlist.paint_automation = None;
            state.playlist.paint_audio = None;
            actions.push(Action::Run(Invocation::bare("pattern.add")));
        }
    }
}

/// Height of a pattern's miniature in the picker.
const MINIATURE_HEIGHT: f32 = 34.0;

/// A pattern drawn as a small clip: its name over its notes.
fn miniature(
    ui: &mut egui::Ui,
    project: &Project,
    pattern: &musiccraft_project::Pattern,
    selected: bool,
) -> egui::Response {
    let width = ui.available_width();
    let (rect, response) =
        ui.allocate_exact_size(vec2(width, MINIATURE_HEIGHT), egui::Sense::click());
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let color = crate::pattern_color(pattern).unwrap_or(UNCOLOURED_CLIP);
    painter.rect_filled(rect, 4.0, color.gamma_multiply(0.8));
    let title = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + TITLE_HEIGHT));
    painter.rect_filled(
        title,
        egui::CornerRadius {
            nw: 4,
            ne: 4,
            sw: 0,
            se: 0,
        },
        theme::shade(70),
    );
    painter.text(
        pos2(title.left() + 4.0, title.center().y),
        Align2::LEFT_CENTER,
        &pattern.name,
        FontId::proportional(10.0),
        theme::TEXT,
    );
    let body = Rect::from_min_max(
        pos2(rect.left() + 3.0, title.bottom() + 2.0),
        pos2(rect.right() - 3.0, rect.bottom() - 2.0),
    );
    if pattern.notes.is_empty() && pattern.automation.is_empty() {
        painter.text(
            body.center(),
            Align2::CENTER_CENTER,
            "empty",
            FontId::proportional(9.0),
            theme::TEXT_DIM,
        );
    } else {
        lane_thumbnail(&painter, pattern, body);
        pattern_thumbnail(&painter, project, pattern, body);
    }
    let edge = if selected {
        Stroke::new(1.5, theme::ACCENT)
    } else if response.hovered() {
        Stroke::new(1.0, theme::TEXT_DIM)
    } else {
        Stroke::new(1.0, theme::shade(140))
    };
    painter.rect_stroke(rect, 4.0, edge, egui::StrokeKind::Inside);
    response
}

fn automation_section(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    {
        if !project.automations.is_empty() {
            ui.add_space(8.0);
            ui.label(
                RichText::new("AUTOMATION")
                    .small()
                    .strong()
                    .color(theme::TEXT_DIM),
            );
            for (i, automation) in project.automations.iter().enumerate() {
                let selected = state.playlist.paint_automation == Some(automation.id);
                let text = RichText::new(format!("~ {}", automation.name)).color(theme::AUTOMATION);
                let response = ui
                    .selectable_label(selected, text)
                    .hint("Click to paint it on the playlist · double-click to edit");
                if response.clicked() {
                    state.playlist.paint_automation = Some(automation.id);
                    state.playlist.paint_audio = None;
                }
                if response.double_clicked() {
                    state.automation_editor = Some(automation.id);
                }
                response.context_menu(|ui| {
                    if ui.button("Edit…").clicked() {
                        state.automation_editor = Some(automation.id);
                        ui.close();
                    }
                    if ui.button("Rename…").clicked() {
                        state.open_dialog(Dialog::rename_automation(i, &automation.name));
                        ui.close();
                    }
                    if ui
                        .button(RichText::new("Delete").color(theme::DANGER))
                        .clicked()
                    {
                        actions.push(Action::Run(Invocation::new(
                            "automation.remove",
                            [(i + 1).to_string()],
                        )));
                        ui.close();
                    }
                });
            }
        }
    }
}

fn canvas(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let px_per_tick = state.playlist.px_per_bar / ZOOM_TICKS as f32;
    let row_h = state.playlist.row_height;
    let meter = state.playlist.meter.clone();
    let bars = (meter.bar(project.playlist.length_ticks()).index + 16).max(32);
    let size = vec2(
        HEADER_WIDTH + meter.bar_start(bars) as f32 * px_per_tick,
        RULER_HEIGHT + PLAYLIST_TRACKS as f32 * row_h,
    );
    // The scroll/zoom row runs along the top and the vertical bar down the
    // right; the view (ruler, track headers and lanes) fills the rest.
    let body = ui.available_rect_before_wrap();
    ui.advance_cursor_after_rect(body);
    let mut view_ui =
        ui.new_child(egui::UiBuilder::new().max_rect(crate::scrollbars::view_rect(body)));
    let mut scroll = egui::ScrollArea::both()
        .id_salt("playlist-scroll")
        .auto_shrink([false, false])
        .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    if let Some(jump) = state.playlist.jump.take() {
        scroll = scroll.scroll_offset(jump);
    }
    let output = scroll.show(&mut view_ui, |ui| {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let clip = ui.clip_rect();
        let grid = Grid {
            origin: pos2(rect.left() + HEADER_WIDTH, rect.top() + RULER_HEIGHT),
            px_per_tick,
            row_height: row_h,
            rows: PLAYLIST_TRACKS,
        };
        state.playlist.last_view = Some((grid, clip));
        let painter = ui.painter_at(clip);
        draw_grid(&painter, &grid, (clip, rect), &meter);

        let clips = &project.playlist.clips;
        let moving = moving_clips(state, clips);
        let (dt, dtrack, dl) = drag_delta(state, &moving);
        for clip_item in clips {
            let is_moving = moving.contains(clip_item);
            let erasing = state.playlist.erased.contains(clip_item);
            if !is_moving && !erasing {
                let selected = state
                    .playlist
                    .selection
                    .contains(&(clip_item.track, clip_item.start));
                draw_clip(
                    &painter,
                    &grid,
                    frame,
                    clip_item,
                    (clip_item.track, clip_item.start, clip_item.length),
                    if selected {
                        Look::Selected
                    } else {
                        Look::Normal
                    },
                );
            }
        }
        // Clips being painted, drawn as the current pattern.
        let pattern_id = project.current_pattern().id;
        for (track, start, length) in &state.playlist.painted {
            let preview = Clip {
                source: ClipSource::Pattern(pattern_id),
                track: *track,
                start: *start,
                length: *length,
                offset: 0,
                muted: false,
            };
            draw_clip(
                &painter,
                &grid,
                frame,
                &preview,
                (*track, *start, *length),
                Look::Ghost,
            );
        }
        for c in &moving {
            let (track, start) = shifted(c, dt, dtrack);
            let length = u32::try_from((i64::from(c.length) + dl).max(1)).unwrap_or(1);
            draw_clip(
                &painter,
                &grid,
                frame,
                c,
                (track, start, length),
                Look::Ghost,
            );
        }
        if let Some((origin, current)) = state.playlist.marquee {
            let area = Rect::from_two_pos(origin, current);
            painter.rect_filled(area, 0.0, theme::SELECTION_FILL);
            painter.rect_stroke(
                area,
                0.0,
                Stroke::new(1.0, theme::ACCENT),
                egui::StrokeKind::Inside,
            );
        }
        if frame.engine.playing && project.play_mode == PlayMode::Song {
            let x = grid.x(frame.engine.position_ticks as f32);
            painter.line_segment(
                [pos2(x, clip.top()), pos2(x, clip.bottom())],
                Stroke::new(1.5, theme::PLAYHEAD),
            );
            // Follow the playhead when it leaves the view.
            let visible = clip.left() + HEADER_WIDTH..clip.right() - 40.0;
            if !visible.contains(&x) && state.playlist.drag.is_none() {
                let target = Rect::from_min_size(
                    pos2(x - HEADER_WIDTH - 20.0, clip.top()),
                    vec2(clip.width() - 60.0, 1.0),
                );
                ui.scroll_to_rect(target, Some(egui::Align::Min));
            }
        }
        let boxes: Vec<(usize, Rect)> = clips
            .iter()
            .enumerate()
            .map(|(i, c)| (i, grid.span(c.track, c.start, c.length)))
            .collect();
        edit(ui, state, project, &grid, &response, clip, &boxes, actions);
        draw_ruler(
            &painter,
            &grid,
            clip,
            project,
            state.playlist.loop_drag,
            &meter,
        );
        ruler(ui, state, project, &grid, clip, actions);
        headers(ui, state, project, &grid, clip, actions);
        if response.hovered() {
            let zoom = ui.input(|i| i.zoom_delta());
            if zoom != 1.0 {
                state.playlist.px_per_bar =
                    (state.playlist.px_per_bar * zoom).clamp(BAR_WIDTH_RANGE.0, BAR_WIDTH_RANGE.1);
            }
        }
    });
    state.playlist.offset = output.state.offset;
    let total = meter.bar_start(bars) as f32;
    bars_around(ui, state, project, body, (output.inner_rect.size(), total));
}

/// The shared scroll/zoom row and vertical bar around the playlist's view.
fn bars_around(
    ui: &egui::Ui,
    state: &mut UiState,
    project: &Project,
    body: Rect,
    (view, total): (Vec2, f32),
) {
    use crate::scrollbars::{self, Setup};
    let list = &mut state.playlist;
    let zoom = ZOOM_TICKS as f32;
    let (lo, hi) = BAR_WIDTH_RANGE;
    let setup = Setup {
        view,
        offset: list.offset,
        lead: HEADER_WIDTH,
        top: RULER_HEIGHT,
        tail: 0.0,
        x: (
            list.px_per_bar / zoom,
            total,
            list.meter.bar_start(1) as f32,
            (lo / zoom, hi / zoom),
        ),
        y: (
            list.row_height,
            PLAYLIST_TRACKS as f32,
            TRACK_HEIGHT_RANGE,
            TRACK_HEIGHT,
        ),
        y_label: "Track height",
    };
    let out = scrollbars::around(
        ui,
        body,
        egui::Id::new("playlist-bars"),
        setup,
        |p, rail, span| {
            overview(p, rail, project, span);
        },
    );
    if let Some(scale) = out.scale {
        list.px_per_bar = scale * zoom;
    }
    if let Some(row) = out.row {
        list.row_height = row;
    }
    if out.jump.is_some() {
        list.jump = out.jump;
    }
    // Above the track headers, beside the scroll row.
    let corner = Rect::from_min_size(body.min, vec2(HEADER_WIDTH, scrollbars::BAR));
    ui.painter().rect_filled(corner, 0.0, theme::PANEL_DARK);
}

/// A miniature of the song along the scroll rail: one dash per clip in its
/// colour, brighter inside the visible range.
fn overview(painter: &egui::Painter, rail: Rect, project: &Project, view: crate::scrollbars::Span) {
    let total = view.total.max(1.0);
    // Spread the tracks that hold clips over the rail's height.
    let used = project
        .playlist
        .clips
        .iter()
        .map(|c| c.track + 1)
        .max()
        .unwrap_or(1)
        .max(4) as f32;
    for clip in &project.playlist.clips {
        let (color, _, _) = clip_identity(project, clip);
        let x0 = rail.left() + clip.start as f32 / total * rail.width();
        let w = (clip.length as f32 / total * rail.width()).max(1.5);
        let y = rail.top() + (clip.track as f32 + 0.5) / used * rail.height();
        let inside = (clip.start as f32) < view.first + view.span
            && (clip.start + clip.length) as f32 > view.first;
        let tone = theme::at_strength(color, if inside { 220 } else { 110 });
        painter.line_segment([pos2(x0, y), pos2(x0 + w, y)], Stroke::new(1.5, tone));
    }
}

/// Bars in one bar group: the unit of the shading and the strongest lines.
const BAR_GROUP: u32 = 4;

/// One lane tone with track separators, alternate bar groups washed a
/// little lighter, and step, beat, bar and group lines getting stronger in
/// that order.
fn draw_grid(painter: &egui::Painter, grid: &Grid, (clip, rect): (Rect, Rect), meter: &Meter) {
    let lanes = Rect::from_min_max(grid.origin, rect.max);
    painter.rect_filled(lanes, 0.0, theme::LANE);
    let first = grid.tick_at(clip.left().max(grid.origin.x)).max(0.0) as u32;
    let last = grid.tick_at(clip.right()) as u32 + 1;
    let (top, bottom) = (grid.origin.y, rect.bottom());
    let first_bar = meter.bar(first).index;
    let last_bar = meter.bar(last).index;
    for bar in (first_bar..=last_bar).filter(|b| (b / BAR_GROUP) % 2 == 1 && b % BAR_GROUP == 0) {
        let x0 = grid.x(meter.bar_start(bar) as f32);
        let x1 = grid.x(meter.bar_start(bar + BAR_GROUP) as f32);
        painter.rect_filled(
            Rect::from_min_max(pos2(x0, top), pos2(x1, bottom)),
            0.0,
            theme::BAR_GROUP,
        );
    }
    let step = PPQ / 4;
    if grid.px_per_tick * step as f32 >= 6.0 {
        let mut tick = first - first % step;
        while tick <= last {
            let x = grid.x(tick as f32);
            painter.line_segment(
                [pos2(x, top), pos2(x, bottom)],
                Stroke::new(1.0, theme::GRID_LINE),
            );
            tick += step;
        }
    }
    let beat_px = grid.px_per_tick * PPQ as f32;
    for beat in meter.beats(first, last) {
        let color = if !beat.downbeat {
            if beat_px < 5.0 {
                continue;
            }
            theme::GRID_BEAT
        } else if beat.bar % BAR_GROUP == 0 {
            theme::GRID_GROUP
        } else {
            theme::GRID_BAR
        };
        let x = grid.x(beat.tick as f32);
        painter.line_segment([pos2(x, top), pos2(x, bottom)], Stroke::new(1.0, color));
    }
    let first_row = grid.row_at(clip.top().max(top)).unwrap_or(0);
    let last_row = grid
        .row_at(clip.bottom() - 1.0)
        .unwrap_or(PLAYLIST_TRACKS - 1);
    for row in first_row..=last_row {
        let y = grid.y(row) + grid.row_height - 0.5;
        painter.line_segment(
            [pos2(grid.origin.x, y), pos2(rect.right(), y)],
            Stroke::new(1.0, theme::GRID_LINE),
        );
    }
}

/// How a clip is drawn.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Normal,
    Selected,
    /// Being moved or painted.
    Ghost,
}

/// A clip at `(track, start, length)`: a title strip in its colour with the
/// kind glyph and name, over a dark body tinted toward that colour that
/// holds the content.
fn draw_clip(
    painter: &egui::Painter,
    grid: &Grid,
    frame: &Frame<'_>,
    clip: &Clip,
    (track, start, length): (u32, u32, u32),
    look: Look,
) {
    let project = frame.project;
    let (color, name, glyph) = clip_identity(project, clip);
    let rect = grid.span(track, start, length).shrink2(vec2(0.5, 1.0));
    let muted = clip.muted || !project.playlist.track_audible(track);
    let strength = match look {
        Look::Ghost => 0.55,
        _ if muted => 0.3,
        _ => 1.0,
    };
    let faded = |c: Color32| theme::at_strength(c, (f32::from(c.a()) * strength) as u8);
    let selected = look == Look::Selected;
    let body_fill = theme::clip_body(color);
    let title_h = TITLE_HEIGHT.min(rect.height());
    let title = Rect::from_min_max(rect.min, pos2(rect.right(), rect.top() + title_h));
    let has_body = rect.height() >= title_h + 6.0;
    if has_body {
        painter.rect_filled(rect, CLIP_RADIUS, faded(body_fill));
    }
    let strip = if selected {
        theme::CLIP_SELECTED
    } else {
        color
    };
    let strip_radius = if has_body {
        egui::CornerRadius {
            nw: CLIP_RADIUS as u8,
            ne: CLIP_RADIUS as u8,
            sw: 0,
            se: 0,
        }
    } else {
        egui::CornerRadius::same(CLIP_RADIUS as u8)
    };
    painter.rect_filled(title, strip_radius, faded(strip));
    let edge = if selected {
        theme::CLIP_SELECTED
    } else {
        theme::mix(theme::mix(theme::LANE, color, 0.22), theme::BACKGROUND, 0.5)
    };
    painter.rect_stroke(
        rect,
        CLIP_RADIUS,
        Stroke::new(1.0, faded(edge)),
        egui::StrokeKind::Inside,
    );
    if has_body {
        let body = Rect::from_min_max(
            pos2(rect.left() + 2.0, title.bottom() + 1.0),
            pos2(rect.right() - 2.0, rect.bottom() - 2.0),
        );
        clip_content(
            painter,
            grid,
            frame,
            clip,
            (start, length),
            body,
            &faded,
            color,
        );
    }
    let text = faded(theme::text_on(strip));
    let strip_painter = painter.with_clip_rect(title.intersect(painter.clip_rect()));
    let glyph_rect = Rect::from_min_size(title.min + vec2(1.0, 0.0), vec2(title_h, title_h));
    icons::paint(&strip_painter, glyph_rect, glyph, text);
    strip_painter.text(
        pos2(glyph_rect.right() + 1.0, title.center().y),
        Align2::LEFT_CENTER,
        name,
        FontId::proportional(theme::FONT_CAPTION + 0.5),
        text,
    );
}

/// A clip's colour, name and kind glyph.
fn clip_identity<'a>(project: &'a Project, clip: &Clip) -> (Color32, &'a str, Icon) {
    match clip.source {
        ClipSource::Pattern(id) => {
            let pattern = project
                .pattern_position(id)
                .and_then(|i| project.patterns.get(i));
            (
                pattern
                    .and_then(crate::pattern_color)
                    .unwrap_or(UNCOLOURED_CLIP),
                pattern.map_or("", |p| p.name.as_str()),
                Icon::Pattern,
            )
        }
        ClipSource::Automation(id) => (
            theme::AUTOMATION,
            project
                .automation_position(id)
                .map_or("", |i| project.automations[i].name.as_str()),
            Icon::Automation,
        ),
        ClipSource::Audio(id) => {
            let index = project.channel_position(id).unwrap_or(0);
            (
                theme::palette(index),
                project.channels.get(index).map_or("", |c| c.name.as_str()),
                Icon::Wave,
            )
        }
    }
}

/// The content in a clip's body: notes, the automation curve with a faint
/// fill under it, or the waveform in a light tint of the clip colour.
#[allow(clippy::too_many_arguments)]
fn clip_content(
    painter: &egui::Painter,
    grid: &Grid,
    frame: &Frame<'_>,
    clip: &Clip,
    (start, length): (u32, u32),
    body: Rect,
    faded: &dyn Fn(Color32) -> Color32,
    color: Color32,
) {
    let project = frame.project;
    match clip.source {
        ClipSource::Pattern(id) => {
            if let Some(pattern) = project
                .pattern_position(id)
                .and_then(|i| project.patterns.get(i))
            {
                pattern_preview(painter, grid, project, pattern, body, (start, length));
            }
        }
        ClipSource::Automation(id) => {
            let Some(automation) = project
                .automation_position(id)
                .map(|i| &project.automations[i])
            else {
                return;
            };
            let steps = (body.width() / 3.0).clamp(2.0, 400.0) as usize;
            let points: Vec<Pos2> = (0..=steps)
                .map(|i| {
                    let tick = i as f32 / steps as f32 * length as f32;
                    let value = automation.value_at(f64::from(tick)).unwrap_or(0.0);
                    pos2(
                        grid.x(start as f32 + tick),
                        body.bottom() - value * body.height(),
                    )
                })
                .collect();
            let fill = faded(theme::at_strength(theme::AUTOMATION, 72));
            for pair in points.windows(2) {
                let [a, b] = [pair[0], pair[1]];
                painter.add(egui::Shape::convex_polygon(
                    vec![a, b, pos2(b.x, body.bottom()), pos2(a.x, body.bottom())],
                    fill,
                    Stroke::NONE,
                ));
            }
            painter.add(egui::Shape::line(
                points,
                Stroke::new(1.5, faded(theme::AUTOMATION)),
            ));
        }
        ClipSource::Audio(id) => {
            let Some((_, peaks)) = frame.clip_waveforms.iter().find(|(c, _)| *c == id) else {
                return;
            };
            let n = peaks.len().max(1);
            let half = body.height() / 2.0;
            let peak = peaks
                .iter()
                .fold(1e-6_f32, |p, (lo, hi)| p.max(lo.abs()).max(hi.abs()));
            // The waveform is drawn at the sample's real duration.
            let duration = frame
                .clip_durations
                .iter()
                .find(|(c, _)| *c == id)
                .map_or(length, |(_, d)| *d);
            let stroke = Stroke::new(1.0, faded(theme::clip_content(color)));
            for (i, (lo, hi)) in peaks.iter().enumerate() {
                let tick = i as f32 / n as f32 * duration as f32;
                if tick >= length as f32 {
                    break;
                }
                let x = grid.x(start as f32 + tick);
                painter.line_segment(
                    [
                        pos2(x, body.center().y - hi / peak * half),
                        pos2(x, body.center().y - lo / peak * half),
                    ],
                    stroke,
                );
            }
        }
    }
}

/// Height of a clip's title strip.
const TITLE_HEIGHT: f32 = 11.0;

/// Corner radius of clips: barely rounded.
const CLIP_RADIUS: f32 = 2.0;

/// Fill of a clip whose pattern has no colour of its own.
const UNCOLOURED_CLIP: Color32 = theme::CLIP_NEUTRAL;

/// How a preview places a pattern's notes vertically.
enum NoteRows {
    /// By key, from the lowest (bottom) to the highest.
    Keys { lo: u8, span: f32 },
    /// Every note has one key and several channels play: one row per
    /// channel, these channel positions in rack order from the top.
    Channels(Vec<usize>),
    /// Every note on one line.
    Single,
}

impl NoteRows {
    fn of(project: &Project, pattern: &musiccraft_project::Pattern) -> Self {
        let (lo, hi) = pattern
            .notes
            .iter()
            .fold((127u8, 0u8), |(lo, hi), n| (lo.min(n.key), hi.max(n.key)));
        if lo < hi {
            return Self::Keys {
                lo,
                span: f32::from(hi - lo),
            };
        }
        let mut channels: Vec<usize> = pattern
            .notes
            .iter()
            .filter_map(|n| project.channel_position(n.channel))
            .collect();
        channels.sort_unstable();
        channels.dedup();
        if channels.len() > 1 {
            Self::Channels(channels)
        } else {
            Self::Single
        }
    }

    /// The height of `note`'s line inside `body`.
    fn y(&self, project: &Project, note: &musiccraft_project::Note, body: Rect) -> f32 {
        match self {
            Self::Keys { lo, span } => {
                body.bottom()
                    - (f32::from(note.key.saturating_sub(*lo)) / span) * (body.height() - 2.0)
                    - 1.0
            }
            Self::Channels(rows) => {
                let row = project
                    .channel_position(note.channel)
                    .and_then(|c| rows.iter().position(|r| *r == c))
                    .unwrap_or(0);
                let pitch = body.height() / rows.len() as f32;
                body.top() + pitch * (row as f32 + 0.5)
            }
            Self::Single => body.center().y,
        }
    }
}

/// A note's colour, as in the piano roll: its colour group, or its channel's
/// colour for group "channel".
fn note_tint(project: &Project, note: &musiccraft_project::Note) -> Color32 {
    if note.props.color > 0 {
        theme::note_color(note.props.color)
    } else {
        theme::palette(project.channel_position(note.channel).unwrap_or(0))
    }
}

/// One note of a preview, from `x0` to `x1` at height `y`.
fn note_mark(painter: &egui::Painter, x0: f32, x1: f32, y: f32, tint: Color32) {
    // A dark edge keeps notes readable on a coloured clip.
    painter.line_segment(
        [pos2(x0, y), pos2(x1, y)],
        Stroke::new(3.0, theme::shade(110)),
    );
    painter.line_segment([pos2(x0, y), pos2(x1, y)], Stroke::new(1.5, tint));
}

/// The notes of `pattern` repeated across a clip at `start` lasting `length`,
/// each in its own colour.
fn pattern_preview(
    painter: &egui::Painter,
    grid: &Grid,
    project: &Project,
    pattern: &musiccraft_project::Pattern,
    body: Rect,
    (start, length): (u32, u32),
) {
    let period = pattern.length_ticks().max(1);
    let rows = NoteRows::of(project, pattern);
    let mut offset = 0;
    while offset < length {
        for note in &pattern.notes {
            let s = offset + note.start;
            if s >= length {
                continue;
            }
            let x0 = grid.x((start + s) as f32);
            let x1 = grid
                .x((start + (s + note.length).min(length)) as f32)
                .max(x0 + 1.5);
            let y = rows.y(project, note, body);
            note_mark(painter, x0, x1, y, note_tint(project, note));
        }
        offset += period;
    }
}

/// The automation recorded into `pattern`, as faint curves under its notes.
fn lane_thumbnail(painter: &egui::Painter, pattern: &musiccraft_project::Pattern, body: Rect) {
    let length = pattern.length_ticks().max(1) as f32;
    let color = theme::AUTOMATION.gamma_multiply(0.6);
    for lane in &pattern.automation {
        let points: Vec<Pos2> = lane
            .points
            .iter()
            .map(|p| {
                pos2(
                    body.left() + (p.tick as f32 / length).min(1.0) * body.width(),
                    body.bottom() - p.value.clamp(0.0, 1.0) * body.height(),
                )
            })
            .collect();
        match points[..] {
            [] => {}
            [only] => {
                painter.line_segment(
                    [pos2(body.left(), only.y), pos2(body.right(), only.y)],
                    Stroke::new(1.0, color),
                );
            }
            _ => {
                painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
            }
        }
    }
}

/// One pass of `pattern`'s notes fitted into `body`, for the picker.
fn pattern_thumbnail(
    painter: &egui::Painter,
    project: &Project,
    pattern: &musiccraft_project::Pattern,
    body: Rect,
) {
    let length = pattern.length_ticks().max(1) as f32;
    let x = |tick: u32| body.left() + (tick as f32 / length).min(1.0) * body.width();
    let rows = NoteRows::of(project, pattern);
    for note in &pattern.notes {
        let x0 = x(note.start);
        let x1 = x(note.start.saturating_add(note.length)).max(x0 + 1.5);
        let y = rows.y(project, note, body);
        note_mark(painter, x0, x1, y, note_tint(project, note));
    }
}

/// The command that places a clip like `clip` at another position.
fn add_clip_invocation(project: &Project, clip: &Clip, track: u32, start: u32) -> Invocation {
    let (name, number) = match clip.source {
        ClipSource::Pattern(id) => ("playlist.add_clip", project.pattern_position(id)),
        ClipSource::Automation(id) => (
            "playlist.add_automation_clip",
            project.automation_position(id),
        ),
        ClipSource::Audio(id) => ("playlist.add_audio_clip", project.channel_position(id)),
    };
    let mut args = vec![
        number.map_or(1, |i| i + 1).to_string(),
        (track + 1).to_string(),
        format!("{start}t"),
        format!("{}t", clip.length),
    ];
    if clip.offset > 0 {
        args.push(format!("{}t", clip.offset));
    }
    Invocation::new(name, args)
}

fn draw_ruler(
    painter: &egui::Painter,
    grid: &Grid,
    clip: Rect,
    project: &Project,
    dragging: Option<(u32, u32)>,
    meter: &Meter,
) {
    let ruler = Rect::from_min_max(
        pos2(clip.left(), clip.top()),
        pos2(clip.right(), clip.top() + RULER_HEIGHT),
    );
    painter.rect_filled(ruler, 0.0, theme::PANEL_DARK);
    let song_end = project.song_length_ticks();
    let end_x = grid.x(song_end as f32);
    if end_x > ruler.left() {
        painter.rect_filled(
            Rect::from_min_max(
                pos2(grid.origin.x.max(ruler.left()), ruler.bottom() - 3.0),
                pos2(end_x, ruler.bottom()),
            ),
            0.0,
            theme::ACCENT_DIM,
        );
    }
    let range = dragging
        .map(|(a, b)| (a.min(b), a.max(b)))
        .or(project.playlist.loop_range);
    if let Some((start, end)) = range {
        let band = Rect::from_min_max(
            pos2(
                grid.x(start as f32).max(ruler.left() + HEADER_WIDTH),
                ruler.top(),
            ),
            pos2(grid.x(end as f32), ruler.bottom()),
        );
        if band.width() > 0.0 {
            painter.rect_filled(band, 0.0, theme::at_strength(theme::HIGHLIGHT, 60));
            let shade = theme::at_strength(theme::HIGHLIGHT, 14);
            painter.rect_filled(
                Rect::from_min_max(
                    pos2(band.left(), ruler.bottom()),
                    pos2(band.right(), clip.bottom()),
                ),
                0.0,
                shade,
            );
            for x in [band.left(), band.right()] {
                painter.line_segment(
                    [pos2(x, ruler.top()), pos2(x, clip.bottom())],
                    Stroke::new(1.0, theme::HIGHLIGHT),
                );
            }
        }
    }
    let first = meter.bar(grid.tick_at(clip.left()) as u32);
    let last_bar = meter.bar(grid.tick_at(clip.right()) as u32).index + 1;
    let every = if grid.px_per_tick * first.len as f32 >= 28.0 {
        1
    } else {
        4
    };
    for bar in (first.index..=last_bar).filter(|b| b % every == 0) {
        let x = grid.x(meter.bar_start(bar) as f32);
        painter.text(
            pos2(x + 3.0, ruler.center().y),
            Align2::LEFT_CENTER,
            (bar + 1).to_string(),
            FontId::monospace(11.0),
            theme::TEXT,
        );
    }
    for marker in &project.playlist.markers {
        let x = grid.x(marker.tick as f32);
        if x < ruler.left() + HEADER_WIDTH || x > ruler.right() {
            continue;
        }
        let top = ruler.top();
        // Each action has its own glyph: a flag for a label, two bars for a
        // pause, a down-facing arrow for the song loop point, a bracket for a
        // time signature change.
        let color = match marker.action {
            MarkerAction::None => theme::AUTOMATION,
            MarkerAction::Pause => theme::DANGER,
            MarkerAction::SongLoop => theme::ACCENT,
            MarkerAction::TimeSignature(_) => theme::HIGHLIGHT,
        };
        match marker.action {
            MarkerAction::None => {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(x, top + 2.0),
                        pos2(x + 7.0, top + 6.0),
                        pos2(x, top + 10.0),
                    ],
                    color,
                    Stroke::NONE,
                ));
            }
            MarkerAction::Pause => {
                for dx in [0.0, 4.0] {
                    painter.rect_filled(
                        Rect::from_min_size(pos2(x + dx, top + 2.0), egui::vec2(2.5, 8.0)),
                        0.0,
                        color,
                    );
                }
            }
            MarkerAction::SongLoop => {
                painter.add(egui::Shape::convex_polygon(
                    vec![
                        pos2(x - 1.0, top + 2.0),
                        pos2(x + 8.0, top + 2.0),
                        pos2(x + 3.5, top + 10.0),
                    ],
                    color,
                    Stroke::NONE,
                ));
            }
            MarkerAction::TimeSignature(_) => {
                painter.rect_filled(
                    Rect::from_min_size(pos2(x, top + 2.0), egui::vec2(2.5, 9.0)),
                    0.0,
                    color,
                );
                painter.rect_filled(
                    Rect::from_min_size(pos2(x, top + 2.0), egui::vec2(7.0, 2.5)),
                    0.0,
                    color,
                );
            }
        }
        painter.line_segment(
            [pos2(x, top), pos2(x, clip.bottom())],
            Stroke::new(1.0, theme::at_strength(color, 70)),
        );
        painter.text(
            pos2(x + 10.0, top + 1.0),
            Align2::LEFT_TOP,
            &marker.name,
            FontId::proportional(10.5),
            color,
        );
    }
}

/// Clicking or dragging on the ruler moves the playhead.
/// Ruler gestures: click or drag to seek, Shift+drag to set the loop,
/// right-click for markers and the loop.
fn ruler(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    grid: &Grid,
    clip: Rect,
    actions: &mut Vec<Action>,
) {
    let snap = state.playlist.snap;
    let meter = state.playlist.meter.clone();
    let area = Rect::from_min_max(
        pos2(clip.left() + HEADER_WIDTH, clip.top()),
        pos2(clip.right(), clip.top() + RULER_HEIGHT),
    );
    let response = ui
        .interact(
            area,
            ui.id().with("playlist-ruler"),
            Sense::click_and_drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .hint("Click to seek · Shift+drag to loop · right-click for markers");
    let shift = ui.input(|i| i.modifiers.shift);
    let tick_at = |x: f32| grid.tick_at(x);
    let marker_at = |x: f32| {
        project.playlist.markers.iter().find(|m| {
            (grid.x(m.tick as f32) - x).abs() < 8.0
                || (x >= grid.x(m.tick as f32) && x < grid.x(m.tick as f32) + 60.0)
        })
    };
    if response.drag_started() && shift {
        if let Some(pos) = ui.input(|i| i.pointer.press_origin()) {
            let start = Snap::Bar.floor(tick_at(pos.x), &meter);
            state.playlist.loop_drag = Some((start, start));
        }
    }
    if let Some((start, end)) = &mut state.playlist.loop_drag {
        if let Some(pos) = response.interact_pointer_pos() {
            *end = Snap::Bar.round(tick_at(pos.x), &meter);
        }
        if response.drag_stopped() {
            let (a, b) = (*start.min(end), *start.max(end));
            if b > a {
                actions.push(Action::Run(Invocation::new(
                    "transport.set_loop",
                    [format!("{a}t"), format!("{b}t")],
                )));
            }
            state.playlist.loop_drag = None;
        }
    } else if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tick = match marker_at(pos.x).filter(|_| response.clicked()) {
                Some(marker) => marker.tick,
                None => snap
                    .floor(tick_at(pos.x), &meter)
                    .min(Snap::Beat.floor(tick_at(pos.x), &meter)),
            };
            actions.push(Action::Seek(f64::from(tick)));
        }
    }
    let press = response.interact_pointer_pos().or(response.hover_pos());
    response.context_menu(|ui| {
        let x = press.map_or(area.left(), |p| p.x);
        let tick = Snap::Bar.floor(tick_at(x), &meter);
        if ui
            .button("Add marker here…")
            .hint("Alt+T adds one at the playhead")
            .clicked()
        {
            state.open_dialog(crate::dialogs::Dialog::add_marker(tick));
            ui.close();
        }
        if ui
            .button("Add pause marker here")
            .hint("Song playback stops when it reaches this point")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                musiccraft_commands::BATCH,
                [
                    format!("playlist.add_marker {tick}t Pause"),
                    format!("playlist.set_marker_action {tick}t pause"),
                ],
            )));
            ui.close();
        }
        let here = meter.sig_at(tick);
        ui.menu_button("Time signature here…", |ui| {
            if tick == 0 {
                ui.label(
                    RichText::new("The start is the project signature (toolbar)")
                        .color(theme::TEXT_DIM),
                );
            }
            if let Some(sig) = signature::picker(ui, Some(here), &mut state.sig_draft) {
                actions.push(Action::Run(if tick == 0 {
                    Invocation::new("project.set_time_signature", [sig.to_string()])
                } else {
                    Invocation::new(
                        "playlist.set_time_signature",
                        [format!("{tick}t"), sig.to_string()],
                    )
                }));
            }
        })
        .response
        .hint("A new bar starts here in this signature");
        if let Some(marker) = marker_at(x) {
            ui.separator();
            ui.label(format!("Marker \"{}\"", marker.name));
            for action in MarkerAction::ALL {
                let (label, hint) = match action {
                    MarkerAction::None => ("Label only", "Only a label"),
                    MarkerAction::Pause => (
                        "Pause here",
                        "Song playback stops here; press play to continue",
                    ),
                    MarkerAction::SongLoop => (
                        "Loop the song here",
                        "At the end of the song, playback returns here instead of to the start",
                    ),
                    MarkerAction::TimeSignature(_) => continue,
                };
                if ui
                    .radio(marker.action == action, label)
                    .hint(hint)
                    .clicked()
                {
                    if marker.action != action {
                        actions.push(Action::Run(Invocation::new(
                            "playlist.set_marker_action",
                            [format!("{}t", marker.tick), action.name().to_owned()],
                        )));
                    }
                    ui.close();
                }
            }
            if ui
                .button(format!("Remove marker \"{}\"", marker.name))
                .clicked()
            {
                actions.push(Action::Run(Invocation::new(
                    "playlist.remove_marker",
                    [format!("{}t", marker.tick)],
                )));
                ui.close();
            }
        }
        if project.playlist.loop_range.is_some() && ui.button("Clear loop").clicked() {
            actions.push(Action::Run(Invocation::bare("transport.clear_loop")));
            ui.close();
        }
    });
}

fn headers(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    grid: &Grid,
    clip: Rect,
    actions: &mut Vec<Action>,
) {
    let area = Rect::from_min_max(
        pos2(clip.left(), clip.top() + RULER_HEIGHT),
        pos2(clip.left() + HEADER_WIDTH, clip.bottom()),
    );
    let painter = ui.painter_at(area);
    painter.rect_filled(area, 0.0, theme::PANEL);
    let first = grid.row_at(area.top()).unwrap_or(0);
    let last = grid
        .row_at(area.bottom() - 1.0)
        .unwrap_or(PLAYLIST_TRACKS - 1);
    for row in first..=last {
        let y = grid.y(row);
        let r = Rect::from_min_max(
            pos2(area.left(), y),
            pos2(area.right(), y + grid.row_height),
        );
        let playlist = &project.playlist;
        if let Some(&color) = playlist.track_colors.get(&row) {
            painter.rect_filled(
                Rect::from_min_size(r.left_top(), vec2(4.0, r.height())),
                0.0,
                theme::track_color(color),
            );
        }
        painter.line_segment(
            [r.left_bottom(), r.right_bottom()],
            Stroke::new(1.0, theme::GRID_LINE),
        );
        let text = if playlist.track_audible(row) {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        // The name stops short of the arm, solo and mute buttons at the
        // right.
        let name_area = Rect::from_min_max(
            pos2(r.left() + 8.0, r.top()),
            pos2(r.right() - 56.0, r.bottom()),
        );
        painter.with_clip_rect(name_area).text(
            name_area.left_center(),
            Align2::LEFT_CENTER,
            playlist.track_name(row),
            FontId::proportional(11.0),
            text,
        );
        let number = (row + 1).to_string();
        // Right-click the header: rename or colour the row.
        let header = ui.interact(r, ui.id().with(("track-header", row)), Sense::click());
        header.context_menu(|ui| {
            if ui.button("Rename…").clicked() {
                state.open_dialog(crate::dialogs::Dialog::rename_playlist_track(
                    row,
                    &playlist.track_name(row),
                ));
                ui.close();
            }
            ui.label("Colour");
            ui.horizontal_wrapped(|ui| {
                for c in 0..16u8 {
                    let (swatch, response) =
                        ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                    ui.painter().rect_filled(swatch, 2.0, theme::track_color(c));
                    if response.clicked() {
                        actions.push(Action::Run(Invocation::new(
                            "playlist.set_track_color",
                            [number.clone(), (c + 1).to_string()],
                        )));
                        ui.close();
                    }
                }
            });
            if ui.button("No colour").clicked() {
                actions.push(Action::Run(Invocation::new(
                    "playlist.set_track_color",
                    [number.clone(), "none".to_owned()],
                )));
                ui.close();
            }
        });
        let dot = Rect::from_center_size(r.right_center() - vec2(12.0, 0.0), vec2(14.0, 14.0));
        if !area.contains_rect(dot) {
            continue;
        }
        let muted = playlist.muted_tracks.contains(&row);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(dot));
        if widgets::dot(&mut child, !muted, theme::ACCENT)
            .hint("Mute track")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "playlist.toggle_track_mute",
                [number.clone()],
            )));
        }
        let arm = Rect::from_center_size(r.right_center() - vec2(46.0, 0.0), vec2(14.0, 14.0));
        let armed = playlist.armed_tracks.contains(&row);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(arm));
        if widgets::dot(&mut child, armed, theme::DANGER)
            .hint("Arm for audio recording: takes go to the first armed track with room")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "playlist.toggle_track_arm",
                [number.clone()],
            )));
        }
        let solo = Rect::from_center_size(r.right_center() - vec2(29.0, 0.0), vec2(16.0, 14.0));
        let soloed = playlist.soloed_tracks.contains(&row);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(solo));
        let label = RichText::new("S").small().color(if soloed {
            theme::HIGHLIGHT
        } else {
            theme::TEXT_DIM
        });
        if child
            .add(egui::Button::new(label).frame(false))
            .on_hover_text("Solo track")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "playlist.toggle_track_solo",
                [number],
            )));
        }
    }
}

fn moving_clips(state: &UiState, clips: &[Clip]) -> Vec<Clip> {
    let Some(drag) = state.playlist.drag else {
        return Vec::new();
    };
    let mut group: Vec<Clip> = clips
        .iter()
        .filter(|c| state.playlist.selection.contains(&(c.track, c.start)))
        .copied()
        .collect();
    if !group.contains(&drag.original) {
        group = vec![drag.original];
    }
    group
}

/// The (start, track, length) change of the drag in progress, clamped so no
/// moved clip leaves the playlist.
fn drag_delta(state: &UiState, group: &[Clip]) -> (i64, i64, i64) {
    let Some(drag) = state.playlist.drag else {
        return (0, 0, 0);
    };
    let o = drag.original;
    match drag.kind {
        DragKind::Move => {
            let (dt, dtrack) = clamp_delta(
                group,
                i64::from(drag.start) - i64::from(o.start),
                i64::from(drag.track) - i64::from(o.track),
            );
            (dt, dtrack, 0)
        }
        DragKind::Resize => (0, 0, i64::from(drag.length) - i64::from(o.length)),
    }
}

fn clamp_delta(group: &[Clip], dt: i64, dtrack: i64) -> (i64, i64) {
    let min_start = group.iter().map(|c| i64::from(c.start)).min().unwrap_or(0);
    let lo = group.iter().map(|c| i64::from(c.track)).min().unwrap_or(0);
    let hi = group.iter().map(|c| i64::from(c.track)).max().unwrap_or(0);
    (
        dt.max(-min_start),
        dtrack.clamp(-lo, i64::from(PLAYLIST_TRACKS) - 1 - hi),
    )
}

fn shifted(clip: &Clip, dt: i64, dtrack: i64) -> (u32, u32) {
    (
        u32::try_from(i64::from(clip.track) + dtrack).unwrap_or(0),
        u32::try_from(i64::from(clip.start) + dt).unwrap_or(0),
    )
}

/// `name <track> <start>t [extra…]` for each clip, as one undo step.
fn per_clip(clips: &[Clip], name: &str, extra: &[String]) -> Option<Invocation> {
    batch(
        clips
            .iter()
            .map(|c| {
                let mut args = vec![(c.track + 1).to_string(), format!("{}t", c.start)];
                args.extend(extra.iter().cloned());
                Invocation::new(name, args)
            })
            .collect(),
    )
}

/// Slices every clip that crosses `at` there.
pub(crate) fn slice_at(clips: &[Clip], at: u32) -> Option<Invocation> {
    batch(
        clips
            .iter()
            .filter(|c| c.start < at && at < c.end())
            .map(|c| {
                Invocation::new(
                    "playlist.slice_clip",
                    [(c.track + 1).to_string(), format!("{at}t")],
                )
            })
            .collect(),
    )
}

/// Merges the selected pattern clips (two or more) into one.
pub(crate) fn merge(clips: &[Clip]) -> Option<Invocation> {
    let patterns: Vec<&Clip> = clips
        .iter()
        .filter(|c| matches!(c.source, ClipSource::Pattern(_)))
        .collect();
    if patterns.len() < 2 {
        return None;
    }
    Some(Invocation::new(
        "playlist.merge_clips",
        patterns
            .iter()
            .flat_map(|c| [(c.track + 1).to_string(), format!("{}t", c.start)]),
    ))
}

/// A menu entry that runs `invocation`, greyed out without one.
fn menu_item(
    ui: &mut egui::Ui,
    actions: &mut Vec<Action>,
    label: &str,
    hint: &str,
    invocation: Option<Invocation>,
) {
    if ui
        .add_enabled(invocation.is_some(), egui::Button::new(label))
        .on_hover_text(hint)
        .clicked()
    {
        actions.extend(invocation.map(Action::Run));
        ui.close();
    }
}

/// The Clip menu: edits for the selected clips.
fn clip_menu(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let selected = selected_clips(state, project);
    let audio: Vec<Clip> = selected
        .iter()
        .filter(|c| matches!(c.source, ClipSource::Audio(_)))
        .copied()
        .collect();
    if selected.is_empty() {
        ui.label(
            RichText::new("Select clips first (Select tool or Ctrl+A)").color(theme::TEXT_DIM),
        );
    }
    let playhead = frame.engine.position_ticks.max(0.0) as u32;
    menu_item(
        ui,
        actions,
        "Slice at playhead",
        "Cut the selected clips where the playhead is (or use the slice tool)",
        slice_at(&selected, playhead),
    );
    menu_item(
        ui,
        actions,
        "Make independent copy",
        "Give each selected clip its own copy of its pattern, automation or sample channel",
        per_clip(&selected, "playlist.make_unique", &[]),
    );
    menu_item(
        ui,
        actions,
        "Merge",
        "Merge the selected pattern clips into one new pattern",
        merge(&selected),
    );
    menu_item(
        ui,
        actions,
        "Consolidate (render to audio)",
        "Render each selected clip to a WAV file and replace it with an audio clip",
        per_clip(
            &selected
                .iter()
                .filter(|c| !matches!(c.source, ClipSource::Automation(_)))
                .copied()
                .collect::<Vec<_>>(),
            "playlist.consolidate",
            &[],
        ),
    );
    menu_item(
        ui,
        actions,
        "Reverse",
        "Play the selected audio clips backwards (again: forwards)",
        per_clip(&audio, "playlist.reverse_clip", &[]),
    );
    if let Some(first) = audio.first() {
        let channel = match first.source {
            ClipSource::Audio(id) => project.channel_position(id),
            _ => None,
        };
        if let Some(channel) = channel.and_then(|i| project.channels.get(i)) {
            for (label, which, param) in [
                ("Fade in", "in", musiccraft_project::params::inst::FADE_IN),
                (
                    "Fade out",
                    "out",
                    musiccraft_project::params::inst::FADE_OUT,
                ),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    let mut seconds = channel.params.get(param);
                    let drag = ui.add(
                        egui::DragValue::new(&mut seconds)
                            .range(0.0..=10.0)
                            .speed(0.01)
                            .suffix(" s"),
                    );
                    if drag.drag_started() || drag.gained_focus() {
                        state.begin_gesture();
                    }
                    if drag.changed() {
                        let fades = per_clip(
                            &audio,
                            "playlist.set_clip_fade",
                            &[which.to_owned(), format!("{seconds:.3}")],
                        );
                        actions.extend(fades.map(|i| Action::Coalesced(i, state.gesture())));
                    }
                });
            }
        }
    }
}

/// Insert time and Remove time, over the loop region.
fn time_items(ui: &mut egui::Ui, project: &Project, actions: &mut Vec<Action>) {
    let space = |name: &str| {
        project.playlist.loop_range.map(|(start, end)| {
            Invocation::new(name, [format!("{start}t"), format!("{}t", end - start)])
        })
    };
    menu_item(
        ui,
        actions,
        "Insert time (Ctrl+Ins)",
        "Insert empty time the size of the loop region at its start (Shift+drag the ruler)",
        space("playlist.insert_space"),
    );
    menu_item(
        ui,
        actions,
        "Remove time (Ctrl+Del)",
        "Remove the time of the loop region; later clips and markers move earlier",
        space("playlist.delete_space"),
    );
}

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

/// Moves clips as one undo step, ordered so no clip lands on one not yet moved.
fn move_clips(group: &[Clip], dt: i64, dtrack: i64) -> Option<Invocation> {
    if dt == 0 && dtrack == 0 {
        return None;
    }
    let mut ordered = group.to_vec();
    ordered.sort_by_key(|c| (c.start, c.track));
    if dt > 0 || (dt == 0 && dtrack > 0) {
        ordered.reverse();
    }
    batch(
        ordered
            .iter()
            .map(|c| {
                let (track, start) = shifted(c, dt, dtrack);
                Invocation::new(
                    "playlist.move_clip",
                    [
                        (c.track + 1).to_string(),
                        format!("{}t", c.start),
                        (track + 1).to_string(),
                        format!("{start}t"),
                    ],
                )
            })
            .collect(),
    )
}

/// Paint and delete strokes. Returns `true` when the tool handled the input.
#[allow(clippy::too_many_arguments)]
fn tool_stroke(
    ui: &egui::Ui,
    state: &mut UiState,
    project: &Project,
    grid: &Grid,
    response: &egui::Response,
    boxes: &[(usize, Rect)],
    editable: &dyn Fn(Pos2) -> bool,
    actions: &mut Vec<Action>,
) -> bool {
    let clips = &project.playlist.clips;
    let under = |p: Pos2| match hit_test(boxes.iter().copied(), p) {
        Hit::Body(i) | Hit::Edge(i) => clips.get(i).copied(),
        Hit::Empty => None,
    };
    let pointer = response.interact_pointer_pos().filter(|p| editable(*p));
    match state.playlist.tool {
        Tool::Delete => {
            if response.drag_started_by(PointerButton::Primary) {
                state.playlist.erased.clear();
            }
            let pressed = response.dragged_by(PointerButton::Primary)
                || response.clicked_by(PointerButton::Primary);
            if let Some(c) = pointer.filter(|_| pressed).and_then(under) {
                if !state.playlist.erased.contains(&c) {
                    state.playlist.erased.push(c);
                }
            }
            if response.drag_stopped_by(PointerButton::Primary)
                || response.clicked_by(PointerButton::Primary)
            {
                let erased = std::mem::take(&mut state.playlist.erased);
                for c in &erased {
                    state.playlist.selection.remove(&(c.track, c.start));
                }
                let removals = erased
                    .iter()
                    .map(|c| {
                        Invocation::new(
                            "playlist.remove_clip",
                            [(c.track + 1).to_string(), format!("{}t", c.start)],
                        )
                    })
                    .collect();
                actions.extend(batch(removals).map(Action::Run));
            }
            true
        }
        Tool::Paint => {
            if state.playlist.drag.is_some() {
                return false;
            }
            if response.drag_started_by(PointerButton::Primary) {
                state.playlist.painted.clear();
                let origin = ui.input(|i| i.pointer.press_origin());
                if origin.and_then(under).is_some() {
                    return false;
                }
            }
            let painting = response.dragged_by(PointerButton::Primary)
                || response.drag_stopped_by(PointerButton::Primary);
            let origin = response
                .drag_started_by(PointerButton::Primary)
                .then(|| ui.input(|i| i.pointer.press_origin()))
                .flatten()
                .filter(|p| editable(*p));
            let length = project.current_pattern().length_ticks().max(1);
            for p in origin.into_iter().chain(pointer.filter(|_| painting)) {
                let Some(track) = grid.row_at(p.y) else {
                    continue;
                };
                let start = state
                    .playlist
                    .snap
                    .floor(grid.tick_at(p.x), &state.playlist.meter);
                let after_last = state
                    .playlist
                    .painted
                    .last()
                    .is_none_or(|(_, s, l)| start >= s + l);
                let end = start + length;
                let free = clips
                    .iter()
                    .all(|c| c.track != track || c.end() <= start || c.start >= end);
                if after_last && free {
                    state.playlist.painted.push((track, start, length));
                }
            }
            if response.drag_stopped_by(PointerButton::Primary) {
                let pattern = (project.current_pattern + 1).to_string();
                let adds = std::mem::take(&mut state.playlist.painted)
                    .into_iter()
                    .map(|(track, start, _)| {
                        Invocation::new(
                            "playlist.add_clip",
                            [
                                pattern.clone(),
                                (track + 1).to_string(),
                                format!("{start}t"),
                            ],
                        )
                    })
                    .collect();
                actions.extend(batch(adds).map(Action::Run));
                return true;
            }
            !state.playlist.painted.is_empty()
        }
        Tool::Mute | Tool::Slice | Tool::Play | Tool::Zoom => {
            let Some(p) = pointer.filter(|_| response.clicked_by(PointerButton::Primary)) else {
                return true;
            };
            let tick = grid.tick_at(p.x);
            let clicked = under(p);
            match (state.playlist.tool, clicked) {
                (Tool::Mute, Some(c)) => actions.push(Action::Run(Invocation::new(
                    "playlist.toggle_clip_mute",
                    [(c.track + 1).to_string(), format!("{}t", c.start)],
                ))),
                (Tool::Slice, Some(c)) => {
                    let at = state.playlist.snap.round(tick, &state.playlist.meter);
                    let at = if at > c.start && at < c.end() {
                        at
                    } else {
                        tick.max(0.0) as u32
                    };
                    actions.extend(slice_at(&[c], at).map(Action::Run));
                }
                (Tool::Play, _) => {
                    let at = state.playlist.snap.floor(tick, &state.playlist.meter);
                    actions.push(Action::Seek(f64::from(at)));
                    actions.push(Action::Run(Invocation::bare("transport.play")));
                }
                (Tool::Zoom, _) => {
                    let factor = if ui.input(|i| i.modifiers.shift) {
                        1.0 / 1.5
                    } else {
                        1.5
                    };
                    state.playlist.px_per_bar = (state.playlist.px_per_bar * factor)
                        .clamp(BAR_WIDTH_RANGE.0, BAR_WIDTH_RANGE.1);
                }
                _ => {}
            }
            true
        }
        Tool::Draw | Tool::Select => false,
    }
}

#[allow(clippy::too_many_arguments)]
fn edit(
    ui: &egui::Ui,
    state: &mut UiState,
    project: &Project,
    grid: &Grid,
    response: &egui::Response,
    clip: Rect,
    boxes: &[(usize, Rect)],
    actions: &mut Vec<Action>,
) {
    let editable = |p: Pos2| p.x > clip.left() + HEADER_WIDTH && p.y > clip.top() + RULER_HEIGHT;
    let snap = state.playlist.snap;
    let meter = state.playlist.meter.clone();
    let clips = &project.playlist.clips;
    let (ctrl, shift) = ui.input(|i| (i.modifiers.command, i.modifiers.shift));
    if tool_stroke(
        ui, state, project, grid, response, boxes, &editable, actions,
    ) {
        return;
    }
    let tool = state.playlist.tool;

    if response.drag_started_by(PointerButton::Primary) {
        if let Some(origin) = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| editable(*p))
        {
            let tick = grid.tick_at(origin.x);
            let hit = hit_test(boxes.iter().copied(), origin);
            if (ctrl || tool == Tool::Select) && hit == Hit::Empty {
                state.playlist.marquee = Some((origin, origin));
            }
            state.playlist.drag = match hit {
                Hit::Body(i) | Hit::Edge(i) => clips.get(i).map(|c| {
                    if !state.playlist.selection.contains(&(c.track, c.start)) {
                        state.playlist.selection = BTreeSet::from([(c.track, c.start)]);
                    }
                    ClipDrag {
                        kind: if matches!(hit, Hit::Edge(_)) {
                            DragKind::Resize
                        } else {
                            DragKind::Move
                        },
                        original: *c,
                        grab: tick - c.start as f32,
                        track: c.track,
                        start: c.start,
                        length: c.length,
                    }
                }),
                Hit::Empty => None,
            };
        }
    }
    if response.dragged_by(PointerButton::Primary) {
        let pos = response.interact_pointer_pos();
        if let (Some((_, current)), Some(pos)) = (&mut state.playlist.marquee, pos) {
            *current = pos;
        } else if let (Some(drag), Some(pos)) = (&mut state.playlist.drag, pos) {
            let tick = grid.tick_at(pos.x);
            match drag.kind {
                DragKind::Move => {
                    drag.start = snap.round(tick - drag.grab, &meter);
                    if let Some(row) = grid.row_at(pos.y) {
                        drag.track = row;
                    }
                }
                DragKind::Resize => {
                    let end = snap
                        .round(tick, &meter)
                        .max(drag.start + snap.ticks_at(drag.start, &meter));
                    drag.length = end - drag.start;
                }
            }
        }
    }
    if response.drag_stopped_by(PointerButton::Primary) {
        if let Some((origin, current)) = state.playlist.marquee.take() {
            let area = Rect::from_two_pos(origin, current);
            if !shift {
                state.playlist.selection.clear();
            }
            for (i, rect) in boxes {
                if let (true, Some(c)) = (area.intersects(*rect), clips.get(*i)) {
                    state.playlist.selection.insert((c.track, c.start));
                }
            }
        } else {
            let group = moving_clips(state, clips);
            let (dt, dtrack, dl) = drag_delta(state, &group);
            if let Some(drag) = state.playlist.drag.take() {
                let invocation = match drag.kind {
                    DragKind::Move => move_clips(&group, dt, dtrack),
                    DragKind::Resize => batch(
                        group
                            .iter()
                            .filter(|_| dl != 0)
                            .map(|c| {
                                let length =
                                    u32::try_from((i64::from(c.length) + dl).max(1)).unwrap_or(1);
                                Invocation::new(
                                    "playlist.resize_clip",
                                    [
                                        (c.track + 1).to_string(),
                                        format!("{}t", c.start),
                                        format!("{length}t"),
                                    ],
                                )
                            })
                            .collect(),
                    ),
                };
                if let Some(invocation) = invocation {
                    actions.push(Action::Run(invocation));
                    state.playlist.selection =
                        group.iter().map(|c| shifted(c, dt, dtrack)).collect();
                }
            }
        }
    }
    if response.clicked_by(PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos().filter(|p| editable(*p)) {
            match (hit_test(boxes.iter().copied(), pos), grid.row_at(pos.y)) {
                (Hit::Body(i) | Hit::Edge(i), _) => {
                    if let Some(c) = clips.get(i) {
                        let id = (c.track, c.start);
                        if shift {
                            if !state.playlist.selection.remove(&id) {
                                state.playlist.selection.insert(id);
                            }
                        } else {
                            state.playlist.selection = BTreeSet::from([id]);
                        }
                    }
                }
                (Hit::Empty, _) if tool == Tool::Select => state.playlist.selection.clear(),
                (Hit::Empty, Some(row)) => {
                    let start = snap.floor(grid.tick_at(pos.x), &meter);
                    let painted = state
                        .playlist
                        .paint_automation
                        .and_then(|id| project.automation_position(id));
                    let audio = state.playlist.paint_audio.and_then(|id| {
                        // New audio clips are as long as the channel's others.
                        let length = clips
                            .iter()
                            .find(|c| c.source == ClipSource::Audio(id))
                            .map(|c| c.length)?;
                        Some((project.channel_position(id)?, length))
                    });
                    let invocation = match (audio, painted) {
                        (Some((channel, length)), _) => Invocation::new(
                            "playlist.add_audio_clip",
                            [
                                (channel + 1).to_string(),
                                (row + 1).to_string(),
                                format!("{start}t"),
                                format!("{length}t"),
                            ],
                        ),
                        (None, Some(i)) => Invocation::new(
                            "playlist.add_automation_clip",
                            [
                                (i + 1).to_string(),
                                (row + 1).to_string(),
                                format!("{start}t"),
                            ],
                        ),
                        (None, None) => Invocation::new(
                            "playlist.add_clip",
                            [
                                (project.current_pattern + 1).to_string(),
                                (row + 1).to_string(),
                                format!("{start}t"),
                            ],
                        ),
                    };
                    actions.push(Action::Run(invocation));
                    state.playlist.selection.clear();
                }
                _ => {}
            }
        }
    }
    if response.double_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Hit::Body(i) | Hit::Edge(i) = hit_test(boxes.iter().copied(), pos) {
                match clips.get(i).map(|c| c.source) {
                    Some(ClipSource::Automation(id)) => state.automation_editor = Some(id),
                    Some(ClipSource::Pattern(id)) => {
                        if let Some(index) = project.pattern_position(id) {
                            actions.push(Action::Run(Invocation::new(
                                "pattern.select",
                                [(index + 1).to_string()],
                            )));
                            state.windows.piano_roll = true;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    // Sounds dropped from the browser become audio clips where they land.
    if response.contains_pointer() && ui.input(|i| i.pointer.any_released()) {
        if let Some(pos) = ui.ctx().pointer_hover_pos().filter(|p| editable(*p)) {
            if let (Some(row), true) = (
                grid.row_at(pos.y),
                egui::DragAndDrop::has_payload_of_type::<crate::browser::Dragged>(ui.ctx()),
            ) {
                if let Some(dragged) =
                    egui::DragAndDrop::take_payload::<crate::browser::Dragged>(ui.ctx())
                {
                    if let Some(sample) = dragged.sample().map(str::to_owned) {
                        actions.push(Action::DropAudio {
                            sample,
                            track: row,
                            start: snap.floor(grid.tick_at(pos.x), &meter),
                        });
                    }
                }
            }
        }
    }
    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Hit::Body(i) | Hit::Edge(i) = hit_test(boxes.iter().copied(), pos) {
                if let Some(c) = clips.get(i) {
                    state.playlist.selection.remove(&(c.track, c.start));
                    actions.push(Action::Run(Invocation::new(
                        "playlist.remove_clip",
                        [(c.track + 1).to_string(), format!("{}t", c.start)],
                    )));
                }
            }
        }
    }
    if response.contains_pointer() {
        keys(ui.ctx(), state, project, actions);
    }
    if let Some(pos) = response.hover_pos().filter(|p| editable(*p)) {
        let cursor = match hit_test(boxes.iter().copied(), pos) {
            Hit::Edge(_) => egui::CursorIcon::ResizeHorizontal,
            Hit::Body(_) => egui::CursorIcon::Grab,
            Hit::Empty if ctrl => egui::CursorIcon::Cell,
            Hit::Empty => egui::CursorIcon::Default,
        };
        ui.ctx().set_cursor_icon(cursor);
    }
}

/// Editing keys while the pointer is over the playlist.
fn keys(ctx: &egui::Context, state: &mut UiState, project: &Project, actions: &mut Vec<Action>) {
    use egui::{Key, Modifiers};
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let clips = &project.playlist.clips;
    let selected = selected_clips(state, project);
    let pressed = |modifiers: Modifiers, key: Key| ctx.input_mut(|i| i.consume_key(modifiers, key));
    if pressed(Modifiers::COMMAND, Key::A) {
        state.playlist.selection = clips.iter().map(|c| (c.track, c.start)).collect();
    }
    if pressed(Modifiers::NONE, Key::Escape) {
        state.playlist.selection.clear();
    }
    if (pressed(Modifiers::NONE, Key::Delete) || pressed(Modifiers::NONE, Key::Backspace))
        && !selected.is_empty()
    {
        actions.extend(removal(&selected).map(Action::Run));
        state.playlist.selection.clear();
    }
    if pressed(Modifiers::COMMAND, Key::B) && !selected.is_empty() {
        let span = duplicate_span(&selected);
        actions.extend(duplication(project, &selected).map(Action::Run));
        state.playlist.selection = selected.iter().map(|c| (c.track, c.start + span)).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_project::PatternId;

    fn pattern_clip(track: u32, start: u32, length: u32) -> Clip {
        Clip {
            source: ClipSource::Pattern(PatternId(1)),
            track,
            start,
            length,
            offset: 0,
            muted: false,
        }
    }

    #[test]
    fn slicing_at_the_playhead_cuts_only_crossing_clips() {
        let clips = [pattern_clip(0, 0, 384), pattern_clip(1, 384, 384)];
        assert_eq!(
            slice_at(&clips, 192).unwrap().to_string(),
            "playlist.slice_clip 1 192t"
        );
        assert!(slice_at(&clips, 384).is_none(), "an edge is not a cut");
        assert!(merge(&clips[..1]).is_none(), "merging needs two clips");
        assert_eq!(
            merge(&clips).unwrap().to_string(),
            "playlist.merge_clips 1 0t 2 384t"
        );
    }

    fn drag(kind: DragKind, track: u32, start: u32, length: u32) -> ClipDrag {
        ClipDrag {
            kind,
            original: Clip {
                source: musiccraft_project::ClipSource::Pattern(PatternId(1)),
                track: 0,
                start: 384,
                length: 384,
                offset: 0,
                muted: false,
            },
            grab: 0.0,
            track,
            start,
            length,
        }
    }

    #[test]
    fn clip_gestures_become_commands() {
        assert_eq!(
            finish(&drag(DragKind::Move, 2, 768, 384))
                .unwrap()
                .to_string(),
            "playlist.move_clip 1 384t 3 768t"
        );
        assert_eq!(
            finish(&drag(DragKind::Resize, 0, 384, 1152))
                .unwrap()
                .to_string(),
            "playlist.resize_clip 1 384t 1152t"
        );
        assert!(finish(&drag(DragKind::Move, 0, 384, 384)).is_none());
    }

    #[test]
    fn previews_place_notes_by_key_or_by_channel_row() {
        let mut s = musiccraft_commands::Session::new(
            Project::default(),
            musiccraft_commands::Registry::with_builtin(),
        );
        for line in [
            "channel.add_sampler builtin:kick",
            "channel.add_sampler builtin:hat",
            "channel.add_synth",
            // Pattern 1: steps, hat entered before kick.
            "step.toggle 2 1",
            "step.toggle 1 1",
            "step.toggle 2 5",
            // Pattern 2: a melody on two keys.
            "pattern.add",
            "note.add 3 0 4 C4 100",
            "note.add 3 4 4 C5 100",
            // Pattern 3: steps of one channel.
            "pattern.add",
            "step.toggle 1 1",
            "step.toggle 1 9",
        ] {
            s.run_line(line).unwrap_or_else(|e| panic!("{line}: {e}"));
        }
        let project = s.project();
        let body = Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 20.0));
        let ys = |pattern: &musiccraft_project::Pattern| -> Vec<(usize, f32)> {
            let rows = NoteRows::of(project, pattern);
            pattern
                .notes
                .iter()
                .map(|n| {
                    let channel = project.channel_position(n.channel).unwrap_or(99);
                    (channel, rows.y(project, n, body))
                })
                .collect()
        };

        // Drums: one row per channel, in rack order whatever the note order.
        let steps = &project.patterns[0];
        assert!(matches!(NoteRows::of(project, steps), NoteRows::Channels(ref r) if r == &[0, 1]));
        for (channel, y) in ys(steps) {
            let want = if channel == 0 { 5.0 } else { 15.0 };
            assert!((y - want).abs() < 1e-4, "channel {channel} at {y}");
        }
        // A melody: by key, the lowest at the bottom.
        let melody = &project.patterns[1];
        let mut by_key: Vec<(u8, f32)> = melody
            .notes
            .iter()
            .map(|n| (n.key, NoteRows::of(project, melody).y(project, n, body)))
            .collect();
        by_key.sort_by_key(|(key, _)| *key);
        assert_eq!(
            by_key.iter().map(|(_, y)| *y).collect::<Vec<_>>(),
            [19.0, 1.0]
        );
        // One channel on one key: a single centre line.
        let single = &project.patterns[2];
        assert!(matches!(NoteRows::of(project, single), NoteRows::Single));
        assert!(ys(single).iter().all(|(_, y)| (*y - 10.0).abs() < 1e-4));
    }
}
