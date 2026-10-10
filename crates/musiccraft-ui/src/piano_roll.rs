//! The piano roll: edit the selected channel's notes in the selected pattern.

use egui::{
    Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Sense, Stroke, pos2, vec2,
};
use musiccraft_commands::Invocation;
use musiccraft_project::music::key_name;
use musiccraft_project::params::{PPQ, TICKS_PER_BAR, TICKS_PER_STEP};
use musiccraft_project::{Note, PlayMode, Project};

use crate::geometry::{Grid, Hit, Snap, hit_test};
use crate::{Action, Frame, UiState, theme};

const KEYS_WIDTH: f32 = 52.0;
const RULER_HEIGHT: f32 = 18.0;
/// Height of the velocity lane pinned to the bottom of the view.
const LANE_HEIGHT: f32 = 64.0;
const KEYS: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DragKind {
    Create,
    Move,
    Resize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct NoteDrag {
    kind: DragKind,
    original: Note,
    /// Ticks between the note start and the grab point.
    grab: f32,
    start: u32,
    key: u8,
    length: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RollState {
    px_per_step: f32,
    key_height: f32,
    snap: Snap,
    last_length: u32,
    velocity: u8,
    drag: Option<NoteDrag>,
    preview: Option<(usize, u8)>,
    /// Channel the view was last centred for.
    centred_for: Option<usize>,
    /// Grid and visible area from the last frame (used by tests and tools).
    pub(crate) last_view: Option<(Grid, Rect)>,
    /// Chord drawn by a click, or `None` for single notes.
    chord: Option<&'static str>,
    /// Highlighted scale: (root pitch class, scale index).
    scale: Option<(u8, usize)>,
    /// Note whose velocity is being dragged in the lane.
    lane_drag: Option<Note>,
}

impl Default for RollState {
    fn default() -> Self {
        Self {
            px_per_step: 24.0,
            key_height: 14.0,
            snap: Snap::Step,
            last_length: TICKS_PER_STEP,
            velocity: 100,
            drag: None,
            preview: None,
            centred_for: None,
            last_view: None,
            chord: None,
            scale: None,
            lane_drag: None,
        }
    }
}

fn is_black(key: u8) -> bool {
    matches!(key % 12, 1 | 3 | 6 | 8 | 10)
}

fn key_of_row(row: u32) -> u8 {
    u8::try_from(KEYS - 1 - row.min(KEYS - 1)).unwrap_or(0)
}

pub(crate) fn row_of_key(key: u8) -> u32 {
    KEYS - 1 - u32::from(key)
}

/// Invocation that finishes a drag gesture.
fn finish(drag: &NoteDrag, channel: usize, velocity: u8) -> Option<Invocation> {
    let ch = (channel + 1).to_string();
    let t = |ticks: u32| format!("{ticks}t");
    let n = drag.original;
    match drag.kind {
        DragKind::Create => Some(Invocation::new(
            "note.add",
            [
                ch,
                t(drag.start),
                t(drag.length),
                drag.key.to_string(),
                velocity.to_string(),
            ],
        )),
        DragKind::Move if (drag.start, drag.key) != (n.start, n.key) => Some(Invocation::new(
            "note.move",
            [
                ch,
                t(n.start),
                n.key.to_string(),
                t(drag.start),
                drag.key.to_string(),
            ],
        )),
        DragKind::Resize if drag.length != n.length => Some(Invocation::new(
            "note.resize",
            [ch, t(n.start), n.key.to_string(), t(drag.length)],
        )),
        _ => None,
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.piano_roll;
    let project = frame.project;
    let title = match project.channels.get(state.selected_channel) {
        Some(channel) => format!("Piano roll — {}", channel.name),
        None => "Piano roll".to_owned(),
    };
    egui::Window::new(title)
        .id(egui::Id::new("piano-roll"))
        .open(&mut open)
        .default_pos([260.0, 160.0])
        .default_size([860.0, 480.0])
        .show(ctx, |ui| {
            if project.channels.is_empty() {
                ui.label(RichText::new("Add a channel to write notes.").color(theme::TEXT_DIM));
                return;
            }
            toolbar(ui, state, project, actions);
            ui.separator();
            canvas(ui, state, frame, actions);
        });
    state.windows.piano_roll = open;
}

fn toolbar(ui: &mut egui::Ui, state: &mut UiState, project: &Project, actions: &mut Vec<Action>) {
    let channel = state.selected_channel;
    let number = (channel + 1).to_string();
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("roll-channel")
            .width(130.0)
            .selected_text(
                project
                    .channels
                    .get(channel)
                    .map_or("", |c| c.name.as_str()),
            )
            .show_ui(ui, |ui| {
                for (i, c) in project.channels.iter().enumerate() {
                    ui.selectable_value(&mut state.selected_channel, i, &c.name);
                }
            });
        ui.label("Snap");
        egui::ComboBox::from_id_salt("roll-snap")
            .width(80.0)
            .selected_text(state.roll.snap.label())
            .show_ui(ui, |ui| {
                for snap in Snap::ALL {
                    ui.selectable_value(&mut state.roll.snap, snap, snap.label());
                }
            });
        egui::ComboBox::from_id_salt("roll-chord")
            .width(70.0)
            .selected_text(state.roll.chord.unwrap_or("Note"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.roll.chord, None, "Note");
                for (name, _) in musiccraft_project::music::CHORDS {
                    ui.selectable_value(&mut state.roll.chord, Some(name), name);
                }
            })
            .response
            .on_hover_text("Click draws a single note or a chord");
        let scale_label = match state.roll.scale {
            Some((root, i)) => format!(
                "{} {}",
                key_name(root).trim_end_matches(char::is_numeric),
                musiccraft_project::music::SCALES[i].0
            ),
            None => "No scale".to_owned(),
        };
        egui::ComboBox::from_id_salt("roll-scale")
            .width(120.0)
            .selected_text(scale_label)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.roll.scale, None, "No scale");
                for root in 0..12_u8 {
                    let name = key_name(root);
                    let name = name.trim_end_matches(char::is_numeric);
                    for (i, (scale, _)) in musiccraft_project::music::SCALES.iter().enumerate() {
                        ui.selectable_value(
                            &mut state.roll.scale,
                            Some((root, i)),
                            format!("{name} {scale}"),
                        );
                    }
                }
            })
            .response
            .on_hover_text("Highlight the keys of a scale");
        ui.label("Velocity");
        ui.add(egui::DragValue::new(&mut state.roll.velocity).range(1..=127));
        ui.separator();
        if ui.button("−").on_hover_text("Zoom out").clicked() {
            state.roll.px_per_step = (state.roll.px_per_step / 1.25).max(6.0);
        }
        if ui.button("+").on_hover_text("Zoom in").clicked() {
            state.roll.px_per_step = (state.roll.px_per_step * 1.25).min(120.0);
        }
        ui.separator();
        if ui
            .button("Quantize")
            .on_hover_text("Snap note starts to the grid")
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "note.quantize",
                [number.clone(), format!("{}t", state.roll.snap.ticks())],
            )));
        }
        if ui.button("Oct −").clicked() {
            actions.push(Action::Run(Invocation::new(
                "note.transpose",
                [number.clone(), "-12".into()],
            )));
        }
        if ui.button("Oct +").clicked() {
            actions.push(Action::Run(Invocation::new(
                "note.transpose",
                [number.clone(), "12".into()],
            )));
        }
        if ui
            .button(RichText::new("Clear").color(theme::DANGER))
            .clicked()
        {
            actions.push(Action::Run(Invocation::new("note.clear", [number])));
        }
    });
}

