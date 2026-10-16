//! The effect rack's window body: one row per branch, each effect a
//! collapsible box with its bypass, remove button and knobs.

use egui::RichText;
use musiccraft_commands::Invocation;
use musiccraft_project::EffectSlot;
use musiccraft_project::params::{EFFECT_KINDS, RACK_BRANCH_EFFECTS, RACK_BRANCHES};

use crate::effect_window::{KnobTarget, knob_row};
use crate::hint::Hint;
use crate::{Action, UiState, theme, widgets};

/// Arguments naming effect `n` of `branch` (all 0-based) for a rack command.
fn place(track: usize, slot: usize, branch: usize, n: usize) -> [String; 4] {
    [
        track.to_string(),
        (slot + 1).to_string(),
        (branch + 1).to_string(),
        (n + 1).to_string(),
    ]
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: usize,
    slot: usize,
    rack: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    ui.set_min_width(420.0);
    for branch in 0..RACK_BRANCHES {
        let effects = rack.branches.get(branch).map_or(&[][..], Vec::as_slice);
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Branch {}", branch + 1)).strong());
            if effects.is_empty() {
                ui.label(RichText::new("empty (silent)").color(theme::TEXT_DIM));
            }
            if effects.len() < RACK_BRANCH_EFFECTS {
                add_menu(ui, track, slot, branch, actions);
            }
        });
        for (n, effect) in effects.iter().enumerate() {
            item(ui, state, (track, slot, branch, n), effect, actions);
        }
        ui.separator();
    }
}

fn add_menu(
    ui: &mut egui::Ui,
    track: usize,
    slot: usize,
    branch: usize,
    actions: &mut Vec<Action>,
) {
    ui.menu_button("Add", |ui| {
        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                for kind in EFFECT_KINDS.iter().filter(|k| k.allowed_in_rack()) {
                    if ui.button(crate::browser::effect_title(*kind)).clicked() {
                        let [t, s, b, _] = place(track, slot, branch, 0);
                        actions.push(Action::Run(Invocation::new(
                            "mixer.rack_add",
                            [t, s, b, kind.name().to_owned()],
                        )));
                        ui.close();
                    }
                }
            });
    })
    .response
    .hint("Append an effect to this branch");
}

fn item(
    ui: &mut egui::Ui,
    state: &mut UiState,
    (track, slot, branch, n): (usize, usize, usize, usize),
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    let id = ui.make_persistent_id(("rack-item", track, slot, branch, n));
    egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false)
        .show_header(ui, |ui| {
            let on = effect.enabled;
            if widgets::dot(ui, on, theme::ACCENT)
                .hint(if on {
                    "On — click to bypass"
                } else {
                    "Bypassed — click to turn on"
                })
                .clicked()
            {
                let [t, s, b, i] = place(track, slot, branch, n);
                let flag = if on { "off" } else { "on" };
                actions.push(Action::Run(Invocation::new(
                    "mixer.rack_enable",
                    [t, s, b, i, flag.to_owned()],
                )));
            }
            ui.label(format!(
                "{}. {}",
                n + 1,
                crate::browser::effect_title(effect.kind)
            ));
            if ui.small_button("×").hint("Remove this effect").clicked() {
                actions.push(Action::Run(Invocation::new(
                    "mixer.rack_remove",
                    place(track, slot, branch, n),
                )));
            }
        })
        .body(|ui| {
            knob_row(
                ui,
                state,
                KnobTarget::Rack(track, slot, branch, n),
                effect,
                actions,
            );
        });
}
