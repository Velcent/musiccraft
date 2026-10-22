//! The shared scroll and zoom bars (`docs/spec/style/main-window.md`, "Scroll
//! and zoom bars"): chevrons at both ends, a recessed rail, a thumb whose
//! length is the visible share, zoom grips on the horizontal thumb, and the
//! vertical zoom box. Everything here works in content units; the caller
//! turns the requests into scroll offsets and zoom levels.

use crate::hint::Hint;
use egui::{CornerRadius, Id, Painter, Rect, Response, Sense, Stroke, Ui, pos2, vec2};

use crate::icons::{self, Icon};
use crate::theme;

/// Thickness of a bar, and the side of a zoom box, in points.
pub const BAR: f32 = 14.0;
/// Width of the zoom grip at each end of the horizontal thumb.
const GRIP: f32 = 5.0;
/// The shortest thumb, so it stays easy to grab.
const MIN_THUMB: f32 = 14.0;

/// The visible part of the content: it starts at `first` and covers `span`
/// of `total`, all in the same unit (ticks, key rows…).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Span {
    pub first: f32,
    pub span: f32,
    pub total: f32,
}

/// What a gesture on a bar asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Request {
    /// Scroll so that the view starts here.
    To(f32),
    /// Show exactly this range (a zoom grip was dragged).
    Show { first: f32, span: f32 },
}

/// True when a held button should act: at the press, then repeatedly after
/// a short delay.
fn repeat(ui: &Ui, response: &Response) -> bool {
    let id = response.id.with("repeat");
    if !response.is_pointer_button_down_on() {
        ui.data_mut(|d| d.remove::<f64>(id));
        return false;
    }
    ui.ctx().request_repaint();
    let now = ui.input(|i| i.time);
    let due = match ui.data(|d| d.get_temp::<f64>(id)) {
        None => Some(now + 0.4),
        Some(at) if now >= at => Some(now + 0.08),
        Some(_) => None,
    };
    if let Some(next) = due {
        ui.data_mut(|d| d.insert_temp(id, next));
    }
    due.is_some()
}

fn chevron(ui: &Ui, rect: Rect, id: Id, icon: Icon, hint: &str) -> bool {
    let response = ui.interact(rect, id, Sense::click()).hint(hint);
    let fill = if response.is_pointer_button_down_on() {
        theme::ACCENT_DIM
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    ui.painter()
        .rect_filled(rect.shrink(1.0), CornerRadius::same(2), fill);
    icons::paint(ui.painter(), rect, icon, theme::TEXT_DIM);
    repeat(ui, &response)
}

fn thumb_fill(response: &Response) -> egui::Color32 {
    if response.dragged() {
        theme::ACCENT_DIM
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    }
}

/// Where `span` sits along a rail from `start` to `start + length`, with the
/// thumb kept at least `MIN_THUMB` long.
fn thumb_range(view: Span, start: f32, length: f32) -> (f32, f32) {
    let total = view.total.max(f32::EPSILON);
    let size = (view.span / total * length).clamp(MIN_THUMB.min(length), length);
    let free = (length - size).max(0.0);
    let hidden = (total - view.span).max(f32::EPSILON);
    let at = start + (view.first / hidden).clamp(0.0, 1.0) * free;
    (at, at + size)
}

/// Content position under a point along the rail.
fn at_rail(view: Span, start: f32, length: f32, x: f32) -> f32 {
    (x - start) / length.max(1.0) * view.total
}

/// The horizontal bar across `rect`: left chevron, rail, thumb with zoom
/// grips, right chevron. `step` is what one chevron click scrolls;
/// `overview` paints a miniature of the content into the rail.
pub fn horizontal(
    ui: &Ui,
    rect: Rect,
    id: Id,
    (view, step): (Span, f32),
    overview: impl FnOnce(&Painter, Rect),
) -> Option<Request> {
    horizontal_bar(ui, rect, id, (view, step), Some(overview))
}

/// The simplified horizontal bar (channel rack, mixer): chevrons, rail and
/// thumb, with no zoom grips. Returns where the view should start.
pub fn plain_horizontal(ui: &Ui, rect: Rect, id: Id, (view, step): (Span, f32)) -> Option<f32> {
    let none: Option<fn(&Painter, Rect)> = None;
    match horizontal_bar(ui, rect, id, (view, step), none) {
        Some(Request::To(first)) => Some(first),
        _ => None,
    }
}

/// The horizontal bar; zoom grips and the overview come with `overview`.
fn horizontal_bar(
    ui: &Ui,
    rect: Rect,
    id: Id,
    (view, step): (Span, f32),
    overview: Option<impl FnOnce(&Painter, Rect)>,
) -> Option<Request> {
    let zooms = overview.is_some();
    let left = Rect::from_min_size(rect.min, vec2(BAR, rect.height()));
    let right = Rect::from_min_size(pos2(rect.right() - BAR, rect.top()), left.size());
    let rail = Rect::from_min_max(left.right_top(), right.left_bottom());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rail, 0.0, theme::PANEL_DARK);
    let (x0, x1) = thumb_range(view, rail.left(), rail.width());
    let thumb = Rect::from_x_y_ranges(x0..=x1, rail.y_range()).shrink2(vec2(0.0, 2.0));
    let rail_response = ui
        .interact(rail, id.with("rail"), Sense::click())
        .hint(if zooms {
            "Click to page · drag the thumb to scroll · drag its ends to zoom"
        } else {
            "Click to page · drag the thumb to scroll"
        });
    let body = ui.interact(thumb, id.with("thumb"), Sense::drag());
    painter.rect_filled(thumb, CornerRadius::same(3), thumb_fill(&body));
    let mut request = None;
    if let Some(overview) = overview {
        request = grips(ui, &painter, id, thumb, (view, rail));
        overview(&painter, rail.shrink2(vec2(0.0, 2.0)));
    }
    if chevron(ui, left, id.with("left"), Icon::ChevronLeft, "Scroll left") {
        request = Some(Request::To(view.first - step));
    }
    if chevron(
        ui,
        right,
        id.with("right"),
        Icon::ChevronRight,
        "Scroll right",
    ) {
        request = Some(Request::To(view.first + step));
    }
    if rail_response.clicked() {
        if let Some(p) = rail_response.interact_pointer_pos() {
            let page = if p.x < thumb.left() {
                -view.span
            } else {
                view.span
            };
            request = Some(Request::To(view.first + page));
        }
    }
    if body.dragged() {
        let dx = body.drag_delta().x;
        request = Some(Request::To(
            view.first + dx / rail.width().max(1.0) * view.total,
        ));
    }
    request
}

