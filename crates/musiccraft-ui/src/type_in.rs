//! The type-in value box that "Enter exact value…" in the control menu opens
//! under a knob or fader (`docs/spec/controls.md`, "Type-in value box").

use egui::{Align2, Key, Modifiers, Pos2, RichText, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::Project;

use crate::icons::{self, Icon};
use crate::{Action, UiState, theme};

/// An open type-in box.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeIn {
    /// The control, named as for `control.set`.
    target: String,
    /// Its readable name, shown in the footer.
    label: String,
    /// The value line, in percent of the range.
    text: String,
    /// The position the fine slider shows, 0..1.
    position: f32,
    /// Where the box opens: under the control.
    anchor: Pos2,
}

/// Opens the box for `target` under `anchor`.
pub(crate) fn open(state: &mut UiState, project: &Project, target: &str, anchor: Pos2) {
    if let Some(info) = musiccraft_commands::control_info(project, target) {
        state.type_in = Some(TypeIn {
            target: target.to_owned(),
            label: info.label,
            text: percent(info.position),
            position: info.position,
            anchor,
        });
    }
}

fn percent(position: f32) -> String {
    format!("{:.1}", position * 100.0)
}

/// The position typed, clamped to the range; `None` when it is not a number.
fn parse(text: &str) -> Option<f32> {
    let number = text
        .trim()
        .trim_end_matches('%')
        .trim()
        .parse::<f32>()
        .ok()?;
    number.is_finite().then(|| (number / 100.0).clamp(0.0, 1.0))
}

/// Draws the open box. Enter, the tick or a click outside commits as one
/// undo step; Esc or the cross cancels.
pub(crate) fn show(ctx: &egui::Context, state: &mut UiState, actions: &mut Vec<Action>) {
    let Some(mut open) = state.type_in.take() else {
        return;
    };
    let id = egui::Id::new("type-in");
    let mut commit = false;
    let mut cancel = false;
    let area = egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(open.anchor + vec2(0.0, 2.0))
        .pivot(Align2::LEFT_TOP)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(theme::MENU_BG)
                .stroke(egui::Stroke::new(1.0, theme::WINDOW_EDGE))
                .corner_radius(3.0)
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.set_width(150.0);
                    ui.horizontal(|ui| {
                        let field = ui.add(
                            egui::TextEdit::singleline(&mut open.text)
                                .id(id.with("field"))
                                .desired_width(110.0)
                                .font(egui::FontId::monospace(theme::FONT_DISPLAY)),
                        );
                        if !field.has_focus() && !field.lost_focus() {
                            field.request_focus();
                        }
                        if field.changed()
                            && let Some(position) = parse(&open.text)
                        {
                            open.position = position;
                        }
                        ui.label(RichText::new("%").color(theme::TEXT_DIM));
                    });
                    let slider = ui.add(
                        egui::Slider::new(&mut open.position, 0.0..=1.0)
                            .show_value(false)
                            .drag_value_speed(0.001),
                    );
                    if slider.changed() {
                        open.text = percent(open.position);
                    }
                    ui.horizontal(|ui| {
                        commit |=
                            icons::header_button(ui, Icon::Check, false, false, "Apply").clicked();
                        cancel |=
                            icons::header_button(ui, Icon::Close, false, true, "Cancel").clicked();
                        ui.label(
                            RichText::new(&open.label)
                                .size(theme::FONT_LABEL)
                                .color(theme::TEXT_DIM),
                        );
                    });
                });
        });
    let (enter, escape) = ctx.input_mut(|i| {
        (
            i.consume_key(Modifiers::NONE, Key::Enter),
            i.consume_key(Modifiers::NONE, Key::Escape),
        )
    });
    let clicked_outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p))
    });
    if escape || cancel {
        return;
    }
    if enter || commit || clicked_outside {
        if let Some(position) = parse(&open.text) {
            actions.push(Action::Run(Invocation::new(
                "control.set",
                [open.target, format!("{position}")],
            )));
        }
        return;
    }
    state.type_in = Some(open);
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn typed_percentages_become_clamped_positions() {
        assert_eq!(parse("50"), Some(0.5));
        assert_eq!(parse(" 25 % "), Some(0.25));
        assert_eq!(parse("150"), Some(1.0));
        assert_eq!(parse("-3"), Some(0.0));
        assert_eq!(parse("loud"), None);
    }
}
