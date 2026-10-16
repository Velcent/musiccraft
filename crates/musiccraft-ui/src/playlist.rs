//! The playlist: arrange patterns as clips on track rows.

use crate::hint::Hint;
use std::collections::BTreeSet;

use egui::{
    Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2,
};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{PPQ, TICKS_PER_BAR};
use musiccraft_project::{
    AutomationId, Clip, ClipSource, MarkerAction, PLAYLIST_TRACKS, PlayMode, Project,
};

use crate::dialogs::Dialog;
use crate::geometry::{Grid, Hit, Snap, hit_test};
use crate::icons::{self, Icon};
use crate::piano_roll::Tool;
use crate::{Action, Frame, UiState, theme, widgets};

const HEADER_WIDTH: f32 = 96.0;
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

#[derive(Debug, Clone, PartialEq)]
pub struct PlaylistState {
    px_per_bar: f32,
    row_height: f32,
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
    /// Loop range being drawn on the ruler.
    loop_drag: Option<(u32, u32)>,
    /// The active tool.
    pub(crate) tool: Tool,
    /// Clips painted in the stroke in progress: (track, start, length).
    painted: Vec<(u32, u32, u32)>,
    /// Clips touched by the delete stroke in progress.
    erased: Vec<Clip>,
}

impl Default for PlaylistState {
    fn default() -> Self {
        Self {
            px_per_bar: 72.0,
            row_height: 30.0,
            snap: Snap::Bar,
            drag: None,
            last_view: None,
            selection: BTreeSet::new(),
            marquee: None,
            paint_automation: None,
            loop_drag: None,
            tool: Tool::Draw,
            painted: Vec::new(),
            erased: Vec::new(),
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

/// Draws the playlist as the main workspace.
pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Playlist").strong().size(15.0));
        ui.label(RichText::new("Arrangement").color(theme::TEXT_DIM));
        ui.separator();
        ui.spacing_mut().item_spacing.x = 2.0;
        for (tool, icon, hint) in [
            (
                Tool::Draw,
                Icon::Pencil,
                "Draw: click places the selected pattern, drag moves, edge resizes",
            ),
            (
                Tool::Paint,
                Icon::Brush,
                "Paint: drag to lay the selected pattern end to end",
            ),
            (
                Tool::Delete,
                Icon::Eraser,
                "Delete: click or drag over clips to delete them",
            ),
            (
                Tool::Select,
                Icon::Select,
                "Select: drag a box around clips; click empty space to clear",
            ),
        ] {
            if icons::button(ui, icon, state.playlist.tool == tool, theme::ACCENT, hint).clicked() {
                state.playlist.tool = tool;
            }
        }
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.separator();
        ui.label("Snap");
        egui::ComboBox::from_id_salt("playlist-snap")
            .width(70.0)
            .selected_text(state.playlist.snap.label())
            .show_ui(ui, |ui| {
                for snap in [Snap::Step, Snap::Beat, Snap::Bar] {
                    ui.selectable_value(&mut state.playlist.snap, snap, snap.label());
                }
            });
        if ui
            .small_button("−")
            .hint("Zoom out (Ctrl+scroll)")
            .clicked()
        {
            state.playlist.px_per_bar = (state.playlist.px_per_bar / 1.25).max(16.0);
        }
        if ui.small_button("+").hint("Zoom in (Ctrl+scroll)").clicked() {
            state.playlist.px_per_bar = (state.playlist.px_per_bar * 1.25).min(400.0);
        }
    });
    ui.add_space(2.0);
    ui.horizontal_top(|ui| {
        pattern_list(ui, state, frame.project, actions);
        canvas(ui, state, frame, actions);
    });
}