/// The zoom grips at both ends of the horizontal thumb.
fn grips(
    ui: &Ui,
    painter: &Painter,
    id: Id,
    thumb: Rect,
    (view, rail): (Span, Rect),
) -> Option<Request> {
    let grips = [
        Rect::from_min_size(thumb.min, vec2(GRIP, thumb.height())),
        Rect::from_min_size(
            pos2(thumb.right() - GRIP, thumb.top()),
            vec2(GRIP, thumb.height()),
        ),
    ];
    let grip_responses = [0, 1].map(|i| {
        ui.interact(grips[i], id.with(("grip", i)), Sense::drag())
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
    });
    for (grip, response) in grips.iter().zip(&grip_responses) {
        let tone = if response.hovered() || response.dragged() {
            theme::TEXT_DIM
        } else {
            theme::TEXT_FAINT
        };
        let x = grip.center().x;
        painter.line_segment(
            [pos2(x, grip.top() + 2.0), pos2(x, grip.bottom() - 2.0)],
            Stroke::new(1.0, tone),
        );
    }
    let mut request = None;
    let end = view.first + view.span;
    let pointer = |r: &Response| {
        r.interact_pointer_pos()
            .map(|p| at_rail(view, rail.left(), rail.width(), p.x))
    };
    if let (true, Some(at)) = (grip_responses[0].dragged(), pointer(&grip_responses[0])) {
        let first = at.clamp(0.0, end - 1.0);
        request = Some(Request::Show {
            first,
            span: end - first,
        });
    }
    if let (true, Some(at)) = (grip_responses[1].dragged(), pointer(&grip_responses[1])) {
        request = Some(Request::Show {
            first: view.first,
            span: (at - view.first).max(1.0),
        });
    }
    request
}

