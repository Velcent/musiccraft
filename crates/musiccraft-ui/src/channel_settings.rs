//! Channel settings: the instrument editor for the selected channel.

use egui::{Color32, Pos2, RichText, Sense, Shape, Stroke, pos2, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::music::key_name;
use musiccraft_project::params::{INSTRUMENT_PARAMS, ParamSpec, WAVEFORMS, inst};
use musiccraft_project::{Channel, DEFAULT_CHANNEL_VOLUME, Instrument, MAX_VOLUME};

use crate::dialogs::Dialog;
use crate::{Action, Frame, UiState, theme, widgets};

pub(crate) fn to_unit(spec: &ParamSpec, value: f32) -> f32 {
    spec.to_unit(value)
}

pub(crate) fn from_unit(spec: &ParamSpec, t: f32) -> f32 {
    spec.from_unit(t)
}

pub(crate) fn format_value(spec: &ParamSpec, value: f32) -> String {
    spec.format(value)
}

fn spec(index: usize) -> ParamSpec {
    INSTRUMENT_PARAMS[index]
}

/// A parameter knob bound to `channel.set_param`.
fn param_knob(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    index: usize,
    label: &str,
    actions: &mut Vec<Action>,
) {
    let spec = spec(index);
    let value = channel.params.get(index);
    let out = widgets::labeled_knob(
        ui,
        label,
        to_unit(&spec, value),
        to_unit(&spec, spec.default),
        spec.min < 0.0 && spec.max > 0.0,
        &format_value(&spec, value),
    );
    crate::automation::menu(
        &out.response,
        &format!("channel:{number}:{}", spec.name),
        actions,
    );
    if out.started {
        state.begin_gesture();
    }
    if let Some(t) = out.changed {
        let new = from_unit(&spec, t);
        if new != value {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_param",
                    [number.to_string(), spec.name.to_owned(), format!("{new}")],
                ),
                state.gesture(),
            ));
        }
    }
}

fn section(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(theme::PANEL_DARK)
        .corner_radius(6.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(title).small().strong().color(theme::TEXT_DIM));
            add(ui);
        });
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let mut open = state.windows.channel_settings;
    let index = state.selected_channel;
    let Some(channel) = frame.project.channels.get(index) else {
        return;
    };
    let area = state.workspace;
    egui::Window::new(format!("Channel — {}", channel.name))
        .id(egui::Id::new("channel-settings"))
        .open(&mut open)
        .default_pos(area.right_top() + vec2(-470.0, 30.0))
        .default_width(440.0)
        .resizable(false)
        .constrain_to(area)
        .show(ctx, |ui| {
            contents(ui, state, frame, index, channel, actions)
        });
    state.windows.channel_settings = open;
}

fn contents(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    index: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    let number = index + 1;
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(&channel.name)
                .size(18.0)
                .strong()
                .color(theme::palette(index)),
        );
        ui.label(RichText::new(channel.instrument.label()).color(theme::TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let track = channel.mixer_track;
            let name = frame
                .project
                .mixer
                .get(track)
                .map_or("", |t| t.name.as_str());
            ui.label(
                RichText::new(format!("Mixer {track} · {name}"))
                    .small()
                    .color(theme::TEXT_DIM),
            );
        });
    });
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        let volume = widgets::labeled_knob(
            ui,
            "Volume",
            channel.volume / MAX_VOLUME,
            DEFAULT_CHANNEL_VOLUME / MAX_VOLUME,
            false,
            &format!("{:.0}%", channel.volume * 100.0),
        );
        crate::automation::menu(
            &volume.response,
            &format!("channel:{number}:volume"),
            actions,
        );
        if volume.started {
            state.begin_gesture();
        }
        if let Some(t) = volume.changed {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_volume",
                    [number.to_string(), format!("{:.3}", t * MAX_VOLUME)],
                ),
                state.gesture(),
            ));
        }
        let pan = widgets::labeled_knob(
            ui,
            "Pan",
            (channel.pan + 1.0) / 2.0,
            0.5,
            true,
            &format!("{:+.0}", channel.pan * 100.0),
        );
        crate::automation::menu(&pan.response, &format!("channel:{number}:pan"), actions);
        if pan.started {
            state.begin_gesture();
        }
        if let Some(t) = pan.changed {
            actions.push(Action::Coalesced(
                Invocation::new(
                    "channel.set_pan",
                    [number.to_string(), format!("{:.3}", t * 2.0 - 1.0)],
                ),
                state.gesture(),
            ));
        }
        param_knob(ui, state, number, channel, inst::PITCH, "Pitch", actions);
        let mut mute = channel.mute;
        if ui.checkbox(&mut mute, "Mute").changed() {
            actions.push(Action::Run(Invocation::new(
                "channel.toggle_mute",
                [number.to_string()],
            )));
        }
    });
    ui.add_space(6.0);
    match &channel.instrument {
        Instrument::Sampler { sample } => {
            sampler(ui, state, frame, index, channel, sample, actions)
        }
        Instrument::Synth => synth(ui, state, number, channel, actions),
        Instrument::Plugin { .. } => {}
    }
    ui.add_space(6.0);
    section(ui, "ENVELOPE", |ui| {
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::ATTACK, "Attack", actions);
            param_knob(ui, state, number, channel, inst::DECAY, "Decay", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::SUSTAIN,
                "Sustain",
                actions,
            );
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::RELEASE,
                "Release",
                actions,
            );
            envelope_graph(ui, channel, theme::palette(index));
        });
    });
}