fn canvas(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    let project = frame.project;
    let channel_index = state.selected_channel;
    let Some(channel) = project.channels.get(channel_index) else {
        return;
    };
    let pattern = project.current_pattern();
    let length = pattern.length_ticks();
    let visible_ticks = length.max(2 * TICKS_PER_BAR) + TICKS_PER_BAR;
    let px_per_tick = state.roll.px_per_step / TICKS_PER_STEP as f32;
    let key_h = state.roll.key_height;
    let size = vec2(
        KEYS_WIDTH + visible_ticks as f32 * px_per_tick,
        RULER_HEIGHT + KEYS as f32 * key_h + LANE_HEIGHT,
    );

    let mut scroll = egui::ScrollArea::both()
        .id_salt("roll-scroll")
        .auto_shrink([false, false]);
    if state.roll.centred_for != Some(channel_index) {
        // Centre vertically on the channel's notes, or on C5 when it has none.
        let keys: Vec<u8> = pattern.notes_of(channel.id).map(|n| n.key).collect();
        let target = match (keys.iter().min(), keys.iter().max()) {
            (Some(lo), Some(hi)) => (u32::from(*lo) + u32::from(*hi)) / 2,
            _ => 60,
        };
        let y = RULER_HEIGHT + row_of_key(u8::try_from(target).unwrap_or(60)) as f32 * key_h;
        scroll = scroll.vertical_scroll_offset((y - ui.available_height() / 2.0).max(0.0));
        state.roll.centred_for = Some(channel_index);
    }
    scroll.show(ui, |ui| {
        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
        let clip = ui.clip_rect();
        let grid = Grid {
            origin: pos2(rect.left() + KEYS_WIDTH, rect.top() + RULER_HEIGHT),
            px_per_tick,
            row_height: key_h,
            rows: KEYS,
        };
        state.roll.last_view = Some((grid, clip));
        let painter = ui.painter_at(clip);
        draw_background(&painter, &grid, clip, rect, length);
        if let Some((root, scale)) = state.roll.scale {
            draw_scale(&painter, &grid, clip, rect, root, scale);
        }

        // Other channels' notes, faded, for context.
        for note in pattern.notes.iter().filter(|n| n.channel != channel.id) {
            let r = grid.span(row_of_key(note.key), note.start, note.length);
            painter.rect_filled(r.shrink(1.0), 2.0, Color32::from_white_alpha(18));
        }
        let notes: Vec<&Note> = pattern.notes_of(channel.id).collect();
        let color = theme::palette(channel_index);
        let dragged = state.roll.drag.map(|d| d.original);
        for note in &notes {
            if state.roll.drag.is_some_and(|d| d.kind != DragKind::Create)
                && Some(**note) == dragged
            {
                continue;
            }
            draw_note(
                &painter,
                &grid,
                note.start,
                note.length,
                note.key,
                note.velocity,
                color,
                false,
            );
        }
        if let Some(drag) = state.roll.drag {
            draw_note(
                &painter,
                &grid,
                drag.start,
                drag.length,
                drag.key,
                state.roll.velocity,
                color,
                true,
            );
        }

        if frame.engine.playing && project.play_mode == PlayMode::Pattern {
            let x = grid.x(frame.engine.position_ticks as f32);
            painter.line_segment(
                [pos2(x, clip.top()), pos2(x, clip.bottom())],
                Stroke::new(1.5, theme::PLAYHEAD),
            );
        }

        let boxes: Vec<(usize, Rect)> = notes
            .iter()
            .enumerate()
            .map(|(i, n)| (i, grid.span(row_of_key(n.key), n.start, n.length)))
            .collect();
        edit(
            ui,
            state,
            &grid,
            &response,
            clip,
            &notes,
            &boxes,
            channel_index,
            actions,
        );
        draw_ruler(&painter, &grid, clip, length);
        velocity_lane(ui, state, &grid, clip, &notes, channel_index, actions);
        keyboard(ui, state, &grid, clip, channel_index, actions);
    });
}

