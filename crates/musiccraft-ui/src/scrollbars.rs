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
    let left = Rect::from_min_size(rect.min, vec2(BAR, rect.height()));
    let right = Rect::from_min_size(pos2(rect.right() - BAR, rect.top()), left.size());
    let rail = Rect::from_min_max(left.right_top(), right.left_bottom());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rail, 0.0, theme::PANEL_DARK);
    let (x0, x1) = thumb_range(view, rail.left(), rail.width());
    let thumb = Rect::from_x_y_ranges(x0..=x1, rail.y_range()).shrink2(vec2(0.0, 2.0));
    let rail_response = ui
        .interact(rail, id.with("rail"), Sense::click())
        .hint("Click to page · drag the thumb to scroll · drag its ends to zoom");
    let body = ui.interact(thumb, id.with("thumb"), Sense::drag());
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
    painter.rect_filled(thumb, CornerRadius::same(3), thumb_fill(&body));
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
    overview(&painter, rail.shrink2(vec2(0.0, 2.0)));

    let mut request = None;
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
