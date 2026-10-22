//! The shared colour picker: one grid of palette swatches and "No colour",
//! used for playlist tracks and patterns (`docs/spec/style/design-language.md`).

use egui::{Sense, Stroke, vec2};

use crate::theme;

/// Swatches per row.
const COLUMNS: u8 = 8;

/// Colours the recent row keeps.
const RECENT: usize = 8;

fn recent_id() -> egui::Id {
    egui::Id::new("swatches-recent")
}

/// Draws the grid with `current` (a 0-based palette index) outlined, the
/// recently picked colours under it, then "No colour" and a random pick.
/// Returns the choice: `Some(Some(index))` for a colour, `Some(None)` for
/// "No colour", `None` while nothing is picked.
pub(crate) fn grid(ui: &mut egui::Ui, current: Option<u8>) -> Option<Option<u8>> {
    let mut picked = None;
    ui.spacing_mut().item_spacing = vec2(3.0, 3.0);
    for row in 0..16 / COLUMNS {
        ui.horizontal(|ui| {
            for c in row * COLUMNS..(row + 1) * COLUMNS {
                if swatch(ui, c, current, 16.0) {
                    picked = Some(Some(c));
                }
            }
        });
    }
    let recent: Vec<u8> = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<Vec<u8>>(recent_id()))
        .unwrap_or_default();
    if !recent.is_empty() {
        ui.horizontal(|ui| {
            for &c in &recent {
                if swatch(ui, c, current, 11.0) {
                    picked = Some(Some(c));
                }
            }
        });
    }
    ui.horizontal(|ui| {
        if ui
            .selectable_label(current.is_none(), "No colour")
            .clicked()
        {
            picked = Some(None);
        }
        let dice = crate::icons::button(
            ui,
            crate::icons::Icon::Dice,
            false,
            theme::ACCENT,
            "Random colour: one not picked lately",
        );
        if dice.clicked() {
            let seed = ui.ctx().cumulative_frame_nr();
            picked = Some(Some(random_pick(current, &recent, seed)));
        }
    });
    if let Some(Some(c)) = picked {
        remember(ui.ctx(), c);
    }
    picked
}

/// One swatch of palette colour `c`, `size` points square; true when clicked.
fn swatch(ui: &mut egui::Ui, c: u8, current: Option<u8>, size: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::click());
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
    response
        .on_hover_text(format!("Colour {}", c + 1))
        .clicked()
}

/// Puts `c` first in the recent row.
fn remember(ctx: &egui::Context, c: u8) {
    ctx.data_mut(|d| {
        let recent = d.get_persisted_mut_or_default::<Vec<u8>>(recent_id());
        recent.retain(|&r| r != c);
        recent.insert(0, c);
        recent.truncate(RECENT);
    });
}

/// A palette colour that is neither `current` nor among `recent`, chosen by
/// `seed` (any colour but `current` once every one was used lately).
fn random_pick(current: Option<u8>, recent: &[u8], seed: u64) -> u8 {
    let fresh: Vec<u8> = (0..16)
        .filter(|c| Some(*c) != current && !recent.contains(c))
        .collect();
    let pool: Vec<u8> = if fresh.is_empty() {
        (0..16).filter(|c| Some(*c) != current).collect()
    } else {
        fresh
    };
    // A cheap mix of the seed, so successive frames do not walk the pool.
    let mixed = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> 32;
    pool[(mixed % pool.len() as u64) as usize]
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

    #[test]
    fn random_picks_avoid_the_current_and_recent_colours() {
        let recent = [0, 1, 2, 3, 4, 5, 6, 7];
        for seed in 0..64 {
            let c = super::random_pick(Some(8), &recent, seed);
            assert!(c > 8 && c < 16, "{c}");
        }
        let all: Vec<u8> = (0..16).collect();
        assert_ne!(super::random_pick(Some(3), &all, 5), 3);
    }
}
