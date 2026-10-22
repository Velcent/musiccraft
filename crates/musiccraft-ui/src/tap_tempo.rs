//! The tap tempo window: a large pad to tap the beat on, a bar that fills
//! as the run of taps grows, and the tempo it measured.

use egui::{Sense, pos2, vec2};
use musiccraft_commands::Invocation;
use musiccraft_commands::tap_tempo::{MAX_TAPS, RESET_SECONDS};

use crate::hint::Hint;
use crate::{Action, UiState, theme};

/// Counts a tap now and sets the tempo once two or more taps give one. A run
/// of taps is one undo step.
pub(crate) fn tap(ui: &egui::Ui, state: &mut UiState, actions: &mut Vec<Action>) {
    let now = ui.input(|i| i.time);
    if state.taps.last().is_none_or(|t| now - t > RESET_SECONDS) {
        state.begin_gesture();
    }
    if let Some(bpm) = musiccraft_commands::tap_tempo::tap(&mut state.taps, now) {
        actions.push(Action::Coalesced(
            Invocation::new("project.set_tempo", [format!("{bpm:.2}")]),
            state.gesture(),
        ));
    }
}

/// Shows the window while it is open.
pub(crate) fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    tempo: f64,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.tap_tempo;
    if !open {
        return;
    }
    let id = egui::Id::new("tap-tempo");
    let default = egui::Rect::from_min_size(pos2(300.0, 140.0), vec2(220.0, 170.0));
    crate::chrome::window(ctx, id, "Tap tempo", state.workspace, default, false)
        .default_height(default.height())
        .show(ctx, |ui| {
            let folded = crate::chrome::Header::new(id, "Tap tempo").show(
                ui,
                &mut open,
                &mut (),
                |_, _| {},
                |_, _| {},
            );
            if !folded {
                body(ui, state, tempo, actions);
            }
        });
    state.windows.tap_tempo = open;
}

fn body(ui: &mut egui::Ui, state: &mut UiState, tempo: f64, actions: &mut Vec<Action>) {
    let now = ui.input(|i| i.time);
    let live = state.taps.last().is_some_and(|t| now - t <= RESET_SECONDS);
    let count = if live { state.taps.len() } else { 0 };
    let width = ui.available_width().max(160.0);
    let (pad, response) = ui.allocate_exact_size(vec2(width, 90.0), Sense::click());
    let pressed = response.is_pointer_button_down_on();
    let fill = if pressed {
        theme::ACCENT
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    let painter = ui.painter().clone();
    painter.rect_filled(pad, 6.0, fill);
    painter.text(
        pad.center(),
        egui::Align2::CENTER_CENTER,
        "TAP",
        egui::FontId::proportional(theme::FONT_READOUT),
        theme::text_on(fill),
    );
    if response.is_pointer_button_down_on() && ui.input(|i| i.pointer.any_pressed()) {
        tap(ui, state, actions);
    }
    response.hint("Tap the beat here; a pause over two seconds starts a new count");
    ui.add_space(4.0);
    let (bar, _) = ui.allocate_exact_size(vec2(width, 6.0), Sense::hover());
    painter.rect_filled(bar, 3.0, theme::PANEL_DARK);
    let share = count as f32 / MAX_TAPS as f32;
    let mut full = bar;
    full.set_width(bar.width() * share.min(1.0));
    painter.rect_filled(full, 3.0, theme::ACCENT);
    ui.add_space(2.0);
    let line = match count {
        0 => "Tap at least twice".to_owned(),
        1 => "1 tap · keep going".to_owned(),
        n => format!("{tempo:.2} BPM · {n} taps"),
    };
    ui.label(egui::RichText::new(line).color(theme::TEXT_DIM));
    if live {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(200));
    }
}
