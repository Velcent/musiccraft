//! The shared colour picker: one grid of palette swatches and "No colour",
//! used for playlist tracks and patterns (`docs/spec/style/design-language.md`).

use egui::{Sense, Stroke, vec2};

use crate::theme;

/// Swatches per row.
const COLUMNS: u8 = 8;

/// Draws the grid with `current` (a 0-based palette index) outlined. Returns
/// the choice: `Some(Some(index))` for a colour, `Some(None)` for "No
/// colour", `None` while nothing is picked.
pub(crate) fn grid(ui: &mut egui::Ui, current: Option<u8>) -> Option<Option<u8>> {
    let mut picked = None;
    ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
    for row in 0..16 / COLUMNS {
        ui.horizontal(|ui| {
            for c in row * COLUMNS..(row + 1) * COLUMNS {
                let (rect, response) = ui.allocate_exact_size(vec2(16.0, 16.0), Sense::click());
                ui.painter().rect_filled(rect, 2.0, theme::track_color(c));
                let edge = if current == Some(c) {
                    Some(theme::TEXT)
                } else if response.hovered() {
                    Some(theme::TEXT_DIM)
                } else {
                    None
                };
                if let Some(edge) = edge {
                    ui.painter().rect_stroke(
                        rect.expand(1.0),
                        3.0,
                        Stroke::new(1.5, edge),
                        egui::StrokeKind::Outside,
                    );
                }
                if response
                    .on_hover_text(format!("Colour {}", c + 1))
                    .clicked()
                {
                    picked = Some(Some(c));
                }
            }
        });
    }
    if ui
        .selectable_label(current.is_none(), "No colour")
        .clicked()
    {
        picked = Some(None);
    }
    picked
}

/// The command argument for a choice from [`grid`].
pub(crate) fn arg(choice: Option<u8>) -> String {
    choice.map_or_else(|| "none".to_owned(), |c| (c + 1).to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn choices_become_one_based_colour_arguments() {
        assert_eq!(super::arg(Some(0)), "1");
        assert_eq!(super::arg(Some(15)), "16");
        assert_eq!(super::arg(None), "none");
    }
}
