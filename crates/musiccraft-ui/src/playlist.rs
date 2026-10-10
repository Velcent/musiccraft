//! The playlist: arrange patterns as clips on track rows.

use egui::{
    Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2,
};
use musiccraft_commands::Invocation;
use musiccraft_project::params::{PPQ, TICKS_PER_BAR};
use musiccraft_project::{Clip, PLAYLIST_TRACKS, PlayMode, Project};

use crate::dialogs::Dialog;
use crate::geometry::{Grid, Hit, Snap, hit_test};
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
}

impl Default for PlaylistState {
    fn default() -> Self {
        Self {
            px_per_bar: 72.0,
            row_height: 30.0,
            snap: Snap::Bar,
            drag: None,
            last_view: None,
        }
    }
}

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
        ui.label("Snap");
        egui::ComboBox::from_id_salt("playlist-snap")
            .width(70.0)
            .selected_text(state.playlist.snap.label())
            .show_ui(ui, |ui| {
                for snap in [Snap::Step, Snap::Beat, Snap::Bar] {
                    ui.selectable_value(&mut state.playlist.snap, snap, snap.label());
                }
            });
        if ui.small_button("−").on_hover_text("Zoom out (Ctrl+scroll)").clicked() {
            state.playlist.px_per_bar = (state.playlist.px_per_bar / 1.25).max(16.0);
        }
        if ui.small_button("+").on_hover_text("Zoom in (Ctrl+scroll)").clicked() {
            state.playlist.px_per_bar = (state.playlist.px_per_bar * 1.25).min(400.0);
        }
        ui.label(
            RichText::new("Click to paint the selected pattern · drag to move · edge to resize · right-click deletes · click the ruler to seek")
                .small()
                .color(theme::TEXT_DIM),
        );
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
                    let response = ui.selectable_label(selected, text);
                    if response.clicked() && !selected {
                        actions.push(Action::Run(Invocation::new(
                            "pattern.select",
                            [(i + 1).to_string()],
                        )));
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
            actions.push(Action::Run(Invocation::bare("pattern.add")));
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
            let dragged = state.playlist.drag.map(|d| d.original);
            for clip_item in clips {
                if Some(*clip_item) == dragged {
                    continue;
                }
                draw_clip(
                    &painter,
                    &grid,
                    project,
                    clip_item,
                    clip_item.track,
                    clip_item.start,
                    clip_item.length,
                    false,
                );
            }
            if let Some(drag) = state.playlist.drag {
                draw_clip(
                    &painter,
                    &grid,
                    project,
                    &drag.original,
                    drag.track,
                    drag.start,
                    drag.length,
                    true,
                );
            }
            if frame.engine.playing && project.play_mode == PlayMode::Song {
                let x = grid.x(frame.engine.position_ticks as f32);
                painter.line_segment(
                    [pos2(x, clip.top()), pos2(x, clip.bottom())],
                    Stroke::new(1.5, theme::PLAYHEAD),
                );
            }
            let boxes: Vec<(usize, Rect)> = clips
                .iter()
                .enumerate()
                .map(|(i, c)| (i, grid.span(c.track, c.start, c.length)))
                .collect();
            edit(ui, state, project, &grid, &response, clip, &boxes, actions);
            draw_ruler(&painter, &grid, clip, project);
            seek_ruler(ui, state.playlist.snap, &grid, clip, actions);
            headers(ui, project, &grid, clip, actions);
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
    project: &Project,
    clip: &Clip,
    track: u32,
    start: u32,
    length: u32,
    ghost: bool,
) {
    let index = project.pattern_position(clip.pattern).unwrap_or(0);
    let color = theme::palette(index);
    let rect = grid.span(track, start, length).shrink2(vec2(0.5, 1.5));
    let muted = project.playlist.muted_tracks.contains(&track);
    let alpha = if ghost {
        120
    } else if muted {
        60
    } else {
        200
    };
    painter.rect_filled(
        rect,
        4.0,
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha),
    );
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(1.0, Color32::from_black_alpha(140)),
        egui::StrokeKind::Inside,
    );
    let Some(pattern) = project.patterns.get(index) else {
        return;
    };
    // Miniature of the pattern's notes, repeated across the clip.
    let period = pattern.length_ticks().max(1);
    let (lo, hi) = pattern
        .notes
        .iter()
        .fold((127u8, 0u8), |(lo, hi), n| (lo.min(n.key), hi.max(n.key)));
    let span = f32::from(hi.saturating_sub(lo).max(1));
    let body = rect.shrink2(vec2(2.0, 4.0));
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
    painter.text(
        rect.left_top() + vec2(4.0, 2.0),
        Align2::LEFT_TOP,
        &pattern.name,
        FontId::proportional(10.5),
        theme::BACKGROUND,
    );
}