#[allow(clippy::too_many_arguments)]
fn draw_note(
    painter: &egui::Painter,
    grid: &Grid,
    start: u32,
    length: u32,
    key: u8,
    velocity: u8,
    color: Color32,
    ghost: bool,
) {
    let rect = grid
        .span(row_of_key(key), start, length)
        .shrink2(vec2(0.5, 1.0));
    let alpha = if ghost {
        150
    } else {
        110 + u32::from(velocity)
    };
    let fill =
        Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), alpha.min(255) as u8);
    painter.rect_filled(rect, 3.0, fill);
    painter.rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_black_alpha(120)),
        egui::StrokeKind::Inside,
    );
    if rect.width() > 28.0 {
        painter.text(
            rect.left_center() + vec2(4.0, 0.0),
            Align2::LEFT_CENTER,
            key_name(key),
            FontId::proportional(9.5),
            theme::BACKGROUND,
        );
    }
}

fn draw_background(painter: &egui::Painter, grid: &Grid, clip: Rect, rect: Rect, length: u32) {
    let first = grid.row_at(clip.top().max(grid.origin.y)).unwrap_or(0);
    let last = grid.row_at(clip.bottom() - 1.0).unwrap_or(KEYS - 1);
    for row in first..=last {
        let key = key_of_row(row);
        let y = grid.y(row);
        let fill = if is_black(key) {
            theme::BLACK_KEY_ROW
        } else {
            theme::WHITE_KEY_ROW
        };
        painter.rect_filled(
            Rect::from_min_max(
                pos2(grid.origin.x, y),
                pos2(rect.right(), y + grid.row_height),
            ),
            0.0,
            fill,
        );
        if key % 12 == 0 {
            painter.line_segment(
                [
                    pos2(grid.origin.x, y + grid.row_height),
                    pos2(rect.right(), y + grid.row_height),
                ],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
        }
    }
    // Past the pattern end is shaded.
    let end = grid.x(length as f32);
    painter.rect_filled(
        Rect::from_min_max(pos2(end, grid.origin.y), rect.right_bottom()),
        0.0,
        Color32::from_black_alpha(60),
    );
    let first_tick = grid.tick_at(clip.left()) as u32 / TICKS_PER_STEP * TICKS_PER_STEP;
    let last_tick = grid.tick_at(clip.right()) as u32;
    for tick in (first_tick..=last_tick).step_by(TICKS_PER_STEP as usize) {
        let color = if tick % TICKS_PER_BAR == 0 {
            theme::GRID_BAR
        } else if tick % PPQ == 0 {
            theme::GRID_BEAT
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

fn draw_scale(
    painter: &egui::Painter,
    grid: &Grid,
    clip: Rect,
    rect: Rect,
    root: u8,
    scale: usize,
) {
    let Some((_, degrees)) = musiccraft_project::music::SCALES.get(scale) else {
        return;
    };
    let first = grid.row_at(clip.top().max(grid.origin.y)).unwrap_or(0);
    let last = grid.row_at(clip.bottom() - 1.0).unwrap_or(KEYS - 1);
    for row in first..=last {
        let key = key_of_row(row);
        if musiccraft_project::music::in_scale(key, root, degrees) {
            let y = grid.y(row);
            let tint = if key % 12 == root % 12 { 22 } else { 12 };
            painter.rect_filled(
                Rect::from_min_max(
                    pos2(grid.origin.x, y),
                    pos2(rect.right(), y + grid.row_height),
                ),
                0.0,
                Color32::from_rgba_unmultiplied(0x2e, 0xc4, 0xb6, tint),
            );
        }
    }
}

fn draw_ruler(painter: &egui::Painter, grid: &Grid, clip: Rect, length: u32) {
    let ruler = Rect::from_min_max(
        pos2(clip.left(), clip.top()),
        pos2(clip.right(), clip.top() + RULER_HEIGHT),
    );
    painter.rect_filled(ruler, 0.0, theme::PANEL_DARK);
    let first_bar = grid.tick_at(clip.left()) as u32 / TICKS_PER_BAR;
    let last_bar = grid.tick_at(clip.right()) as u32 / TICKS_PER_BAR + 1;
    for bar in first_bar..=last_bar {
        let x = grid.x((bar * TICKS_PER_BAR) as f32);
        let color = if bar * TICKS_PER_BAR < length {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        painter.text(
            pos2(x + 3.0, ruler.center().y),
            Align2::LEFT_CENTER,
            (bar + 1).to_string(),
            FontId::monospace(11.0),
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn edit(
    ui: &egui::Ui,
    state: &mut UiState,
    grid: &Grid,
    response: &egui::Response,
    clip: Rect,
    notes: &[&Note],
    boxes: &[(usize, Rect)],
    channel: usize,
    actions: &mut Vec<Action>,
) {
    let editable = |p: Pos2| {
        p.x > clip.left() + KEYS_WIDTH
            && p.y > clip.top() + RULER_HEIGHT
            && p.y < clip.bottom() - LANE_HEIGHT
    };
    let snap = state.roll.snap;
    let at =
        |p: Pos2| -> (f32, Option<u8>) { (grid.tick_at(p.x), grid.row_at(p.y).map(key_of_row)) };

    if response.drag_started_by(PointerButton::Primary) {
        let origin = ui
            .input(|i| i.pointer.press_origin())
            .filter(|p| editable(*p));
        if let Some(origin) = origin {
            let (tick, key) = at(origin);
            let hit = hit_test(boxes.iter().copied(), origin);
            state.roll.drag = match hit {
                Hit::Body(i) | Hit::Edge(i) => notes.get(i).map(|n| NoteDrag {
                    kind: if matches!(hit, Hit::Edge(_)) {
                        DragKind::Resize
                    } else {
                        DragKind::Move
                    },
                    original: **n,
                    grab: tick - n.start as f32,
                    start: n.start,
                    key: n.key,
                    length: n.length,
                }),
                Hit::Empty => key.map(|key| {
                    let start = snap.floor(tick);
                    NoteDrag {
                        kind: DragKind::Create,
                        original: Note {
                            channel: musiccraft_project::ChannelId(0),
                            start,
                            length: state.roll.last_length,
                            key,
                            velocity: state.roll.velocity,
                        },
                        grab: tick - start as f32,
                        start,
                        key,
                        length: state.roll.last_length,
                    }
                }),
            };
            if let Some(drag) = state.roll.drag {
                actions.push(Action::Preview {
                    channel,
                    key: drag.key,
                    on: true,
                });
                state.roll.preview = Some((channel, drag.key));
            }
        }
    }
    if response.dragged_by(PointerButton::Primary) {
        if let (Some(drag), Some(pos)) = (&mut state.roll.drag, response.interact_pointer_pos()) {
            let (tick, key) = at(pos);
            match drag.kind {
                DragKind::Create | DragKind::Move => {
                    drag.start = snap.floor(tick - drag.grab + snap.ticks() as f32 / 2.0);
                    if let Some(key) = key {
                        if key != drag.key {
                            drag.key = key;
                            if let Some((ch, old)) = state.roll.preview.replace((channel, key)) {
                                actions.push(Action::Preview {
                                    channel: ch,
                                    key: old,
                                    on: false,
                                });
                            }
                            actions.push(Action::Preview {
                                channel,
                                key,
                                on: true,
                            });
                        }
                    }
                }
                DragKind::Resize => {
                    let end = snap
                        .round(tick)
                        .max(drag.start + snap.ticks().min(TICKS_PER_STEP));
                    drag.length = end.saturating_sub(drag.start).max(1);
                }
            }
        }
    }
    if response.drag_stopped_by(PointerButton::Primary) {
        if let Some(drag) = state.roll.drag.take() {
            if drag.kind != DragKind::Move {
                state.roll.last_length = drag.length;
            }
            if let Some(invocation) = finish(&drag, channel, state.roll.velocity) {
                actions.push(Action::Run(invocation));
            }
        }
    }
    if !response.dragged() {
        if let Some((ch, key)) = state.roll.preview.take() {
            actions.push(Action::Preview {
                channel: ch,
                key,
                on: false,
            });
        }
    }
    if response.clicked_by(PointerButton::Primary) {
        if let Some(pos) = response.interact_pointer_pos().filter(|p| editable(*p)) {
            if let (Hit::Empty, (tick, Some(key))) = (hit_test(boxes.iter().copied(), pos), at(pos))
            {
                actions.push(Action::Run(Invocation::new(
                    "note.add",
                    [
                        (channel + 1).to_string(),
                        format!("{}t", snap.floor(tick)),
                        format!("{}t", state.roll.last_length),
                        key.to_string(),
                        state.roll.velocity.to_string(),
                    ],
                )));
                actions.push(Action::PreviewShort { channel, key });
            }
        }
    }
    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Hit::Body(i) | Hit::Edge(i) = hit_test(boxes.iter().copied(), pos) {
                if let Some(note) = notes.get(i) {
                    actions.push(Action::Run(Invocation::new(
                        "note.remove",
                        [
                            (channel + 1).to_string(),
                            format!("{}t", note.start),
                            note.key.to_string(),
                        ],
                    )));
                }
            }
        }
    }
    if let Some(pos) = response.hover_pos().filter(|p| editable(*p)) {
        let cursor = match hit_test(boxes.iter().copied(), pos) {
            Hit::Edge(_) => egui::CursorIcon::ResizeHorizontal,
            Hit::Body(_) => egui::CursorIcon::Grab,
            Hit::Empty => egui::CursorIcon::Crosshair,
        };
        ui.ctx().set_cursor_icon(cursor);
    }
}

/// Velocity of a pointer height inside the lane.
fn lane_velocity(lane: Rect, y: f32) -> u8 {
    let t = ((lane.bottom() - 4.0 - y) / (lane.height() - 8.0)).clamp(0.0, 1.0);
    (1.0 + t * 126.0).round() as u8
}

/// The note whose stem is under `x` (the latest-starting note covering it).
fn note_at_x<'a>(grid: &Grid, notes: &[&'a Note], x: f32) -> Option<&'a Note> {
    let tick = grid.tick_at(x);
    notes
        .iter()
        .filter(|n| (n.start as f32) <= tick && tick < (n.start + n.length) as f32)
        .max_by_key(|n| (n.start, n.key))
        .copied()
}

fn velocity_lane(
    ui: &mut egui::Ui,
    state: &mut UiState,
    grid: &Grid,
    clip: Rect,
    notes: &[&Note],
    channel: usize,
    actions: &mut Vec<Action>,
) {
    let lane = Rect::from_min_max(
        pos2(clip.left() + KEYS_WIDTH, clip.bottom() - LANE_HEIGHT),
        clip.right_bottom(),
    );
    let response = ui.interact(lane, ui.id().with("velocity-lane"), Sense::click_and_drag());
    let strip = Rect::from_min_max(pos2(clip.left(), lane.top()), lane.right_bottom());
    let painter = ui.painter_at(strip);
    painter.rect_filled(strip, 0.0, theme::PANEL_DARK);
    painter.line_segment(
        [strip.left_top(), strip.right_top()],
        Stroke::new(1.0, theme::GRID_BAR),
    );
    painter.text(
        pos2(clip.left() + 6.0, lane.center().y),
        Align2::LEFT_CENTER,
        "Velocity",
        FontId::proportional(10.0),
        theme::TEXT_DIM,
    );
    if response.drag_started() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            state.roll.lane_drag = note_at_x(grid, notes, pos.x).copied();
            state.begin_gesture();
        }
    }
    let mut shown: Option<(Note, u8)> = None;
    if let (Some(note), Some(pos)) = (state.roll.lane_drag, response.interact_pointer_pos()) {
        if response.dragged() || response.clicked() {
            let velocity = lane_velocity(lane, pos.y);
            shown = Some((note, velocity));
            if velocity != note.velocity {
                actions.push(Action::Coalesced(
                    Invocation::new(
                        "note.set_velocity",
                        [
                            (channel + 1).to_string(),
                            format!("{}t", note.start),
                            note.key.to_string(),
                            velocity.to_string(),
                        ],
                    ),
                    state.gesture(),
                ));
            }
        }
    }
    if response.drag_stopped() || (!response.dragged() && !response.clicked()) {
        state.roll.lane_drag = None;
    }
    let color = theme::palette(channel);
    for note in notes {
        let velocity = match shown {
            Some((n, v)) if n.start == note.start && n.key == note.key => v,
            _ => note.velocity,
        };
        let x = grid.x(note.start as f32) + 2.0;
        if x < lane.left() || x > lane.right() {
            continue;
        }
        let top =
            lane.bottom() - 4.0 - f32::from(velocity.max(1) - 1) / 126.0 * (lane.height() - 8.0);
        painter.line_segment(
            [pos2(x, lane.bottom()), pos2(x, top)],
            Stroke::new(2.0, color),
        );
        painter.circle_filled(pos2(x, top), 3.0, color);
    }
}

fn keyboard(
    ui: &mut egui::Ui,
    state: &mut UiState,
    grid: &Grid,
    clip: Rect,
    channel: usize,
    actions: &mut Vec<Action>,
) {
    let keys_rect = Rect::from_min_max(
        pos2(clip.left(), clip.top() + RULER_HEIGHT),
        pos2(clip.left() + KEYS_WIDTH, clip.bottom() - LANE_HEIGHT),
    );
    let response = ui.interact(
        keys_rect,
        ui.id().with("roll-keys"),
        Sense::click_and_drag(),
    );
    let painter = ui.painter_at(keys_rect);
    let first = grid.row_at(keys_rect.top()).unwrap_or(0);
    let last = grid.row_at(keys_rect.bottom() - 1.0).unwrap_or(KEYS - 1);
    let held = state.roll.preview.map(|(_, k)| k);
    for row in first..=last {
        let key = key_of_row(row);
        let y = grid.y(row);
        let r = Rect::from_min_max(
            pos2(keys_rect.left(), y),
            pos2(keys_rect.right(), y + grid.row_height),
        );
        let fill = if held == Some(key) {
            theme::ACCENT
        } else if is_black(key) {
            Color32::from_rgb(0x24, 0x28, 0x30)
        } else {
            Color32::from_rgb(0xd5, 0xd9, 0xe0)
        };
        painter.rect_filled(r.shrink2(vec2(0.0, 0.5)), 1.0, fill);
        if key % 12 == 0 {
            painter.text(
                r.right_center() - vec2(3.0, 0.0),
                Align2::RIGHT_CENTER,
                key_name(key),
                FontId::proportional(9.0),
                theme::BACKGROUND,
            );
        }
    }
    let pressed_key = response
        .interact_pointer_pos()
        .filter(|_| response.is_pointer_button_down_on())
        .and_then(|p| grid.row_at(p.y))
        .map(key_of_row);
    match (pressed_key, state.roll.preview) {
        (Some(key), None) => {
            actions.push(Action::Preview {
                channel,
                key,
                on: true,
            });
            state.roll.preview = Some((channel, key));
        }
        (Some(key), Some((ch, old))) if old != key && state.roll.drag.is_none() => {
            actions.push(Action::Preview {
                channel: ch,
                key: old,
                on: false,
            });
            actions.push(Action::Preview {
                channel,
                key,
                on: true,
            });
            state.roll.preview = Some((channel, key));
        }
        (None, Some((ch, old))) if state.roll.drag.is_none() => {
            actions.push(Action::Preview {
                channel: ch,
                key: old,
                on: false,
            });
            state.roll.preview = None;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_project::ChannelId;

    fn drag(kind: DragKind, start: u32, key: u8, length: u32) -> NoteDrag {
        NoteDrag {
            kind,
            original: Note {
                channel: ChannelId(1),
                start: 24,
                length: 24,
                key: 60,
                velocity: 100,
            },
            grab: 0.0,
            start,
            key,
            length,
        }
    }

    #[test]
    fn gestures_become_single_commands() {
        let create = finish(&drag(DragKind::Create, 48, 62, 12), 0, 90).unwrap();
        assert_eq!(create.to_string(), "note.add 1 48t 12t 62 90");
        let moved = finish(&drag(DragKind::Move, 96, 64, 24), 2, 100).unwrap();
        assert_eq!(moved.to_string(), "note.move 3 24t 60 96t 64");
        let resized = finish(&drag(DragKind::Resize, 24, 60, 72), 0, 100).unwrap();
        assert_eq!(resized.to_string(), "note.resize 1 24t 60 72t");
        assert!(
            finish(&drag(DragKind::Move, 24, 60, 24), 0, 100).is_none(),
            "no-op move"
        );
        assert!(
            finish(&drag(DragKind::Resize, 24, 60, 24), 0, 100).is_none(),
            "no-op resize"
        );
    }

    #[test]
    fn lane_maps_height_to_velocity() {
        let lane = Rect::from_min_max(pos2(0.0, 100.0), pos2(500.0, 164.0));
        assert_eq!(lane_velocity(lane, 160.0), 1);
        assert_eq!(lane_velocity(lane, 104.0), 127);
        assert_eq!(lane_velocity(lane, 50.0), 127);
        assert_eq!(lane_velocity(lane, 132.0), 64);
    }

    #[test]
    fn rows_map_to_keys_top_down() {
        assert_eq!(key_of_row(0), 127);
        assert_eq!(key_of_row(127), 0);
        assert_eq!(row_of_key(60), 67);
        assert!(is_black(61) && !is_black(60));
    }
}
