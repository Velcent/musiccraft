//! Side panel listing sounds and instruments to add to the channel rack.

use egui::RichText;
use musiccraft_commands::Invocation;
use musiccraft_project::params::{BUILTIN_PREFIX, BUILTIN_SAMPLES};

use crate::{Action, Frame, UiState, dialogs::Dialog, theme};

pub fn show(ui: &mut egui::Ui, state: &mut UiState, frame: &Frame<'_>, actions: &mut Vec<Action>) {
    egui::Panel::left("browser")
        .resizable(true)
        .default_size(170.0)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(8.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new("BROWSER")
                    .small()
                    .strong()
                    .color(theme::TEXT_DIM),
            );
            ui.add_space(4.0);
            ui.label(RichText::new("Built-in sounds").strong());
            for name in BUILTIN_SAMPLES {
                let label = name.replace('_', " ");
                let response = ui
                    .add(egui::Button::new(format!("♪  {label}")).frame(false))
                    .on_hover_text("Click to add to the channel rack");
                if response.clicked() {
                    actions.push(Action::Run(Invocation::new(
                        "channel.add_sampler",
                        [format!("{BUILTIN_PREFIX}{name}")],
                    )));
                }
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Instruments").strong());
            if ui.add(egui::Button::new("♪  Synth").frame(false)).clicked() {
                actions.push(Action::Run(Invocation::bare("channel.add_synth")));
            }
            ui.add_space(8.0);
            ui.label(RichText::new("Files").strong());
            if ui.button("Add WAV sample…").clicked() {
                state.open_dialog(Dialog::add_sample(frame.project));
            }
            ui.add_space(12.0);
            ui.separator();
            ui.label(
                RichText::new(
                    "Shortcuts: F5 playlist, F6 rack, F7 piano roll, F9 mixer, Space play",
                )
                .small()
                .color(theme::TEXT_DIM),
            );
        });
}