fn draw_ruler(painter: &egui::Painter, grid: &Grid, clip: Rect, project: &Project) {
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
}

/// Clicking or dragging on the ruler moves the playhead.
fn seek_ruler(ui: &mut egui::Ui, snap: Snap, grid: &Grid, clip: Rect, actions: &mut Vec<Action>) {
    let ruler = Rect::from_min_max(
        pos2(clip.left() + HEADER_WIDTH, clip.top()),
        pos2(clip.right(), clip.top() + RULER_HEIGHT),
    );
    let response = ui
        .interact(
            ruler,
            ui.id().with("playlist-ruler"),
            Sense::click_and_drag(),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() || response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let tick = snap
                .floor(grid.tick_at(pos.x))
                .min(Snap::Beat.floor(grid.tick_at(pos.x)));
            actions.push(Action::Seek(f64::from(tick)));
        }
    }
}

fn headers(
    ui: &mut egui::Ui,
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
        painter.line_segment(
            [r.left_bottom(), r.right_bottom()],
            Stroke::new(1.0, theme::GRID_LINE),
        );
        painter.text(
            r.left_center() + vec2(24.0, 0.0),
            Align2::LEFT_CENTER,
            format!("Track {}", row + 1),
            FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
        let dot = Rect::from_center_size(r.left_center() + vec2(12.0, 0.0), vec2(14.0, 14.0));
        if !area.contains_rect(dot) {
            continue;
        }
        let muted = project.playlist.muted_tracks.contains(&row);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(dot));
        if widgets::dot(&mut child, !muted, theme::ACCENT)
            .on_hover_text("Mute track")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "playlist.toggle_track_mute",
                [(row + 1).to_string()],
            )));
        }
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

    if response.drag_started_by(PointerButton::Primary) {
        if let Some(origin) = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| editable(*p))
        {
            let tick = grid.tick_at(origin.x);
            state.playlist.drag = match hit_test(boxes.iter().copied(), origin) {
                Hit::Body(i) => clips.get(i).map(|c| ClipDrag {
                    kind: DragKind::Move,
                    original: *c,
                    grab: tick - c.start as f32,
                    track: c.track,
                    start: c.start,
                    length: c.length,
                }),
                Hit::Edge(i) => clips.get(i).map(|c| ClipDrag {
                    kind: DragKind::Resize,
                    original: *c,
                    grab: 0.0,
                    track: c.track,
                    start: c.start,
                    length: c.length,
                }),
                Hit::Empty => None,
            };
        }
    }
    if response.dragged_by(PointerButton::Primary) {
        if let (Some(drag), Some(pos)) = (&mut state.playlist.drag, response.interact_pointer_pos())
        {
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
        if let Some(invocation) = state.playlist.drag.take().as_ref().and_then(finish) {
            actions.push(Action::Run(invocation));
        }
    }
    if response.clicked_by(PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos().filter(|p| editable(*p)) {
            if let (Hit::Empty, Some(row)) =
                (hit_test(boxes.iter().copied(), pos), grid.row_at(pos.y))
            {
                let start = snap.floor(grid.tick_at(pos.x));
                actions.push(Action::Run(Invocation::new(
                    "playlist.add_clip",
                    [
                        (project.current_pattern + 1).to_string(),
                        (row + 1).to_string(),
                        format!("{start}t"),
                    ],
                )));
            }
        }
    }
    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Hit::Body(i) | Hit::Edge(i) = hit_test(boxes.iter().copied(), pos) {
                if let Some(c) = clips.get(i) {
                    actions.push(Action::Run(Invocation::new(
                        "playlist.remove_clip",
                        [(c.track + 1).to_string(), format!("{}t", c.start)],
                    )));
                }
            }
        }
    }
    if let Some(pos) = response.hover_pos().filter(|p| editable(*p)) {
        let cursor = match hit_test(boxes.iter().copied(), pos) {
            Hit::Edge(_) => egui::CursorIcon::ResizeHorizontal,
            Hit::Body(_) => egui::CursorIcon::Grab,
            Hit::Empty => egui::CursorIcon::Default,
        };
        ui.ctx().set_cursor_icon(cursor);
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
                pattern: PatternId(1),
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