fn sampler(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    index: usize,
    channel: &Channel,
    sample: &str,
    actions: &mut Vec<Action>,
) {
    let number = index + 1;
    section(ui, "SAMPLE", |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::Label::new(RichText::new(sample).monospace().small()).truncate());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("Change…").clicked() {
                    state.open_dialog(Dialog::set_sample(index, sample.to_owned()));
                }
            });
        });
        waveform(ui, frame.waveform, theme::palette(index));
        ui.horizontal(|ui| {
            let mut one_shot = channel.params.get(inst::ONE_SHOT) >= 0.5;
            if ui
                .checkbox(&mut one_shot, "One shot")
                .on_hover_text("Play the whole sample regardless of note length")
                .changed()
            {
                actions.push(Action::Run(Invocation::new(
                    "channel.set_param",
                    [
                        number.to_string(),
                        "one_shot".to_owned(),
                        if one_shot { "1" } else { "0" }.to_owned(),
                    ],
                )));
            }
            ui.add_space(12.0);
            let root = channel.params.get(inst::ROOT_KEY) as u8;
            ui.label("Root key");
            let mut key = u32::from(root);
            let response = ui.add(
                egui::DragValue::new(&mut key)
                    .range(0..=127)
                    .speed(0.2)
                    .custom_formatter(|v, _| key_name(v as u8)),
            );
            if response.changed() {
                actions.push(Action::Run(Invocation::new(
                    "channel.set_param",
                    [number.to_string(), "root_key".to_owned(), key.to_string()],
                )));
            }
        });
    });
}

fn synth(
    ui: &mut egui::Ui,
    state: &mut UiState,
    number: usize,
    channel: &Channel,
    actions: &mut Vec<Action>,
) {
    section(ui, "OSCILLATORS", |ui| {
        for (label, param) in [("Osc 1", inst::WAVE1), ("Osc 2", inst::WAVE2)] {
            ui.horizontal(|ui| {
                ui.add_sized(
                    vec2(44.0, 20.0),
                    egui::Label::new(RichText::new(label).small()),
                );
                let current = channel.params.get(param).round() as usize;
                for (wave, name) in WAVEFORMS.iter().enumerate() {
                    if wave_button(ui, wave, wave == current)
                        .on_hover_text(*name)
                        .clicked()
                        && wave != current
                    {
                        actions.push(Action::Run(Invocation::new(
                            "channel.set_param",
                            [
                                number.to_string(),
                                INSTRUMENT_PARAMS[param].name.to_owned(),
                                wave.to_string(),
                            ],
                        )));
                    }
                }
            });
        }
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::DETUNE, "Detune", actions);
            param_knob(ui, state, number, channel, inst::GLIDE, "Glide", actions);
            param_knob(ui, state, number, channel, inst::OSC_MIX, "Mix", actions);
        });
    });
    ui.add_space(6.0);
    section(ui, "FILTER", |ui| {
        ui.horizontal(|ui| {
            param_knob(ui, state, number, channel, inst::CUTOFF, "Cutoff", actions);
            param_knob(ui, state, number, channel, inst::RESONANCE, "Reso", actions);
            param_knob(
                ui,
                state,
                number,
                channel,
                inst::ENV_AMOUNT,
                "Env amt",
                actions,
            );
        });
    });
}

