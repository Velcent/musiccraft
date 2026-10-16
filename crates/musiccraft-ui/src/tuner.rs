//! The display of the pitch corrector and tuner windows: the note being
//! heard, how far off it is, and (for the corrector) the scale it is pulled
//! to.

use egui::{Color32, Rect, Sense, Stroke, pos2};
use musiccraft_dsp::pitch::{hz_to_note, nearest_in_scale, note_and_cents};
use musiccraft_project::EffectSlot;
use musiccraft_project::params::{CORRECTION_SCALES, EffectKind, NOTE_NAMES, index_of};

use crate::hint::Hint;
use crate::theme;

/// Cents either side of the centre of the meter.
const SPAN_CENTS: f32 = 50.0;
/// Within this many cents a note counts as in tune.
const IN_TUNE_CENTS: f32 = 5.0;

fn value(effect: &EffectSlot, name: &str) -> f32 {
    index_of(effect.kind.params(), name)
        .and_then(|i| effect.values.get(i))
        .copied()
        .unwrap_or_default()
}

/// The pitch classes (bit `n` = `n` semitones above C) a corrector pulls to.
#[must_use]
pub(crate) fn scale_classes(effect: &EffectSlot) -> u16 {
    let key = value(effect, "key").round().clamp(0.0, 11.0) as u32;
    let mask = CORRECTION_SCALES
        .get(value(effect, "scale").round().max(0.0) as usize)
        .map_or(0x0FFF, |(_, mask)| *mask);
    // Rotate the key-relative mask so bit 0 is C.
    ((u32::from(mask) << key | u32::from(mask) >> (12 - key)) & 0x0FFF) as u16
}

/// Draws the tuner display for `effect`, given the pitch the engine heard.
pub(crate) fn show(
    ui: &mut egui::Ui,
    effect: &EffectSlot,
    detected: Option<f32>,
    size: egui::Vec2,
) {
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let reference = value(effect, "reference");
    let corrector = effect.kind == EffectKind::PitchCorrect;
    let meter = Rect::from_min_max(
        pos2(rect.left() + 16.0, rect.top() + 64.0),
        pos2(rect.right() - 16.0, rect.top() + 84.0),
    );
    cents_scale(&painter, meter);
    match detected.filter(|_| effect.enabled) {
        Some(hz) => {
            let (name, cents) = note_and_cents(hz, reference);
            let color = if cents.abs() <= IN_TUNE_CENTS {
                theme::ACCENT
            } else {
                theme::HIGHLIGHT
            };
            let mut heading = name;
            if corrector {
                let note = hz_to_note(hz, reference);
                let target = nearest_in_scale(note, 0, scale_classes(effect));
                let (target_name, _) =
                    note_and_cents(reference * 2.0_f32.powf((target - 69.0) / 12.0), reference);
                if target_name != heading || cents.abs() > IN_TUNE_CENTS {
                    heading = format!("{heading} → {target_name}");
                }
            }
            painter.text(
                pos2(rect.center().x, rect.top() + 8.0),
                egui::Align2::CENTER_TOP,
                heading,
                egui::FontId::proportional(28.0),
                color,
            );
            painter.text(
                pos2(rect.center().x, rect.top() + 44.0),
                egui::Align2::CENTER_TOP,
                format!("{hz:.1} Hz   {cents:+.0} ct"),
                egui::FontId::proportional(12.0),
                theme::TEXT,
            );
            let t = (cents / SPAN_CENTS).clamp(-1.0, 1.0);
            let x = meter.center().x + t * meter.width() * 0.5;
            painter.line_segment(
                [pos2(x, meter.top() - 4.0), pos2(x, meter.bottom() + 4.0)],
                Stroke::new(3.0, color),
            );
        }
        None => {
            painter.text(
                pos2(rect.center().x, rect.top() + 18.0),
                egui::Align2::CENTER_TOP,
                if effect.enabled {
                    "—  no clear pitch  —"
                } else {
                    "bypassed"
                },
                egui::FontId::proportional(16.0),
                theme::TEXT_DIM,
            );
        }
    }
    if corrector {
        scale_strip(&painter, rect, scale_classes(effect), detected, reference);
    } else {
        painter.text(
            pos2(rect.center().x, rect.bottom() - 6.0),
            egui::Align2::CENTER_BOTTOM,
            format!("A = {reference:.0} Hz"),
            egui::FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
    }
    response.hint(if corrector {
        "The note heard, where it is pulled to, and how far off it is; lit keys belong to the scale"
    } else {
        "The note heard and how far it is from the nearest note (±50 cents across)"
    });
}

/// Tick marks every 10 cents with the centre emphasized.
fn cents_scale(painter: &egui::Painter, meter: Rect) {
    painter.rect_filled(meter, 2.0, theme::METER_BG);
    for step in -5..=5 {
        let x = meter.center().x + step as f32 / 5.0 * meter.width() * 0.5;
        let (height, color) = if step == 0 {
            (meter.height(), theme::TEXT)
        } else {
            (meter.height() * 0.5, theme::GRID_BAR)
        };
        painter.line_segment(
            [
                pos2(x, meter.center().y - height * 0.5),
                pos2(x, meter.center().y + height * 0.5),
            ],
            Stroke::new(1.0, color),
        );
    }
}

/// Twelve keys from C to B; scale notes lit, the heard pitch class marked.
fn scale_strip(
    painter: &egui::Painter,
    rect: Rect,
    classes: u16,
    detected: Option<f32>,
    reference: f32,
) {
    let top = rect.bottom() - 34.0;
    let width = (rect.width() - 32.0) / 12.0;
    let heard = detected.map(|hz| (hz_to_note(hz, reference).round() as i32).rem_euclid(12));
    for (i, name) in NOTE_NAMES.iter().enumerate() {
        let left = rect.left() + 16.0 + width * i as f32;
        let key = Rect::from_min_max(pos2(left + 1.0, top), pos2(left + width - 1.0, top + 26.0));
        let lit = classes & (1 << i) != 0;
        let fill = if lit {
            theme::ACCENT_DIM
        } else {
            theme::PANEL_DARK
        };
        painter.rect_filled(key, 2.0, fill);
        if heard == Some(i as i32) {
            painter.rect_stroke(
                key,
                2.0,
                Stroke::new(2.0, theme::HIGHLIGHT),
                egui::StrokeKind::Inside,
            );
        }
        let text: Color32 = if lit { theme::TEXT } else { theme::TEXT_DIM };
        painter.text(
            key.center(),
            egui::Align2::CENTER_CENTER,
            *name,
            egui::FontId::proportional(10.0),
            text,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corrector(key: f32, scale: f32) -> EffectSlot {
        let mut slot = EffectSlot::new(EffectKind::PitchCorrect);
        if let Some(i) = index_of(EffectKind::PitchCorrect.params(), "key") {
            slot.values[i] = key;
        }
        if let Some(i) = index_of(EffectKind::PitchCorrect.params(), "scale") {
            slot.values[i] = scale;
        }
        slot
    }

    #[test]
    fn scale_classes_follow_the_key() {
        assert_eq!(scale_classes(&corrector(0.0, 0.0)), 0x0FFF, "chromatic");
        let c_major = scale_classes(&corrector(0.0, 1.0));
        assert_eq!(c_major, 0b1010_1011_0101);
        let d_major = scale_classes(&corrector(2.0, 1.0));
        // D major: D E F# G A B C#.
        for class in [2, 4, 6, 7, 9, 11, 1] {
            assert!(d_major & (1 << class) != 0, "{class} in D major");
        }
        assert!(d_major & 1 == 0, "no C natural in D major");
    }
}