fn pattern_list(
    ui: &mut egui::Ui,
    state: &mut UiState,
    project: &Project,
    actions: &mut Vec<Action>,
) {
    ui.vertical(|ui| {
        ui.set_width(LIST_WIDTH);
        ui.label(
            RichText::new("PATTERNS")
                .small()
                .strong()
                .color(theme::TEXT_DIM),
        );
        egui::ScrollArea::vertical()
            .id_salt("pattern-list")
            .max_height(260.0)
            .show(ui, |ui| {
                for (i, pattern) in project.patterns.iter().enumerate() {
                    let selected = i == project.current_pattern;
                    let text =
                        RichText::new(format!("■ {}", pattern.name)).color(theme::palette(i));
                    let painting = state.playlist.paint_automation.is_none();
                    let response = ui.selectable_label(selected && painting, text);
                    if response.clicked() {
                        state.playlist.paint_automation = None;
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
        if ui.button("+ New pattern").clicked() {
            state.playlist.paint_automation = None;
            actions.push(Action::Run(Invocation::bare("pattern.add")));
        }
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
    });
}

fn canvas(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let px_per_tick = state.playlist.px_per_bar / TICKS_PER_BAR as f32;
    let row_h = state.playlist.row_height;
    let bars = (project.playlist.length_ticks() / TICKS_PER_BAR + 16).max(32);
    let size = vec2(
        HEADER_WIDTH + (bars * TICKS_PER_BAR) as f32 * px_per_tick,
        RULER_HEIGHT + PLAYLIST_TRACKS as f32 * row_h,
    );
    egui::ScrollArea::both()
        .id_salt("playlist-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
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
            draw_grid(&painter, &grid, clip, rect);

            let clips = &project.playlist.clips;
            let moving = moving_clips(state, clips);
            let (dt, dtrack, dl) = drag_delta(state, &moving);
            for clip_item in clips {
                let is_moving = moving.contains(clip_item);
                let erasing = state.playlist.erased.contains(clip_item);
                if !is_moving && !erasing {
                    draw_clip(
                        &painter,
                        &grid,
                        frame,
                        clip_item,
                        clip_item.track,
                        clip_item.start,
                        clip_item.length,
                        false,
                    );
                    if state
                        .playlist
                        .selection
                        .contains(&(clip_item.track, clip_item.start))
                    {
                        let r = grid
                            .span(clip_item.track, clip_item.start, clip_item.length)
                            .shrink2(vec2(0.5, 1.5));
                        painter.rect_stroke(
                            r,
                            4.0,
                            Stroke::new(1.5, theme::TEXT),
                            egui::StrokeKind::Inside,
                        );
                    }
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
                };
                draw_clip(
                    &painter, &grid, frame, &preview, *track, *start, *length, true,
                );
            }
            for c in &moving {
                let (track, start) = shifted(c, dt, dtrack);
                let length = u32::try_from((i64::from(c.length) + dl).max(1)).unwrap_or(1);
                draw_clip(&painter, &grid, frame, c, track, start, length, true);
            }
            if let Some((origin, current)) = state.playlist.marquee {
                let area = Rect::from_two_pos(origin, current);
                painter.rect_filled(
                    area,
                    0.0,
                    Color32::from_rgba_unmultiplied(0x2e, 0xc4, 0xb6, 24),
                );
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
            draw_ruler(&painter, &grid, clip, project, state.playlist.loop_drag);
            ruler(ui, state, project, &grid, clip, actions);
            headers(ui, state, project, &grid, clip, actions);
            if response.hovered() {
                let zoom = ui.input(|i| i.zoom_delta());
                if zoom != 1.0 {
                    state.playlist.px_per_bar =
                        (state.playlist.px_per_bar * zoom).clamp(16.0, 400.0);
                }
            }
        });
}

fn draw_grid(painter: &egui::Painter, grid: &Grid, clip: Rect, rect: Rect) {
    let first = grid.row_at(clip.top().max(grid.origin.y)).unwrap_or(0);
    let last = grid
        .row_at(clip.bottom() - 1.0)
        .unwrap_or(PLAYLIST_TRACKS - 1);
    for row in first..=last {
        let y = grid.y(row);
        let fill = if row % 2 == 0 {
            theme::WHITE_KEY_ROW
        } else {
            theme::BLACK_KEY_ROW
        };
        painter.rect_filled(
            Rect::from_min_max(
                pos2(grid.origin.x, y),
                pos2(rect.right(), y + grid.row_height),
            ),
            0.0,
            fill,
        );
    }
    let first_beat = grid.tick_at(clip.left()) as u32 / PPQ;
    let last_beat = grid.tick_at(clip.right()) as u32 / PPQ + 1;
    for beat in first_beat..=last_beat {
        let tick = beat * PPQ;
        let color = if tick % TICKS_PER_BAR == 0 {
            theme::GRID_BAR
        } else {
            theme::GRID_LINE
        };
        let x = grid.x(tick as f32);
        painter.line_segment(
            [pos2(x, grid.origin.y), pos2(x, rect.bottom())],
            Stroke::new(1.0, color),
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_clip(
    painter: &egui::Painter,
    grid: &Grid,
    frame: &Frame<'_>,
    clip: &Clip,
    track: u32,
    start: u32,
    length: u32,
    ghost: bool,
) {
    let project = frame.project;
    let (color, name) = match clip.source {
        ClipSource::Pattern(id) => {
            let index = project.pattern_position(id).unwrap_or(0);
            (
                theme::palette(index),
                project.patterns.get(index).map_or("", |p| p.name.as_str()),
            )
        }
        ClipSource::Automation(id) => (
            theme::AUTOMATION,
            project
                .automation_position(id)
                .map_or("", |i| project.automations[i].name.as_str()),
        ),
        ClipSource::Audio(id) => {
            let index = project.channel_position(id).unwrap_or(0);
            (
                theme::palette(index),
                project.channels.get(index).map_or("", |c| c.name.as_str()),
            )
        }
    };
    let rect = grid.span(track, start, length).shrink2(vec2(0.5, 1.5));
    let muted = project.playlist.muted_tracks.contains(&track);
    let alpha = if ghost {
        120
    } else if muted {
        60
    } else {
        200
    };
    let fill = match clip.source {
        ClipSource::Automation(_) => Color32::from_rgba_unmultiplied(0x2a, 0x2f, 0x3a, alpha),
        _ => Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
    };
    painter.rect_filled(rect, 4.0, fill);
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(
            1.0,
            if matches!(clip.source, ClipSource::Automation(_)) {
                color
            } else {
                Color32::from_black_alpha(140)
            },
        ),
        egui::StrokeKind::Inside,
    );
    let body = rect.shrink2(vec2(2.0, 4.0));
    match clip.source {
        ClipSource::Pattern(id) => {
            if let Some(pattern) = project
                .pattern_position(id)
                .and_then(|i| project.patterns.get(i))
            {
                pattern_preview(painter, grid, pattern, body, start, length);
            }
        }
        ClipSource::Automation(id) => {
            if let Some(automation) = project
                .automation_position(id)
                .map(|i| &project.automations[i])
            {
                let steps = (rect.width() / 3.0).clamp(2.0, 400.0) as usize;
                let points: Vec<Pos2> = (0..=steps)
                    .map(|i| {
                        let t = i as f32 / steps as f32;
                        let tick = t * length as f32;
                        let value = automation.value_at(f64::from(tick)).unwrap_or(0.0);
                        pos2(
                            grid.x(start as f32 + tick),
                            body.bottom() - value * body.height(),
                        )
                    })
                    .collect();
                painter.add(egui::Shape::line(points, Stroke::new(1.5, color)));
            }
        }
        ClipSource::Audio(id) => {
            if let Some((_, peaks)) = frame.clip_waveforms.iter().find(|(c, _)| *c == id) {
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
                        Stroke::new(1.0, Color32::from_black_alpha(150)),
                    );
                }
            }
        }
    }
    painter.text(
        rect.left_top() + vec2(4.0, 2.0),
        Align2::LEFT_TOP,
        name,
        FontId::proportional(10.5),
        if matches!(clip.source, ClipSource::Automation(_)) {
            theme::TEXT
        } else {
            theme::BACKGROUND
        },
    );
}

/// Miniature of a pattern's notes, repeated across the clip.
fn pattern_preview(
    painter: &egui::Painter,
    grid: &Grid,
    pattern: &musiccraft_project::Pattern,
    body: Rect,
    start: u32,
    length: u32,
) {
    let period = pattern.length_ticks().max(1);
    let (lo, hi) = pattern
        .notes
        .iter()
        .fold((127u8, 0u8), |(lo, hi), n| (lo.min(n.key), hi.max(n.key)));
    let span = f32::from(hi.saturating_sub(lo).max(1));
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
            let y = if hi == lo {
                body.center().y
            } else {
                body.bottom()
                    - (f32::from(note.key - lo.min(note.key)) / span) * (body.height() - 2.0)
                    - 1.0
            };
            painter.line_segment(
                [pos2(x0, y), pos2(x1, y)],
                Stroke::new(1.5, Color32::from_black_alpha(150)),
            );
        }
        offset += period;
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
    Invocation::new(
        name,
        [
            number.map_or(1, |i| i + 1).to_string(),
            (track + 1).to_string(),
            format!("{start}t"),
            format!("{}t", clip.length),
        ],
    )
}

fn draw_ruler(
    painter: &egui::Painter,
    grid: &Grid,
    clip: Rect,
    project: &Project,
    dragging: Option<(u32, u32)>,
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
            painter.rect_filled(
                band,
                0.0,
                Color32::from_rgba_unmultiplied(0xff, 0xb3, 0x47, 60),
            );
            let shade = Color32::from_rgba_unmultiplied(0xff, 0xb3, 0x47, 14);
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
    let first_bar = grid.tick_at(clip.left()) as u32 / TICKS_PER_BAR;
    let last_bar = grid.tick_at(clip.right()) as u32 / TICKS_PER_BAR + 1;
    let every = if grid.px_per_tick * TICKS_PER_BAR as f32 >= 28.0 {
        1
    } else {
        4
    };
    for bar in (first_bar..=last_bar).filter(|b| b % every == 0) {
        let x = grid.x((bar * TICKS_PER_BAR) as f32);
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
        // pause, a down-facing arrow for the song loop point.
        let color = match marker.action {
            MarkerAction::None => theme::AUTOMATION,
            MarkerAction::Pause => theme::DANGER,
            MarkerAction::SongLoop => theme::ACCENT,
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
        }
        let [r, g, b, _] = color.to_array();
        painter.line_segment(
            [pos2(x, top), pos2(x, clip.bottom())],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(r, g, b, 70)),
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
            let start = Snap::Bar.floor(tick_at(pos.x));
            state.playlist.loop_drag = Some((start, start));
        }
    }
    if let Some((start, end)) = &mut state.playlist.loop_drag {
        if let Some(pos) = response.interact_pointer_pos() {
            *end = Snap::Bar.round(tick_at(pos.x));
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
                    .floor(tick_at(pos.x))
                    .min(Snap::Beat.floor(tick_at(pos.x))),
            };
            actions.push(Action::Seek(f64::from(tick)));
        }
    }
    let press = response.interact_pointer_pos().or(response.hover_pos());
    response.context_menu(|ui| {
        let x = press.map_or(area.left(), |p| p.x);
        let tick = Snap::Bar.floor(tick_at(x));
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
        if let Some(marker) = marker_at(x) {
            ui.separator();
            ui.label(format!("Marker \"{}\"", marker.name));
            for action in MarkerAction::ALL {
                let (label, hint) = match action {
                    MarkerAction::None => ("No action", "Only a label"),
                    MarkerAction::Pause => {
                        ("Pause", "Song playback stops here; press play to continue")
                    }
                    MarkerAction::SongLoop => (
                        "Song loop",
                        "At the end of the song, playback returns here instead of to the start",
                    ),
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
        painter.text(
            r.left_center() + vec2(24.0, 0.0),
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
        let dot = Rect::from_center_size(r.left_center() + vec2(12.0, 0.0), vec2(14.0, 14.0));
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
        let solo = Rect::from_center_size(r.right_center() - vec2(12.0, 0.0), vec2(16.0, 14.0));
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
                let start = state.playlist.snap.floor(grid.tick_at(p.x));
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
        // The playlist has no mute tool.
        Tool::Draw | Tool::Select | Tool::Mute => false,
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
                    drag.start = snap.round(tick - drag.grab);
                    if let Some(row) = grid.row_at(pos.y) {
                        drag.track = row;
                    }
                }
                DragKind::Resize => {
                    let end = snap.round(tick).max(drag.start + snap.ticks());
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
                    let start = snap.floor(grid.tick_at(pos.x));
                    let painted = state
                        .playlist
                        .paint_automation
                        .and_then(|id| project.automation_position(id));
                    let invocation = match painted {
                        Some(i) => Invocation::new(
                            "playlist.add_automation_clip",
                            [
                                (i + 1).to_string(),
                                (row + 1).to_string(),
                                format!("{start}t"),
                            ],
                        ),
                        None => Invocation::new(
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
                    if let Some(sample) = dragged.0.clone() {
                        actions.push(Action::DropAudio {
                            sample,
                            track: row,
                            start: snap.floor(grid.tick_at(pos.x)),
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
    let selected: Vec<Clip> = clips
        .iter()
        .filter(|c| state.playlist.selection.contains(&(c.track, c.start)))
        .copied()
        .collect();
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
        let removes = selected
            .iter()
            .map(|c| {
                Invocation::new(
                    "playlist.remove_clip",
                    [(c.track + 1).to_string(), format!("{}t", c.start)],
                )
            })
            .collect();
        actions.extend(batch(removes).map(Action::Run));
        state.playlist.selection.clear();
    }
    if pressed(Modifiers::COMMAND, Key::B) && !selected.is_empty() {
        let first = selected.iter().map(|c| c.start).min().unwrap_or(0);
        let end = selected.iter().map(Clip::end).max().unwrap_or(first);
        let span = end - first;
        let adds = selected
            .iter()
            .map(|c| add_clip_invocation(project, c, c.track, c.start + span))
            .collect();
        actions.extend(batch(adds).map(Action::Run));
        state.playlist.selection = selected.iter().map(|c| (c.track, c.start + span)).collect();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_project::PatternId;

    fn drag(kind: DragKind, track: u32, start: u32, length: u32) -> ClipDrag {
        ClipDrag {
            kind,
            original: Clip {
                source: musiccraft_project::ClipSource::Pattern(PatternId(1)),
                track: 0,
                start: 384,
                length: 384,
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
}