/// A button showing one oscillator wave shape.
fn wave_button(ui: &mut egui::Ui, wave: usize, selected: bool) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, 22.0), Sense::click());
    let fill = if selected {
        theme::ACCENT_DIM
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    let r = rect.shrink2(vec2(6.0, 5.0));
    let at = |x: f32, y: f32| {
        pos2(
            r.left() + r.width() * x,
            r.center().y - r.height() / 2.0 * y,
        )
    };
    let points: Vec<Pos2> = match wave {
        0 => (0..=24)
            .map(|i| {
                let x = i as f32 / 24.0;
                at(x, (x * std::f32::consts::TAU).sin())
            })
            .collect(),
        1 => vec![at(0.0, -1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, 1.0)],
        2 => vec![at(0.0, 1.0), at(0.5, 1.0), at(0.5, -1.0), at(1.0, -1.0)],
        3 => vec![at(0.0, 0.0), at(0.25, 1.0), at(0.75, -1.0), at(1.0, 0.0)],
        _ => (0..=16)
            .map(|i| {
                at(
                    i as f32 / 16.0,
                    [
                        0.2, -0.7, 0.9, -0.3, 0.6, -1.0, 0.1, 0.8, -0.5, 0.3, -0.9, 0.7, -0.2, 0.5,
                        -0.6, 0.9, -0.1,
                    ][i],
                )
            })
            .collect(),
    };
    let color = if selected {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    ui.painter()
        .add(Shape::line(points, Stroke::new(1.5, color)));
    response
}

/// Min/max overview of the sample.
fn waveform(ui: &mut egui::Ui, overview: Option<&[(f32, f32)]>, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 72.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    painter.line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, theme::LCD_EDGE),
    );
    let Some(points) = overview.filter(|p| !p.is_empty()) else {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "no audio",
            egui::FontId::proportional(11.0),
            theme::TEXT_DIM,
        );
        return;
    };
    let peak = points
        .iter()
        .fold(0.0_f32, |p, (lo, hi)| p.max(lo.abs()).max(hi.abs()))
        .max(1e-6);
    let step = rect.width() / points.len() as f32;
    for (i, (lo, hi)) in points.iter().enumerate() {
        let x = rect.left() + (i as f32 + 0.5) * step;
        let half = rect.height() * 0.45 / peak;
        painter.line_segment(
            [
                pos2(x, rect.center().y - hi * half),
                pos2(x, rect.center().y - lo * half - 0.5),
            ],
            Stroke::new(step.max(1.0), color),
        );
    }
}

/// The envelope shape: attack, decay, a fixed sustain segment, release.
fn envelope_graph(ui: &mut egui::Ui, channel: &Channel, color: Color32) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width().max(100.0), 56.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let a = channel.params.get(inst::ATTACK);
    let d = channel.params.get(inst::DECAY);
    let s = channel.params.get(inst::SUSTAIN).clamp(0.0, 1.0);
    let r = channel.params.get(inst::RELEASE);
    // Time axis compressed with a square root so short stages stay visible.
    let w = |t: f32| t.max(0.0).sqrt();
    let hold = 0.6;
    let total = w(a) + w(d) + hold + w(r);
    let inner = rect.shrink(6.0);
    let x = |t: f32| inner.left() + t / total.max(1e-6) * inner.width();
    let y = |level: f32| inner.bottom() - level * inner.height();
    let points = vec![
        pos2(x(0.0), y(0.0)),
        pos2(x(w(a)), y(1.0)),
        pos2(x(w(a) + w(d)), y(s)),
        pos2(x(w(a) + w(d) + hold), y(s)),
        pos2(x(total), y(0.0)),
    ];
    painter.add(Shape::line(points, Stroke::new(2.0, color)));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knob_mappings_round_trip() {
        for (index, value) in [
            (inst::CUTOFF, 1_000.0),
            (inst::ATTACK, 0.05),
            (inst::SUSTAIN, 0.4),
            (inst::DETUNE, -300.0),
        ] {
            let spec = spec(index);
            let back = from_unit(&spec, to_unit(&spec, value));
            assert!(
                (back - value).abs() / value.abs().max(1.0) < 1e-3,
                "{} {value} -> {back}",
                spec.name
            );
        }
        let cutoff = spec(inst::CUTOFF);
        assert!(
            (to_unit(&cutoff, 632.5) - 0.5).abs() < 0.01,
            "log scale centre"
        );
        assert_eq!(format_value(&cutoff, 2_500.0), "2.50 kHz");
        assert_eq!(format_value(&spec(inst::ATTACK), 0.25), "250 ms");
        assert_eq!(format_value(&spec(inst::SUSTAIN), 0.5), "50%");
    }
}