/// The vertical bar down `rect`: up chevron, rail, thumb, down chevron.
/// Returns where the view should start.
pub fn vertical(ui: &Ui, rect: Rect, id: Id, (view, step): (Span, f32)) -> Option<f32> {
    let top = Rect::from_min_size(rect.min, vec2(rect.width(), BAR));
    let bottom = Rect::from_min_size(pos2(rect.left(), rect.bottom() - BAR), top.size());
    let rail = Rect::from_min_max(top.left_bottom(), bottom.right_top());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rail, 0.0, theme::PANEL_DARK);
    let (y0, y1) = thumb_range(view, rail.top(), rail.height());
    let thumb = Rect::from_x_y_ranges(rail.x_range(), y0..=y1).shrink2(vec2(2.0, 0.0));
    let rail_response = ui
        .interact(rail, id.with("rail"), Sense::click())
        .hint("Click to page · drag the thumb to scroll");
    let body = ui.interact(thumb, id.with("thumb"), Sense::drag());
    painter.rect_filled(thumb, CornerRadius::same(3), thumb_fill(&body));

    let mut request = None;
    if chevron(ui, top, id.with("up"), Icon::ChevronUp, "Scroll up") {
        request = Some(view.first - step);
    }
    if chevron(
        ui,
        bottom,
        id.with("down"),
        Icon::ChevronDown,
        "Scroll down",
    ) {
        request = Some(view.first + step);
    }
    if rail_response.clicked() {
        if let Some(p) = rail_response.interact_pointer_pos() {
            let page = if p.y < thumb.top() {
                -view.span
            } else {
                view.span
            };
            request = Some(view.first + page);
        }
    }
    if body.dragged() {
        let dy = body.drag_delta().y;
        request = Some(view.first + dy / rail.height().max(1.0) * view.total);
    }
    request
}

/// What the zoom box was asked to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoomBox {
    /// Dragged by this many points (negative is up, which zooms in).
    Drag(f32),
    /// Double-clicked: back to the default.
    Reset,
}

/// The vertical zoom box: two lines whose gap shows `level` (0 to 1).
pub fn zoom_box(ui: &Ui, rect: Rect, id: Id, level: f32, hint: &str) -> Option<ZoomBox> {
    let response = ui
        .interact(rect, id, Sense::click_and_drag())
        .on_hover_cursor(egui::CursorIcon::ResizeVertical)
        .hint(hint);
    let painter = ui.painter();
    let well = rect.shrink(1.0);
    painter.rect_filled(well, CornerRadius::same(2), theme::LCD_BG);
    let edge = if response.hovered() {
        theme::TEXT_FAINT
    } else {
        theme::LCD_EDGE
    };
    painter.rect_stroke(
        well,
        CornerRadius::same(2),
        Stroke::new(1.0, edge),
        egui::StrokeKind::Inside,
    );
    let lines = if response.dragged() {
        theme::ACCENT
    } else {
        theme::TEXT_DIM
    };
    let gap = 1.0 + level.clamp(0.0, 1.0) * (well.height() - 5.0);
    let mid = well.center().y;
    for y in [mid - gap / 2.0, mid + gap / 2.0] {
        painter.line_segment(
            [pos2(well.left() + 3.0, y), pos2(well.right() - 3.0, y)],
            Stroke::new(1.0, lines),
        );
    }
    if response.double_clicked() {
        Some(ZoomBox::Reset)
    } else if response.dragged() && response.drag_delta().y != 0.0 {
        Some(ZoomBox::Drag(response.drag_delta().y))
    } else {
        None
    }
}

/// `area` with the simplified bars in place of the toolkit's: a vertical bar
/// at the right and a horizontal bar underneath, each shown only while the
/// content overflows that way. Chevrons step by `step` points.
pub fn plain_area<R>(
    ui: &mut Ui,
    area: egui::ScrollArea,
    bars: (Id, f32),
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    plain(ui, area, bars, false, add)
}

/// [`plain_area`] with the vertical bar at the left, for a column docked at
/// the left edge (the bar sits on its outer side).
pub fn plain_area_left<R>(
    ui: &mut Ui,
    area: egui::ScrollArea,
    bars: (Id, f32),
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    plain(ui, area, bars, true, add)
}

