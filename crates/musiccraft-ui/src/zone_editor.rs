//! The zone sampler's editor: one row per zone with its sample, key range,
//! root key and velocity range. Emits `channel.*_zone` invocations only.

use egui::RichText;
use musiccraft_commands::Invocation;
use musiccraft_project::SampleZone;
use musiccraft_project::music::key_name;

use crate::channel_settings::section;
use crate::dialogs::Dialog;
use crate::hint::Hint;
use crate::{Action, Frame, UiState, theme};

/// A short display name for a sample reference.
fn sample_name(sample: &str) -> String {
    sample
        .strip_prefix(musiccraft_project::params::BUILTIN_PREFIX)
        .map(str::to_owned)
        .unwrap_or_else(|| {
            std::path::Path::new(sample)
                .file_stem()
                .map_or_else(|| sample.to_owned(), |s| s.to_string_lossy().into_owned())
        })
}

/// A dragged whole number bound to one zone field; one undo step per drag.
#[allow(clippy::too_many_arguments)]
fn field(
    ui: &mut egui::Ui,
    state: &mut UiState,
    actions: &mut Vec<Action>,
    channel: usize,
    zone: usize,
    name: &str,
    value: u8,
    range: std::ops::RangeInclusive<u8>,
    show_key: bool,
) {
    let mut edited = value;
    let mut drag = egui::DragValue::new(&mut edited).range(range).speed(0.2);
    if show_key {
        drag = drag.custom_formatter(|v, _| key_name(v as u8));
    }
    let response = ui.add(drag);
    if response.drag_started() {
        state.begin_gesture();
    }
    if response.changed() && edited != value {
        actions.push(Action::Coalesced(
            Invocation::new(
                "channel.set_zone",
                [
                    (channel + 1).to_string(),
                    (zone + 1).to_string(),
                    name.to_owned(),
                    edited.to_string(),
                ],
            ),
            state.gesture(),
        ));
    }
}

pub(crate) fn show(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    channel: usize,
    zones: &[SampleZone],
    actions: &mut Vec<Action>,
) {
    section(ui, "ZONES", |ui| {
        ui.label(
            RichText::new("A note plays the first zone whose keys and velocities contain it.")
                .small()
                .color(theme::TEXT_DIM),
        );
        egui::Grid::new(("zones", channel))
            .spacing([6.0, 4.0])
            .show(ui, |ui| {
                for header in ["", "sample", "keys", "root", "velocity", ""] {
                    ui.label(RichText::new(header).small().color(theme::TEXT_DIM));
                }
                ui.end_row();
                for (z, zone) in zones.iter().enumerate() {
                    ui.label(RichText::new((z + 1).to_string()).small());
                    if ui
                        .button(RichText::new(sample_name(&zone.sample)).small())
                        .hint("Click to hear the zone at its root key")
                        .clicked()
                    {
                        actions.push(Action::PreviewShort {
                            channel,
                            key: zone.root,
                        });
                    }
                    ui.horizontal(|ui| {
                        field(
                            ui,
                            state,
                            actions,
                            channel,
                            z,
                            "low",
                            zone.low,
                            0..=zone.high,
                            true,
                        );
                        ui.label("–");
                        field(
                            ui,
                            state,
                            actions,
                            channel,
                            z,
                            "high",
                            zone.high,
                            zone.low..=127,
                            true,
                        );
                    });
                    field(
                        ui,
                        state,
                        actions,
                        channel,
                        z,
                        "root",
                        zone.root,
                        0..=127,
                        true,
                    );
                    ui.horizontal(|ui| {
                        field(
                            ui,
                            state,
                            actions,
                            channel,
                            z,
                            "vel_low",
                            zone.vel_low,
                            1..=zone.vel_high,
                            false,
                        );
                        ui.label("–");
                        field(
                            ui,
                            state,
                            actions,
                            channel,
                            z,
                            "vel_high",
                            zone.vel_high,
                            zone.vel_low..=127,
                            false,
                        );
                    });
                    if ui.small_button("×").hint("Delete this zone").clicked() {
                        actions.push(Action::Run(Invocation::new(
                            "channel.remove_zone",
                            [(channel + 1).to_string(), (z + 1).to_string()],
                        )));
                    }
                    ui.end_row();
                }
            });
        if ui.button("Add zone…").clicked() {
            state.open_dialog(Dialog::add_zone(channel, frame.project));
        }
    });
}
