//! The spectrum analyser window: the master output's frequency content on a
//! log-frequency scale, with a slowly falling peak line.

use egui::{Pos2, Rect, Stroke, pos2, vec2};

use crate::{Frame, UiState, theme};

/// Lowest and highest level shown.
const TOP_DB: f32 = 0.0;
const BOTTOM_DB: f32 = -90.0;
const F_LOW: f32 = 20.0;
const F_HIGH: f32 = 20_000.0;

/// Shows the analyzer while it is open.
pub(crate) fn show(ctx: &egui::Context, state: &mut UiState, frame: &Frame<'_>) {
    let mut open = state.windows.spectrum;
    if !open {
        return;
    }
    let sample_rate = match frame.settings_info.running_rate {
        0 => 48_000.0,
        rate => rate as f32,
    };
    let id = egui::Id::new("spectrum-analyzer");
    let default = egui::Rect::from_min_size(pos2(240.0, 120.0), vec2(520.0, 240.0));
    crate::chrome::window(ctx, id, "Spectrum analyser", state.workspace, default, true)
        .default_height(default.height())
        .show(ctx, |ui| {
            let folded = crate::chrome::Header::new(id, "Spectrum analyser").show(
                ui,
                &mut open,
                &mut (),
                |_, _| {},
                |_, _| {},
            );
            if folded {
                return;
            }
            let size = vec2(
                ui.available_width().max(200.0),
                ui.available_height().max(140.0),
            );
            let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, theme::LCD_BG);
            grid(&painter, rect);
            let bins = musiccraft_dsp::fft::spectrum_db(&frame.engine.scope);
            smooth(&mut state.spectrum, &bins);
            let bin_hz = sample_rate / (bins.len().max(1) * 2) as f32;
            let width = rect.width() as usize;
            let level_at = |levels: &[f32], x: f32| {
                let f = x_freq(rect, x);
                let bin = (f / bin_hz.max(1e-3)).round() as usize;
                levels.get(bin).copied().unwrap_or(BOTTOM_DB)
            };
            let fill: Vec<Pos2> = (0..=width)
                .map(|i| {
                    let x = rect.left() + i as f32;
                    pos2(x, db_y(rect, level_at(&state.spectrum.0, x)))
                })
                .collect();
            for pair in fill.windows(2) {
                let [a, b] = [pair[0], pair[1]];
                painter.line_segment(
                    [pos2(a.x, rect.bottom()), a],
                    Stroke::new(1.0, theme::ACCENT.gamma_multiply(0.25)),
                );
                painter.line_segment([a, b], Stroke::new(1.5, theme::ACCENT));
            }
            let peaks: Vec<Pos2> = (0..=width)
                .map(|i| {
                    let x = rect.left() + i as f32;
                    pos2(x, db_y(rect, level_at(&state.spectrum.1, x)))
                })
                .collect();
            painter.add(egui::Shape::line(peaks, Stroke::new(1.0, theme::HIGHLIGHT)));
            use crate::hint::Hint;
            if let Some(pointer) = response.hover_pos() {
                let f = x_freq(rect, pointer.x);
                let db = level_at(&state.spectrum.0, pointer.x);
                response.hint(format!("{f:.0} Hz: {db:.0} dB"));
            } else {
                response.hint("Master output by frequency; the thin line holds recent peaks");
            }
        });
    state.windows.spectrum = open;
}

/// Averages new levels into the display and lets peaks fall slowly.
fn smooth(display: &mut (Vec<f32>, Vec<f32>), bins: &[f32]) {
    let (levels, peaks) = display;
    if levels.len() != bins.len() {
        *levels = bins.to_vec();
        *peaks = bins.to_vec();
        return;
    }
    for ((level, peak), new) in levels.iter_mut().zip(peaks.iter_mut()).zip(bins) {
        *level += (new.max(BOTTOM_DB) - *level) * 0.4;
        *peak = (*peak - 0.5).max(*level);
    }
}

fn x_freq(rect: Rect, x: f32) -> f32 {
    let t = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    F_LOW * (F_HIGH / F_LOW).powf(t)
}

fn freq_x(rect: Rect, f: f32) -> f32 {
    rect.left() + rect.width() * (f / F_LOW).ln() / (F_HIGH / F_LOW).ln()
}

fn db_y(rect: Rect, db: f32) -> f32 {
    let t = ((db - BOTTOM_DB) / (TOP_DB - BOTTOM_DB)).clamp(0.0, 1.0);
    rect.bottom() - t * rect.height()
}

fn grid(painter: &egui::Painter, rect: Rect) {
    let font = egui::FontId::proportional(theme::FONT_VALUE);
    for (f, label) in [
        (50.0, "50"),
        (100.0, "100"),
        (200.0, "200"),
        (500.0, "500"),
        (1_000.0, "1k"),
        (2_000.0, "2k"),
        (5_000.0, "5k"),
        (10_000.0, "10k"),
    ] {
        let x = freq_x(rect, f);
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, theme::GRID_LINE),
        );
        painter.text(
            pos2(x + 2.0, rect.bottom() - 2.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            font.clone(),
            theme::TEXT_DIM,
        );
    }
    for db in [-12.0, -24.0, -36.0, -48.0, -60.0, -72.0] {
        let y = db_y(rect, db);
        painter.line_segment(
            [pos2(rect.left(), y), pos2(rect.right(), y)],
            Stroke::new(1.0, theme::GRID_LINE),
        );
        painter.text(
            pos2(rect.left() + 2.0, y),
            egui::Align2::LEFT_BOTTOM,
            format!("{db:.0}"),
            font.clone(),
            theme::TEXT_DIM,
        );
    }
}