fn plain<R>(
    ui: &mut Ui,
    area: egui::ScrollArea,
    (id, step): (Id, f32),
    left: bool,
    add: impl FnOnce(&mut Ui) -> R,
) -> R {
    let key = id.with("plain-bars");
    let (over_x, over_y, jump) = ui
        .data(|d| d.get_temp::<(bool, bool, Option<egui::Vec2>)>(key))
        .unwrap_or_default();
    let mut area = area.scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden);
    if let Some(jump) = jump {
        area = area.scroll_offset(jump);
    }
    if over_y {
        area = area.max_width(ui.available_width() - BAR);
    }
    let output = if over_y && left {
        let full = ui.available_rect_before_wrap();
        let body = Rect::from_min_max(full.min + vec2(BAR, 0.0), full.max);
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(body).layout(*ui.layout()));
        let output = area.show(&mut child, add);
        ui.advance_cursor_after_rect(child.min_rect());
        output
    } else {
        area.show(ui, add)
    };
    let inner = output.inner_rect;
    let content = output.content_size;
    let offset = output.state.offset;
    let mut next = None;
    if over_y {
        let x = if left {
            inner.left() - BAR
        } else {
            inner.right()
        };
        let rect = Rect::from_min_size(egui::pos2(x, inner.top()), vec2(BAR, inner.height()));
        let view = Span {
            first: offset.y,
            span: inner.height(),
            total: content.y,
        };
        if let Some(first) = vertical(ui, rect, id.with("v"), (view, step)) {
            let y = first.clamp(0.0, (content.y - inner.height()).max(0.0));
            next = Some(vec2(offset.x, y));
        }
    }
    if over_x {
        let (rect, _) = ui.allocate_exact_size(vec2(inner.width(), BAR), Sense::hover());
        let view = Span {
            first: offset.x,
            span: inner.width(),
            total: content.x,
        };
        if let Some(first) = plain_horizontal(ui, rect, id.with("h"), (view, step)) {
            let x = first.clamp(0.0, (content.x - inner.width()).max(0.0));
            next = Some(vec2(x, next.map_or(offset.y, |n| n.y)));
        }
    }
    let state = (
        content.x > inner.width() + 0.5,
        content.y > inner.height() + 0.5,
        next,
    );
    if state != (over_x, over_y, None) {
        ui.ctx().request_repaint();
    }
    ui.data_mut(|d| {
        d.insert_temp(key, state);
        d.insert_temp(id.with("offset"), offset);
    });
    output.inner
}

/// Mouse-wheel notches this frame (up positive), one per wheel event
/// whatever its size, so stepped views move by whole items.
pub fn wheel_notches(ui: &Ui) -> f32 {
    ui.input(|i| {
        i.raw
            .events
            .iter()
            .filter_map(|e| match e {
                // Some systems turn Shift+wheel into a sideways scroll.
                egui::Event::MouseWheel { delta, .. } if delta.y != 0.0 => Some(delta.y.signum()),
                egui::Event::MouseWheel { delta, .. } if delta.x != 0.0 => Some(delta.x.signum()),
                _ => None,
            })
            .sum::<f32>()
    })
}

/// Scrolls the `plain_area` with bars `id` by `delta` points next frame.
pub fn plain_step(ctx: &egui::Context, id: Id, delta: egui::Vec2) {
    let key = id.with("plain-bars");
    let offset: egui::Vec2 = ctx
        .data(|d| d.get_temp(id.with("offset")))
        .unwrap_or_default();
    ctx.data_mut(|d| {
        let (x, y, _) = d
            .get_temp::<(bool, bool, Option<egui::Vec2>)>(key)
            .unwrap_or_default();
        d.insert_temp(key, (x, y, Some((offset + delta).max(egui::Vec2::ZERO))));
    });
    ctx.request_repaint();
}

/// The scroll offset `plain_area` had in the last frame.
#[cfg(test)]
pub fn plain_offset(ctx: &egui::Context, id: Id) -> egui::Vec2 {
    ctx.data(|d| d.get_temp(id.with("offset")))
        .unwrap_or_default()
}

/// An editor's view and the bars around it.
#[derive(Debug, Clone, Copy)]
pub struct Setup {
    /// Size of the visible view and its scroll offset, in points.
    pub view: egui::Vec2,
    pub offset: egui::Vec2,
    /// Fixed panes inside the view: the left column (keys, track headers),
    /// the ruler on top and a lane at the bottom.
    pub lead: f32,
    pub top: f32,
    pub tail: f32,
    /// Horizontal: points per unit, units in all, one chevron step, and the
    /// zoom range in points per unit.
    pub x: (f32, f32, f32, (f32, f32)),
    /// Vertical: row height, rows, the zoom range and the default height.
    pub y: (f32, f32, (f32, f32), f32),
    /// What the zoom box sets ("Key height", "Track height").
    pub y_label: &'static str,
}

/// What the bars ask the editor to change; it applies them next frame.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Outcome {
    /// New scroll offset, in points.
    pub jump: Option<egui::Vec2>,
    /// New horizontal zoom, in points per unit.
    pub scale: Option<f32>,
    /// New row height, in points.
    pub row: Option<f32>,
}

/// Middle-drag pans a view both ways: while the middle button, pressed
/// inside `view`, is held, the content follows the pointer. Returns the new
/// scroll offset (from `offset`, this frame's: the view's top-left minus the
/// content's) when it moved. It reads the
/// pointer rather than a response, so it works over notes and clips too.
pub fn middle_pan(ui: &Ui, view: Rect, offset: egui::Vec2) -> Option<egui::Vec2> {
    let (down, origin, delta) = ui.input(|i| {
        (
            i.pointer.button_down(egui::PointerButton::Middle),
            i.pointer.press_origin(),
            i.pointer.delta(),
        )
    });
    if !down || !origin.is_some_and(|o| view.contains(o)) {
        return None;
    }
    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    (delta != egui::Vec2::ZERO).then(|| (offset - delta).max(egui::Vec2::ZERO))
}

