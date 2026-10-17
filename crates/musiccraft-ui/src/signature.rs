//! The time signature picker used by the toolbar, the pattern list and the
//! playlist ruler.

use musiccraft_project::TimeSig;
use musiccraft_project::meter::{DENOMINATORS, MAX_NUMERATOR};

use crate::hint::Hint;

/// Signatures offered first.
const COMMON: [&str; 9] = [
    "2/4", "3/4", "4/4", "5/4", "6/8", "7/8", "9/8", "12/8", "2/2",
];

/// Menu contents: common signatures, then a custom numerator and
/// denominator kept in `draft`. Returns the signature picked.
pub(crate) fn picker(
    ui: &mut egui::Ui,
    current: Option<TimeSig>,
    draft: &mut (u32, u32),
) -> Option<TimeSig> {
    let mut picked = None;
    ui.horizontal_wrapped(|ui| {
        ui.set_max_width(180.0);
        for text in COMMON {
            let Ok(sig) = text.parse::<TimeSig>() else {
                continue;
            };
            if ui.selectable_label(current == Some(sig), text).clicked() {
                picked = Some(sig);
            }
        }
    });
    ui.separator();
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut draft.0).range(1..=MAX_NUMERATOR))
            .hint("Beats per bar");
        ui.label("/");
        egui::ComboBox::from_id_salt("signature-denominator")
            .width(44.0)
            .selected_text(draft.1.to_string())
            .show_ui(ui, |ui| {
                for den in DENOMINATORS {
                    ui.selectable_value(&mut draft.1, den, den.to_string());
                }
            })
            .response
            .hint("Beat note value");
        if ui.button("Set").clicked() {
            picked = TimeSig::new(draft.0, draft.1);
        }
    });
    if picked.is_some() {
        ui.close();
    }
    picked
}
