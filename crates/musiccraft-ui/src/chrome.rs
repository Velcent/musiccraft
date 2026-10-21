//! The compact header row of the editor windows: a collapse arrow, the title,
//! the window's own tools and a close cross on one row (see
//! `docs/spec/window-layout.md`).

use crate::hint::Hint;
use egui::{Color32, Sense, Shape, Stroke, pos2, vec2};

use crate::theme;

/// Whether the window `id` is collapsed to its header.
#[must_use]
pub fn collapsed(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.data_mut(|d| d.get_persisted::<bool>(id.with("collapsed")))
        .unwrap_or(false)
}

/// Draws the header of window `id`: collapse arrow, `title`, `tools`, and a
/// close cross that clears `open`. Returns whether the window is collapsed.
pub fn header(
    ui: &mut egui::Ui,
    id: egui::Id,
    title: &str,
    open: &mut bool,
    tools: impl FnOnce(&mut egui::Ui),
) -> bool {
    let mut folded = collapsed(ui.ctx(), id);
    ui.horizontal(|ui| {
        let (rect, arrow) = ui.allocate_exact_size(vec2(14.0, 18.0), Sense::click());
        let color = if arrow.hovered() {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        let c = rect.center();
        let points = if folded {
            vec![
                c + vec2(-2.5, -4.0),
                c + vec2(3.5, 0.0),
                c + vec2(-2.5, 4.0),
            ]
        } else {
            vec![
                c + vec2(-4.0, -2.5),
                c + vec2(4.0, -2.5),
                c + vec2(0.0, 3.5),
            ]
        };
        ui.painter()
            .add(Shape::convex_polygon(points, color, Stroke::NONE));
        if arrow
            .hint(if folded { "Expand" } else { "Collapse" })
            .clicked()
        {
            folded = !folded;
        }
        ui.label(egui::RichText::new(title).size(13.0).color(theme::TEXT));
        ui.add_space(6.0);
        // The cross takes the right end first, so tools that wrap onto a
        // second line stay clear of it.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
            if close(ui).hint("Close").clicked() {
                *open = false;
            }
            ui.add_space(6.0);
            ui.with_layout(
                egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
                tools,
            );
        });
    });
    ui.data_mut(|d| d.insert_persisted(id.with("collapsed"), folded));
    folded
}

/// In a wrapping row, starts a new row unless `width` more points fit. Combo
/// boxes need this: they lay out in a child row of their own, which egui does
/// not wrap.
pub fn fit(ui: &mut egui::Ui, width: f32) {
    if ui.available_size_before_wrap().x < width {
        ui.end_row();
    }
}

fn close(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let color: Color32 = if response.hovered() {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    let r = rect.shrink(5.0);
    let stroke = Stroke::new(1.4, color);
    ui.painter()
        .line_segment([r.left_top(), r.right_bottom()], stroke);
    ui.painter().line_segment(
        [pos2(r.right(), r.top()), pos2(r.left(), r.bottom())],
        stroke,
    );
    response
}