/// The view's rect inside `body`: everything but the scroll/zoom row along
/// the top and the vertical bar down the right.
pub fn view_rect(body: Rect) -> Rect {
    Rect::from_min_max(
        pos2(body.left(), body.top() + BAR),
        pos2(body.right() - BAR, body.bottom()),
    )
}

/// Draws the scroll/zoom row, the zoom box and the vertical bar around the
/// view in `body`, and turns their gestures into an [`Outcome`].
pub fn around(
    ui: &Ui,
    body: Rect,
    id: Id,
    setup: Setup,
    overview: impl FnOnce(&Painter, Rect, Span),
) -> Outcome {
    let (scale, total, step, (lo_x, hi_x)) = setup.x;
    let (row, rows_total, (lo_y, hi_y), default_row) = setup.y;
    let grid_w = (setup.view.x - setup.lead).max(1.0);
    let grid_h = (setup.view.y - setup.top - setup.tail).max(1.0);
    let across = Span {
        first: setup.offset.x / scale,
        span: grid_w / scale,
        total,
    };
    let down = Span {
        first: setup.offset.y / row,
        span: grid_h / row,
        total: rows_total,
    };
    let mut out = Outcome::default();
    let mut x = setup.offset.x;
    let mut y = setup.offset.y;
    let hbar = Rect::from_min_max(
        pos2(body.left() + setup.lead, body.top()),
        pos2(body.right() - BAR, body.top() + BAR),
    );
    let request = horizontal(ui, hbar, id.with("h"), (across, step), |painter, rail| {
        overview(painter, rail, across);
    });
    match request {
        Some(Request::To(first)) => {
            x = first.clamp(0.0, (total - across.span).max(0.0)) * scale;
            out.jump = Some(vec2(x, y));
        }
        Some(Request::Show { first, span }) => {
            let new = (grid_w / span.max(f32::EPSILON)).clamp(lo_x, hi_x);
            x = first.max(0.0) * new;
            out.scale = Some(new);
            out.jump = Some(vec2(x, y));
        }
        None => {}
    }
    let vbar = Rect::from_min_max(
        pos2(body.right() - BAR, body.top() + BAR),
        pos2(body.right(), body.bottom() - setup.tail),
    );
    if let Some(first) = vertical(ui, vbar, id.with("v"), (down, 1.0)) {
        y = first.clamp(0.0, (rows_total - down.span).max(0.0)) * row;
        out.jump = Some(vec2(x, y));
    }
    let zoom = Rect::from_min_size(pos2(body.right() - BAR, body.top()), vec2(BAR, BAR));
    let label = setup.y_label;
    let asked = zoom_box(
        ui,
        zoom,
        id.with("zoom"),
        (row - lo_y) / (hi_y - lo_y),
        &format!("{label}: {row:.0} pt · drag up or down to zoom, double-click to reset"),
    );
    let new_row = match asked {
        Some(ZoomBox::Drag(dy)) => Some((row - dy * 0.15).clamp(lo_y, hi_y)),
        Some(ZoomBox::Reset) => Some(default_row),
        None => None,
    };
    if let Some(new_row) = new_row {
        // Keep the row in the middle of the view where it is.
        let centre = down.first + down.span / 2.0;
        y = (centre - grid_h / new_row / 2.0).max(0.0) * new_row;
        out.row = Some(new_row);
        out.jump = Some(vec2(x, y));
    }
    // Below the vertical bar, beside the bottom lane, the column stays plain.
    let rest = Rect::from_min_max(vbar.left_bottom(), body.right_bottom());
    ui.painter().rect_filled(rest, 0.0, theme::PANEL_DARK);
    if out != Outcome::default() {
        ui.ctx().request_repaint();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_thumb_shows_the_visible_share() {
        let view = Span {
            first: 0.0,
            span: 25.0,
            total: 100.0,
        };
        assert_eq!(thumb_range(view, 0.0, 200.0), (0.0, 50.0));
        let end = Span {
            first: 75.0,
            ..view
        };
        assert_eq!(thumb_range(end, 0.0, 200.0), (150.0, 200.0));
        // A tiny share still gets a thumb that can be grabbed.
        let tiny = Span {
            first: 0.0,
            span: 1.0,
            total: 1000.0,
        };
        let (a, b) = thumb_range(tiny, 0.0, 200.0);
        assert!(b - a >= MIN_THUMB);
    }
}
