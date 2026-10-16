//! Windows for mixer effects: a preset bar with bypass, a graph you can drag
//! (filter and EQ curves, dynamics transfer curves, delay taps…) and the
//! effect's knobs. Hosted plugins show their parameter panel instead.

use egui::{Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_dsp::character::GRAPHIC_BANDS;
use musiccraft_dsp::modulation::{SYNC_RATE_NAMES, autopan_shape, gate_step_on};
use musiccraft_dsp::{SvfMode, response};
use musiccraft_project::EffectSlot;
use musiccraft_project::params::presets::{self, Preset};
use musiccraft_project::params::{EffectKind, ParamSpec, index_of};

use crate::hint::Hint;
use crate::{Action, Frame, UiState, theme, widgets};

/// Beats shown on the multi-tap delay's graph.
const TAP_SPAN: f32 = 4.0;

/// Top and bottom (dB) of a frequency graph.
fn db_range(kind: EffectKind) -> (f32, f32) {
    match kind {
        EffectKind::GraphicEq | EffectKind::ParametricEq => (18.0, -18.0),
        EffectKind::BassBoost => (24.0, -12.0),
        _ => (24.0, -48.0),
    }
}

/// A multi-tap delay level for a pointer height.
fn tap_level(rect: Rect, y: f32) -> f32 {
    ((rect.bottom() - 6.0 - y) / (rect.height() - 12.0)).clamp(0.0, 1.0)
}

/// Size of the graph area.
const GRAPH: egui::Vec2 = vec2(380.0, 150.0);
/// Lowest and highest frequency on frequency graphs.
const F_LOW: f32 = 20.0;
const F_HIGH: f32 = 20_000.0;

/// A point on a graph being dragged: which window and which handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Drag {
    track: usize,
    slot: usize,
    handle: usize,
}

/// Opens (or closes) the window of effect `slot` on `track`.
pub(crate) fn toggle(state: &mut UiState, track: usize, slot: usize) {
    if let Some(i) = state
        .effect_windows
        .iter()
        .position(|w| *w == (track, slot))
    {
        state.effect_windows.remove(i);
    } else {
        state.effect_windows.push((track, slot));
    }
}

fn set_param(track: usize, slot: usize, name: &str, value: f32) -> Invocation {
    Invocation::new(
        "mixer.set_effect_param",
        [
            track.to_string(),
            (slot + 1).to_string(),
            name.to_owned(),
            format!("{value}"),
        ],
    )
}

/// One undo step setting several parameters.
fn batch(invocations: Vec<Invocation>) -> Option<Invocation> {
    match invocations.len() {
        0 => None,
        1 => invocations.into_iter().next(),
        _ => Some(Invocation::new(
            BATCH,
            invocations.iter().map(ToString::to_string),
        )),
    }
}

/// The batch that applies `preset` to an effect, or `None` if nothing changes.
fn preset_invocation(
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    preset: &Preset,
) -> Option<Invocation> {
    let specs = effect.kind.params();
    let changes = preset
        .resolve(specs)
        .into_iter()
        .filter(|(name, value)| {
            index_of(specs, name)
                .and_then(|i| effect.values.get(i))
                .is_some_and(|current| (current - value).abs() > 1e-6)
        })
        .map(|(name, value)| set_param(track, slot, name, value))
        .collect();
    batch(changes)
}

pub(crate) fn show_all(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let project = frame.project;
    // Windows whose slot is gone close.
    state.effect_windows.retain(|(t, s)| {
        project
            .mixer
            .get(*t)
            .is_some_and(|track| *s < track.effects.len())
    });
    let open = state.effect_windows.clone();
    for (index, (track, slot)) in open.into_iter().enumerate() {
        let Some(effect) = project.mixer.get(track).and_then(|t| t.effects.get(slot)) else {
            continue;
        };
        let track_name = project.mixer.get(track).map_or("", |t| t.name.as_str());
        let title = format!("{track_name} › {}", effect_title(effect));
        let mut keep = true;
        egui::Window::new(title)
            .id(egui::Id::new(("effect-window", track, slot)))
            .open(&mut keep)
            .resizable(false)
            .default_pos(
                state.workspace.left_top()
                    + vec2(60.0 + 24.0 * index as f32, 60.0 + 24.0 * index as f32),
            )
            .constrain_to(state.workspace)
            .show(ctx, |ui| {
                contents(ui, state, frame, track, slot, effect, actions)
            });
        if !keep {
            toggle(state, track, slot);
        }
    }
}

fn effect_title(effect: &EffectSlot) -> String {
    match &effect.plugin {
        Some(plugin) => plugin.name.clone(),
        None => crate::browser::effect_title(effect.kind).to_owned(),
    }
}

fn contents(
    ui: &mut egui::Ui,
    state: &mut UiState,
    frame: &Frame<'_>,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    if let Some(plugin) = &effect.plugin {
        let target = crate::plugins::Target::Effect(track, slot + 1);
        crate::plugins::panel(ui, state, frame.plugins, plugin, target, actions);
        return;
    }
    preset_bar(ui, track, slot, effect, actions);
    ui.add_space(4.0);
    let sample_rate = match frame.settings_info.running_rate {
        0 => 48_000.0,
        rate => rate as f32,
    };
    if effect.kind == EffectKind::Convolution {
        impulse_picker(ui, state, track, slot, effect, actions);
    }
    if matches!(effect.kind, EffectKind::PitchCorrect | EffectKind::Tuner) {
        let detected = frame
            .engine
            .pitches
            .iter()
            .find(|(t, s, _)| *t == track && *s == slot)
            .map(|(_, _, hz)| *hz);
        crate::tuner::show(ui, effect, detected, GRAPH);
    } else {
        graph(ui, state, track, slot, effect, sample_rate, actions);
    }
    ui.add_space(4.0);
    knobs(ui, state, track, slot, effect, actions);
}

/// The convolution reverb's impulse response: generated rooms or a file.
fn impulse_picker(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    let current = effect.file.clone().unwrap_or_else(|| "ir:hall".to_owned());
    let label = current.strip_prefix("ir:").map_or_else(
        || {
            std::path::Path::new(&current)
                .file_name()
                .map_or_else(|| current.clone(), |n| n.to_string_lossy().into_owned())
        },
        pretty,
    );
    ui.horizontal(|ui| {
        ui.label("Impulse");
        egui::ComboBox::from_id_salt(("impulse", track, slot))
            .selected_text(label)
            .show_ui(ui, |ui| {
                for name in musiccraft_project::params::IMPULSE_RESPONSES {
                    let key = format!("ir:{name}");
                    if ui.selectable_label(current == key, pretty(name)).clicked() {
                        actions.push(Action::Run(Invocation::new(
                            "mixer.set_effect_file",
                            [track.to_string(), (slot + 1).to_string(), key],
                        )));
                    }
                }
            })
            .response
            .hint("Generated rooms, from small to huge");
        if ui
            .button("Load file…")
            .hint("Use an audio file (a recorded impulse response) as the room")
            .clicked()
        {
            let start = effect.file.clone().filter(|f| !f.starts_with("ir:"));
            state.open_dialog(crate::dialogs::Dialog::set_effect_file(
                track,
                slot,
                start.unwrap_or_default(),
            ));
        }
    });
}

fn preset_bar(
    ui: &mut egui::Ui,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    let list = presets::effect(effect.kind);
    let specs = effect.kind.params();
    let current = list.iter().position(|p| p.matches(specs, &effect.values));
    ui.horizontal(|ui| {
        let on = effect.enabled;
        if widgets::dot(ui, on, theme::ACCENT)
            .hint(if on {
                "On — click to bypass"
            } else {
                "Bypassed — click to turn on"
            })
            .clicked()
        {
            actions.push(Action::Run(Invocation::new(
                "mixer.toggle_effect",
                [track.to_string(), (slot + 1).to_string()],
            )));
        }
        let step = |delta: isize| {
            let n = list.len() as isize;
            let from = current.map_or(if delta > 0 { -1 } else { 0 }, |c| c as isize);
            ((from + delta).rem_euclid(n.max(1))) as usize
        };
        let mut chosen = None;
        if ui.small_button("◀").hint("Previous preset").clicked() {
            chosen = Some(step(-1));
        }
        let name = current.map_or("(custom)", |i| list[i].name);
        egui::ComboBox::from_id_salt(("effect-preset", track, slot))
            .width(170.0)
            .selected_text(name)
            .show_ui(ui, |ui| {
                for (i, preset) in list.iter().enumerate() {
                    if ui
                        .selectable_label(current == Some(i), preset.name)
                        .clicked()
                    {
                        chosen = Some(i);
                    }
                }
            });
        if ui.small_button("▶").hint("Next preset").clicked() {
            chosen = Some(step(1));
        }
        if let Some(invocation) = chosen
            .and_then(|i| list.get(i))
            .and_then(|preset| preset_invocation(track, slot, effect, preset))
        {
            actions.push(Action::Run(invocation));
        }
    });
}

fn knobs(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    actions: &mut Vec<Action>,
) {
    ui.horizontal_wrapped(|ui| {
        for (p, spec) in effect.kind.params().iter().enumerate() {
            let value = effect.values[p];
            if spec.name == "steps" {
                // Edited by clicking the steps in the graph.
                continue;
            }
            let synced = matches!(
                effect.kind,
                EffectKind::Gate | EffectKind::Autopan | EffectKind::AutoFilter
            );
            let synced = synced
                || (effect.kind == EffectKind::Stutter && matches!(spec.name, "slice" | "every"));
            let pick = |names: &[&str]| {
                names
                    .get(value.round() as usize)
                    .copied()
                    .unwrap_or("?")
                    .to_owned()
            };
            let text = match spec.name {
                "sidechain" if value < 0.5 => "off".to_owned(),
                "sidechain" => format!("from {value:.0}"),
                "rate" | "slice" | "every" if synced => SYNC_RATE_NAMES
                    .get(value.round() as usize)
                    .copied()
                    .unwrap_or("?")
                    .to_owned(),
                "shape" if effect.kind == EffectKind::Waveshaper => {
                    pick(&["tanh", "hard clip", "fold", "sine", "asymmetric", "rectify"])
                }
                "shape" => pick(&["sine", "triangle", "square"]),
                "mode" if effect.kind == EffectKind::Distortion => pick(&["soft", "hard"]),
                "mode" => pick(&musiccraft_project::params::FILTER_TYPES),
                "key" => pick(&musiccraft_project::params::NOTE_NAMES),
                "scale" => {
                    let names: Vec<&str> = musiccraft_project::params::CORRECTION_SCALES
                        .iter()
                        .map(|(name, _)| *name)
                        .collect();
                    pick(&names)
                }
                "vowel" => {
                    let i = (value.floor() as usize).min(4);
                    let next = (i + 1).min(4);
                    let letters = ["a", "e", "i", "o", "u"];
                    if value.fract() < 0.05 || i == next {
                        letters[i].to_owned()
                    } else {
                        format!("{}→{}", letters[i], letters[next])
                    }
                }
                "invert_left" | "invert_right" => {
                    if value >= 0.5 { "on" } else { "off" }.to_owned()
                }
                _ => crate::channel_settings::format_value(spec, value),
            };
            let label = pretty(spec.name);
            let out = widgets::labeled_knob(
                ui,
                &label,
                crate::channel_settings::to_unit(spec, value),
                crate::channel_settings::to_unit(spec, spec.default),
                spec.min < 0.0 && spec.max > 0.0,
                &text,
            );
            crate::automation::menu(
                &out.response,
                &format!("mixer:{track}:{}:{}", slot + 1, spec.name),
                actions,
            );
            if out.started {
                state.begin_gesture();
            }
            if let Some(t) = out.changed {
                let new = crate::channel_settings::from_unit(spec, t);
                if new != value {
                    actions.push(Action::Coalesced(
                        set_param(track, slot, spec.name, new),
                        state.gesture(),
                    ));
                }
            }
        }
    });
}

/// The stereo shaper's matrix: a line from each input to each output whose
/// thickness is the gain (red when inverted), and the delayed side marked.
fn stereo_shaper_graph(painter: &egui::Painter, rect: Rect, color: Color32, effect: &EffectSlot) {
    let inputs = [
        pos2(rect.left() + rect.width() * 0.25, rect.top() + 18.0),
        pos2(rect.left() + rect.width() * 0.75, rect.top() + 18.0),
    ];
    let outputs = [
        pos2(inputs[0].x, rect.bottom() - 18.0),
        pos2(inputs[1].x, rect.bottom() - 18.0),
    ];
    let routes = [
        (0, 0, "left_to_left"),
        (1, 0, "right_to_left"),
        (0, 1, "left_to_right"),
        (1, 1, "right_to_right"),
    ];
    for (from, to, name) in routes {
        let gain = value(effect, name);
        if gain.abs() < 0.01 {
            continue;
        }
        let tint = if gain < 0.0 { theme::DANGER } else { color };
        painter.line_segment(
            [inputs[from], outputs[to]],
            Stroke::new(1.0 + gain.abs() * 4.0, tint),
        );
    }
    let delay = value(effect, "delay");
    for (i, label) in ["L", "R"].into_iter().enumerate() {
        let font = egui::FontId::proportional(11.0);
        painter.text(
            inputs[i],
            egui::Align2::CENTER_BOTTOM,
            label,
            font.clone(),
            theme::TEXT,
        );
        let late = (i == 1 && delay > 0.0) || (i == 0 && delay < 0.0);
        let text = if late {
            format!("{label} +{:.1} ms", delay.abs())
        } else {
            label.to_owned()
        };
        painter.text(
            outputs[i],
            egui::Align2::CENTER_TOP,
            text,
            font,
            theme::TEXT,
        );
    }
}

/// Short label for a pan position: `L`, `C`, `R`, or a percentage.
fn pan_label(pan: f32) -> String {
    match pan {
        p if p <= -0.99 => "L".to_owned(),
        p if p >= 0.99 => "R".to_owned(),
        p if p.abs() < 0.01 => "C".to_owned(),
        p if p < 0.0 => format!("{:.0}L", -p * 100.0),
        p => format!("{:.0}R", p * 100.0),
    }
}

/// A parameter name for display: `low_gain` → `Low gain`.
pub(crate) fn pretty(name: &str) -> String {
    let mut text = name.replace('_', " ");
    if let Some(first) = text.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    text
}

// --- graphs ---------------------------------------------------------------

fn freq_x(rect: Rect, f: f32) -> f32 {
    rect.left() + rect.width() * (f.max(F_LOW) / F_LOW).ln() / (F_HIGH / F_LOW).ln()
}

fn x_freq(rect: Rect, x: f32) -> f32 {
    let t = ((x - rect.left()) / rect.width()).clamp(0.0, 1.0);
    F_LOW * (F_HIGH / F_LOW).powf(t)
}

fn db_y(rect: Rect, db: f32, top: f32, bottom: f32) -> f32 {
    let t = ((top - db) / (top - bottom)).clamp(0.0, 1.0);
    rect.top() + rect.height() * t
}

fn y_db(rect: Rect, y: f32, top: f32, bottom: f32) -> f32 {
    let t = ((y - rect.top()) / rect.height()).clamp(0.0, 1.0);
    top - t * (top - bottom)
}

fn value(effect: &EffectSlot, name: &str) -> f32 {
    let specs = effect.kind.params();
    index_of(specs, name)
        .and_then(|i| effect.values.get(i).copied())
        .unwrap_or(0.0)
}

fn spec(effect: &EffectSlot, name: &str) -> Option<ParamSpec> {
    let specs = effect.kind.params();
    index_of(specs, name).and_then(|i| specs.get(i).copied())
}

fn frequency_grid(painter: &egui::Painter, rect: Rect) {
    for f in [
        50.0, 100.0, 200.0, 500.0, 1_000.0, 2_000.0, 5_000.0, 10_000.0,
    ] {
        let x = freq_x(rect, f);
        painter.line_segment(
            [pos2(x, rect.top()), pos2(x, rect.bottom())],
            Stroke::new(1.0, theme::GRID_LINE),
        );
        let label = if f >= 1_000.0 {
            format!("{}k", f / 1_000.0)
        } else {
            format!("{f}")
        };
        painter.text(
            pos2(x + 2.0, rect.bottom() - 2.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(9.0),
            theme::TEXT_DIM,
        );
    }
}

fn curve(painter: &egui::Painter, rect: Rect, color: Color32, points: Vec<Pos2>) {
    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
    }
    // A soft area under the curve.
    for pair in points.windows(2) {
        let quad = vec![
            pair[0],
            pair[1],
            pos2(pair[1].x, rect.bottom()),
            pos2(pair[0].x, rect.bottom()),
        ];
        painter.add(egui::Shape::convex_polygon(
            quad,
            color.gamma_multiply(0.12),
            Stroke::NONE,
        ));
    }
}

fn handle(painter: &egui::Painter, at: Pos2, active: bool, color: Color32) {
    painter.circle_filled(at, if active { 7.0 } else { 5.5 }, color);
    painter.circle_stroke(at, 7.5, Stroke::new(1.0, theme::BACKGROUND));
}

/// Draws the graph for `effect` and turns drags into parameter changes.
fn graph(
    ui: &mut egui::Ui,
    state: &mut UiState,
    track: usize,
    slot: usize,
    effect: &EffectSlot,
    sample_rate: f32,
    actions: &mut Vec<Action>,
) {
    let (rect, response) = ui.allocate_exact_size(GRAPH, Sense::click_and_drag());
    state.effect_graph = Some(rect);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::LCD_BG);
    let color = if effect.enabled {
        theme::ACCENT
    } else {
        theme::TEXT_DIM
    };
    let width = rect.width() as usize;
    // Handles: (position, parameters it moves along x and y).
    let mut handles: Vec<(Pos2, Option<&str>, Option<&str>)> = Vec::new();
    let mut hint = "";
    match effect.kind {
        EffectKind::Filter | EffectKind::Eq => {
            frequency_grid(&painter, rect);
            let (top, bottom) = if effect.kind == EffectKind::Eq {
                (18.0, -18.0)
            } else {
                (24.0, -48.0)
            };
            let zero = db_y(rect, 0.0, top, bottom);
            painter.line_segment(
                [pos2(rect.left(), zero), pos2(rect.right(), zero)],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
            let db_at = |f: f32| -> f32 {
                if effect.kind == EffectKind::Eq {
                    response::eq_db(&effect.values, f, sample_rate)
                } else {
                    let mode = SvfMode::from_param(value(effect, "mode"));
                    let mix = value(effect, "mix");
                    let wet = musiccraft_dsp::db_to_gain(response::svf_db(
                        mode,
                        value(effect, "cutoff"),
                        value(effect, "resonance"),
                        f,
                        sample_rate,
                    ));
                    // Dry and wet are mixed in phase for display.
                    musiccraft_dsp::gain_to_db(1.0 - mix + wet * mix)
                }
            };
            let points = (0..=width)
                .map(|i| {
                    let x = rect.left() + i as f32;
                    let f = x_freq(rect, x);
                    pos2(x, db_y(rect, db_at(f), top, bottom))
                })
                .collect();
            curve(&painter, rect, color, points);
            if effect.kind == EffectKind::Eq {
                for (freq, gain) in [
                    ("low_freq", "low_gain"),
                    ("mid_freq", "mid_gain"),
                    ("high_freq", "high_gain"),
                ] {
                    let at = pos2(
                        freq_x(rect, value(effect, freq)),
                        db_y(rect, value(effect, gain), top, bottom),
                    );
                    handles.push((at, Some(freq), Some(gain)));
                }
                hint = "Drag a point to set a band's frequency and gain; scroll on the middle point for its width";
            } else {
                let res = value(effect, "resonance");
                let at = pos2(
                    freq_x(rect, value(effect, "cutoff")),
                    rect.bottom() - rect.height() * (0.15 + 0.7 * res),
                );
                handles.push((at, Some("cutoff"), Some("resonance")));
                hint = "Drag the point: left/right sets the cutoff, up/down the resonance";
            }
        }
        EffectKind::Compressor
        | EffectKind::Limiter
        | EffectKind::NoiseGate
        | EffectKind::Dynamics => {
            let (top, bottom) = (6.0, -60.0);
            let map = |db_in: f32, db_out: f32| {
                pos2(
                    rect.left() + rect.width() * (db_in - bottom) / (top - bottom),
                    db_y(rect, db_out, top, bottom),
                )
            };
            for db in [-48.0, -36.0, -24.0, -12.0, 0.0] {
                let a = map(db, bottom);
                painter.line_segment(
                    [pos2(a.x, rect.top()), pos2(a.x, rect.bottom())],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
                let y = db_y(rect, db, top, bottom);
                painter.line_segment(
                    [pos2(rect.left(), y), pos2(rect.right(), y)],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
            }
            painter.line_segment(
                [map(bottom, bottom), map(top, top)],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
            let transfer = |db: f32| {
                if effect.kind == EffectKind::Compressor {
                    response::compressor_db(&effect.values, db)
                } else if effect.kind == EffectKind::NoiseGate {
                    response::noise_gate_db(&effect.values, db).max(bottom)
                } else if effect.kind == EffectKind::Dynamics {
                    response::dynamics_db(&effect.values, db).max(bottom)
                } else {
                    response::limiter_db(&effect.values, db)
                }
            };
            let points = (0..=width)
                .map(|i| {
                    let db_in = bottom + (top - bottom) * i as f32 / width as f32;
                    map(db_in, transfer(db_in))
                })
                .collect();
            curve(&painter, rect, color, points);
            if effect.kind == EffectKind::NoiseGate {
                let threshold = value(effect, "threshold");
                handles.push((map(threshold, threshold), Some("threshold"), None));
                hint = "Input level across, output level up: below the threshold the gate closes";
            } else if effect.kind == EffectKind::Compressor {
                let threshold = value(effect, "threshold");
                handles.push((map(threshold, transfer(threshold)), Some("threshold"), None));
                hint = "Input level across, output level up. Drag the knee to set the threshold";
            } else if effect.kind == EffectKind::Dynamics {
                let threshold = value(effect, "comp_threshold");
                handles.push((
                    map(threshold, transfer(threshold)),
                    Some("comp_threshold"),
                    None,
                ));
                let ceiling = value(effect, "limit_ceiling");
                handles.push((
                    pos2(rect.right() - 12.0, db_y(rect, ceiling, top, bottom)),
                    None,
                    Some("limit_ceiling"),
                ));
                hint = "Gate, compressor and limiter combined. Drag the knee for the compressor threshold, the right point for the ceiling";
            } else {
                let ceiling = value(effect, "ceiling");
                handles.push((
                    pos2(rect.right() - 12.0, db_y(rect, ceiling, top, bottom)),
                    None,
                    Some("ceiling"),
                ));
                hint = "Input level across, output level up. Drag the point to set the ceiling";
            }
        }
        EffectKind::Distortion
        | EffectKind::Overdrive
        | EffectKind::SoftClipper
        | EffectKind::Waveshaper => {
            let mid = rect.center();
            painter.line_segment(
                [pos2(rect.left(), mid.y), pos2(rect.right(), mid.y)],
                Stroke::new(1.0, theme::GRID_LINE),
            );
            painter.line_segment(
                [pos2(mid.x, rect.top()), pos2(mid.x, rect.bottom())],
                Stroke::new(1.0, theme::GRID_LINE),
            );
            let points: Vec<Pos2> = (0..=width)
                .map(|i| {
                    let x = i as f32 / width as f32 * 2.0 - 1.0;
                    let y = response::shaper(effect.kind, &effect.values, x)
                        .unwrap_or(x)
                        .clamp(-1.2, 1.2);
                    pos2(rect.left() + i as f32, mid.y - y * rect.height() * 0.4)
                })
                .collect();
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
            }
            hint = match effect.kind {
                EffectKind::SoftClipper => {
                    "Input across, output up: straight below the threshold, bending smoothly into it"
                }
                EffectKind::Waveshaper => "Input across, output up: the shape picks the curve",
                EffectKind::Overdrive => {
                    "Input across, output up: bias makes the curve lopsided (before the tone filter)"
                }
                _ => "Input across, output up: more drive bends the curve into clipping",
            };
        }
        EffectKind::Delay => {
            let time = value(effect, "time");
            let feedback = value(effect, "feedback");
            let ping_pong = value(effect, "ping_pong") >= 0.5;
            let span = (time * 8.0).max(1.0);
            for beat in 0..=(span.ceil() as i32) {
                let x = rect.left() + rect.width() * beat as f32 / span;
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
            }
            let mid = rect.center().y;
            painter.line_segment(
                [pos2(rect.left(), mid), pos2(rect.right(), mid)],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
            painter.line_segment(
                [
                    pos2(rect.left() + 3.0, mid),
                    pos2(rect.left() + 3.0, rect.top() + 6.0),
                ],
                Stroke::new(3.0, theme::TEXT),
            );
            // Echo heights relative to the first; mix is on its knob.
            let mut level = 1.0;
            for n in 1..=8 {
                let x = rect.left() + rect.width() * (n as f32 * time) / span;
                if x > rect.right() || level < 0.01 {
                    break;
                }
                let up = !ping_pong || n % 2 == 1;
                let h = level * (rect.height() * 0.45);
                let end = if up { mid - h } else { mid + h };
                painter.line_segment([pos2(x, mid), pos2(x, end)], Stroke::new(3.0, color));
                level *= feedback;
            }
            hint = "Echoes over time (grid lines are beats); left and right alternate in ping-pong";
        }
        EffectKind::Gate => {
            let duty = value(effect, "duty");
            let depth = value(effect, "depth");
            let steps = value(effect, "steps");
            let cell = rect.width() / 16.0;
            for i in 0..16 {
                let x = rect.left() + cell * i as f32;
                let r = Rect::from_min_max(
                    pos2(x + 1.0, rect.top() + 6.0),
                    pos2(x + cell - 1.0, rect.bottom() - 6.0),
                );
                let beat = if i % 4 == 0 {
                    theme::GRID_BAR
                } else {
                    theme::GRID_LINE
                };
                painter.rect_stroke(r, 2.0, Stroke::new(1.0, beat), egui::StrokeKind::Inside);
                if gate_step_on(steps, i) {
                    let open =
                        Rect::from_min_max(r.min, pos2(r.left() + r.width() * duty, r.bottom()));
                    painter.rect_filled(open, 2.0, color.gamma_multiply(0.8));
                } else if depth < 1.0 {
                    let floor = r.bottom() - r.height() * (1.0 - depth);
                    painter.rect_filled(
                        Rect::from_min_max(pos2(r.left(), floor), r.max),
                        2.0,
                        color.gamma_multiply(0.2),
                    );
                }
            }
            hint = "Click a step to turn it on or off; duty sets how much of each step is open";
        }
        EffectKind::Autopan => {
            let shape = value(effect, "shape");
            let depth = value(effect, "depth");
            let mid = rect.center().y;
            painter.line_segment(
                [pos2(rect.left(), mid), pos2(rect.right(), mid)],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
            painter.text(
                pos2(rect.left() + 4.0, rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                "R",
                egui::FontId::proportional(10.0),
                theme::TEXT_DIM,
            );
            painter.text(
                pos2(rect.left() + 4.0, rect.bottom() - 4.0),
                egui::Align2::LEFT_BOTTOM,
                "L",
                egui::FontId::proportional(10.0),
                theme::TEXT_DIM,
            );
            let points: Vec<Pos2> = (0..=width)
                .map(|i| {
                    let phase = i as f64 / width as f64 * 2.0;
                    let pan = autopan_shape(shape, phase) * depth;
                    pos2(rect.left() + i as f32, mid - pan * rect.height() * 0.42)
                })
                .collect();
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
            }
            hint = "Where the sound sits over two cycles: up is right, down is left";
        }
        EffectKind::Bitcrusher => {
            let bits = value(effect, "bits");
            let hold = value(effect, "downsample").max(1.0);
            let levels = 2.0_f32.powf(bits - 1.0);
            let mid = rect.center().y;
            let wave = |x: f32| (x / rect.width() * std::f32::consts::TAU * 2.0).sin();
            let smooth: Vec<Pos2> = (0..=width)
                .map(|i| {
                    pos2(
                        rect.left() + i as f32,
                        mid - wave(i as f32) * rect.height() * 0.4,
                    )
                })
                .collect();
            for pair in smooth.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(1.0, theme::TEXT_DIM));
            }
            let px_per_sample = 1.5;
            let crushed: Vec<Pos2> = (0..=width)
                .map(|i| {
                    let held = ((i as f32 / px_per_sample / hold).floor() * hold * px_per_sample)
                        .min(rect.width());
                    let y = (wave(held) * levels).round() / levels;
                    pos2(rect.left() + i as f32, mid - y * rect.height() * 0.4)
                })
                .collect();
            for pair in crushed.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
            }
            hint = "A sine wave before (grey) and after crushing: fewer bits mean bigger steps";
        }
        EffectKind::Stereo => {
            let width_v = value(effect, "width");
            let balance = value(effect, "balance");
            let center = rect.center();
            let h = rect.height() * 0.42;
            for (side, invert) in [(-1.0_f32, "invert_left"), (1.0, "invert_right")] {
                let level = (1.0 + side * balance).min(1.0);
                let spread = (width_v / 2.0).min(1.0) * 0.8 + 0.1;
                let end = pos2(center.x + side * h * spread * 1.6, center.y - h * level);
                let stroke = if value(effect, invert) >= 0.5 {
                    Stroke::new(2.0, theme::DANGER)
                } else {
                    Stroke::new(2.0, color)
                };
                painter.line_segment([pos2(center.x, rect.bottom() - 6.0), end], stroke);
                painter.text(
                    end,
                    egui::Align2::CENTER_BOTTOM,
                    if side < 0.0 { "L" } else { "R" },
                    egui::FontId::proportional(11.0),
                    theme::TEXT,
                );
            }
            hint = "Lines show how far apart left and right sit; red means phase inverted";
        }
        EffectKind::StereoShaper => {
            stereo_shaper_graph(&painter, rect, color, effect);
            hint = "Where each output side's sound comes from (red is phase inverted); the delay shifts one side later";
        }
        EffectKind::Reverb | EffectKind::Chorus | EffectKind::Phaser | EffectKind::Flanger => {
            let points: Vec<Pos2> = if effect.kind == EffectKind::Reverb {
                let size = value(effect, "size");
                let damping = value(effect, "damping");
                let mix = value(effect, "mix");
                let decay = 0.4 + size * 4.0;
                (0..=width)
                    .map(|i| {
                        let t = i as f32 / width as f32 * 5.0;
                        let level = mix.max(0.05) * (-t / decay * (1.0 + damping)).exp();
                        pos2(
                            rect.left() + i as f32,
                            rect.bottom() - level * rect.height() * 0.9,
                        )
                    })
                    .collect()
            } else {
                let rate = value(effect, "rate");
                let depth = value(effect, "depth")
                    / spec(effect, "depth").map_or(10.0, |s| s.max.max(1e-3))
                    * 0.9;
                (0..=width)
                    .map(|i| {
                        let t = i as f32 / width as f32 * 2.0;
                        let y = (t * rate * std::f32::consts::TAU).sin() * depth;
                        pos2(
                            rect.left() + i as f32,
                            rect.center().y - y * rect.height() * 0.4,
                        )
                    })
                    .collect()
            };
            curve(&painter, rect, color, points);
            hint = if effect.kind == EffectKind::Reverb {
                "The tail over five seconds: size lengthens it, damping darkens and shortens it"
            } else {
                "The sweep over two seconds: rate is speed, depth is height"
            };
        }
        EffectKind::GraphicEq
        | EffectKind::ParametricEq
        | EffectKind::BassBoost
        | EffectKind::DcBlock
        | EffectKind::Formant
        | EffectKind::AutoFilter => {
            frequency_grid(&painter, rect);
            let (top, bottom) = db_range(effect.kind);
            let zero = db_y(rect, 0.0, top, bottom);
            painter.line_segment(
                [pos2(rect.left(), zero), pos2(rect.right(), zero)],
                Stroke::new(1.0, theme::GRID_BEAT),
            );
            let points = (0..=width)
                .map(|i| {
                    let x = rect.left() + i as f32;
                    let db = response::effect_db(
                        effect.kind,
                        &effect.values,
                        x_freq(rect, x),
                        sample_rate,
                    )
                    .unwrap_or(0.0);
                    pos2(x, db_y(rect, db, top, bottom))
                })
                .collect();
            curve(&painter, rect, color, points);
            let names = effect.kind.params();
            let name = |i: usize| names.get(i).map(|s| s.name);
            match effect.kind {
                EffectKind::GraphicEq => {
                    for (i, f) in GRAPHIC_BANDS.iter().enumerate() {
                        if let Some(n) = name(i) {
                            let at =
                                pos2(freq_x(rect, *f), db_y(rect, value(effect, n), top, bottom));
                            handles.push((at, None, Some(n)));
                        }
                    }
                    hint = "Drag a band's point up or down to boost or cut it";
                }
                EffectKind::ParametricEq => {
                    for band in 0..7 {
                        if let (Some(f), Some(g)) = (name(band * 3), name(band * 3 + 1)) {
                            let at = pos2(
                                freq_x(rect, value(effect, f)),
                                db_y(rect, value(effect, g), top, bottom),
                            );
                            handles.push((at, Some(f), Some(g)));
                        }
                    }
                    hint = "Drag a point to set a band's frequency and gain; band 1 is a low shelf, band 7 a high shelf";
                }
                EffectKind::BassBoost => {
                    let at = pos2(
                        freq_x(rect, value(effect, "freq")),
                        db_y(rect, value(effect, "boost") * 0.5, top, bottom),
                    );
                    handles.push((at, Some("freq"), Some("boost")));
                    hint = "Drag the point: left/right sets the frequency, up/down the boost";
                }
                EffectKind::DcBlock => {
                    let at = pos2(
                        freq_x(rect, value(effect, "freq")),
                        db_y(rect, -3.0, top, bottom),
                    );
                    handles.push((at, Some("freq"), None));
                    hint = "Everything below the point is removed; drag it left or right";
                }
                EffectKind::AutoFilter => {
                    let res = value(effect, "resonance");
                    let at = pos2(
                        freq_x(rect, value(effect, "cutoff")),
                        rect.bottom() - rect.height() * (0.15 + 0.7 * res),
                    );
                    handles.push((at, Some("cutoff"), Some("resonance")));
                    hint = "The filter at rest; the LFO and the input level sweep it from here. Drag to set cutoff and resonance";
                }
                _ => hint = "The vowel's three formant peaks; the vowel knob morphs a-e-i-o-u",
            }
        }
        EffectKind::MultiTap => {
            for beat in 0..=(TAP_SPAN as i32) {
                let x = rect.left() + rect.width() * beat as f32 / TAP_SPAN;
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
            }
            painter.line_segment(
                [
                    pos2(rect.left() + 3.0, rect.bottom() - 6.0),
                    pos2(rect.left() + 3.0, rect.top() + 6.0),
                ],
                Stroke::new(3.0, theme::TEXT),
            );
            for tap in 1..=4 {
                let (time, level) = (format!("tap{tap}_time"), format!("tap{tap}_level"));
                let x = rect.left() + rect.width() * value(effect, &time) / TAP_SPAN;
                let y = rect.bottom() - 6.0 - (rect.height() - 12.0) * value(effect, &level);
                painter.line_segment(
                    [pos2(x, rect.bottom() - 6.0), pos2(x, y)],
                    Stroke::new(3.0, color),
                );
                painter.text(
                    pos2(x + 4.0, y),
                    egui::Align2::LEFT_TOP,
                    pan_label(value(effect, &format!("tap{tap}_pan"))),
                    egui::FontId::proportional(10.0),
                    theme::TEXT_DIM,
                );
                let names = effect.kind.params();
                let t = names.get((tap - 1) * 2).map(|s| s.name);
                let l = names.get((tap - 1) * 2 + 1).map(|s| s.name);
                handles.push((pos2(x, y), t, l));
            }
            hint = "Four echoes over four beats: drag a tap sideways for its time, up or down for its level; each tap's pan and filters are on the knobs";
        }
        EffectKind::RingMod | EffectKind::FreqShift => {
            frequency_grid(&painter, rect);
            let input = 440.0;
            let mix = value(effect, "mix");
            let outputs: Vec<f32> = if effect.kind == EffectKind::RingMod {
                let f = value(effect, "freq");
                vec![input + f, (input - f).abs()]
            } else {
                vec![(input + value(effect, "shift")).abs()]
            };
            let bar = |f: f32, height: f32, c: Color32| {
                let x = freq_x(rect, f.clamp(F_LOW, F_HIGH));
                painter.line_segment(
                    [
                        pos2(x, rect.bottom() - 4.0),
                        pos2(x, rect.bottom() - 4.0 - height),
                    ],
                    Stroke::new(3.0, c),
                );
            };
            let h = rect.height() * 0.8;
            bar(input, h * (1.0 - mix).max(0.08), theme::TEXT_DIM);
            for f in outputs {
                bar(f, h * mix.max(0.08), color);
            }
            hint = if effect.kind == EffectKind::RingMod {
                "A 440 Hz tone (grey) becomes the sum and difference of it and the carrier"
            } else {
                "A 440 Hz tone (grey) moves by the shift; harmonics move by the same hertz, so they stop being harmonic"
            };
        }
        EffectKind::Transient => {
            let (before, after) = response::transient_envelopes(&effect.values, width + 1);
            let peak = 1.6;
            let to_points = |v: &[f32]| -> Vec<Pos2> {
                v.iter()
                    .enumerate()
                    .map(|(i, level)| {
                        pos2(
                            rect.left() + i as f32,
                            rect.bottom() - 4.0 - (level / peak).min(1.0) * (rect.height() - 8.0),
                        )
                    })
                    .collect()
            };
            for pair in to_points(&before).windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(1.0, theme::TEXT_DIM));
            }
            for pair in to_points(&after).windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
            }
            hint = "A drum hit's level before (grey) and after: attack shapes the front, sustain the tail";
        }
        EffectKind::Multiband => {
            frequency_grid(&painter, rect);
            let (f1, f2) = (value(effect, "low_freq"), value(effect, "high_freq"));
            let (top, bottom) = db_range(EffectKind::GraphicEq);
            let edges = [F_LOW, f1, f2, F_HIGH];
            for (band, name) in ["low", "mid", "high"].iter().enumerate() {
                let (a, b) = (freq_x(rect, edges[band]), freq_x(rect, edges[band + 1]));
                let threshold = value(effect, &format!("{name}_thresh"));
                let ratio = value(effect, &format!("{name}_ratio"));
                // Darker bands compress harder (at −6 dB input).
                let squeeze = -musiccraft_dsp::creative::band_gain_db(-6.0, threshold, ratio);
                let fill = color.gamma_multiply((0.1 + squeeze / 40.0).clamp(0.1, 0.6));
                painter.rect_filled(
                    Rect::from_min_max(pos2(a, rect.top()), pos2(b, rect.bottom())),
                    0.0,
                    fill,
                );
                let gain = db_y(rect, value(effect, &format!("{name}_gain")), top, bottom);
                painter.line_segment([pos2(a, gain), pos2(b, gain)], Stroke::new(2.0, color));
                painter.text(
                    pos2((a + b) * 0.5, rect.top() + 4.0),
                    egui::Align2::CENTER_TOP,
                    format!("{name} {threshold:.0} dB {ratio:.1}:1"),
                    egui::FontId::proportional(10.0),
                    theme::TEXT,
                );
            }
            let mid = rect.center().y;
            handles.push((pos2(freq_x(rect, f1), mid), Some("low_freq"), None));
            handles.push((pos2(freq_x(rect, f2), mid), Some("high_freq"), None));
            hint = "Three bands with their own compressors; drag the points to move the crossovers. Lines show each band's gain; brighter shading compresses harder";
        }
        EffectKind::MultibandLimiter => {
            frequency_grid(&painter, rect);
            let (f1, f2) = (value(effect, "low_freq"), value(effect, "high_freq"));
            let (top, bottom) = (6.0, -24.0);
            let edges = [F_LOW, f1, f2, F_HIGH];
            let amount = value(effect, "amount");
            for (band, name) in ["low", "mid", "high"].iter().enumerate() {
                let (a, b) = (freq_x(rect, edges[band]), freq_x(rect, edges[band + 1]));
                let drive = value(effect, &format!("{name}_drive")) * amount;
                let fill = color.gamma_multiply((0.1 + drive / 40.0).clamp(0.1, 0.7));
                painter.rect_filled(
                    Rect::from_min_max(pos2(a, rect.top()), pos2(b, rect.bottom())),
                    0.0,
                    fill,
                );
                let ceiling = db_y(rect, value(effect, &format!("{name}_ceiling")), top, bottom);
                painter.line_segment(
                    [pos2(a, ceiling), pos2(b, ceiling)],
                    Stroke::new(2.0, color),
                );
                painter.text(
                    pos2((a + b) * 0.5, rect.top() + 4.0),
                    egui::Align2::CENTER_TOP,
                    format!("{name} +{drive:.0} dB"),
                    egui::FontId::proportional(10.0),
                    theme::TEXT,
                );
            }
            let master = db_y(rect, value(effect, "ceiling"), top, bottom);
            painter.line_segment(
                [pos2(rect.left(), master), pos2(rect.right(), master)],
                Stroke::new(1.0, theme::DANGER),
            );
            let mid = rect.center().y;
            handles.push((pos2(freq_x(rect, f1), mid), Some("low_freq"), None));
            handles.push((pos2(freq_x(rect, f2), mid), Some("high_freq"), None));
            hint = "Three bands, each driven into its own ceiling (lines); brighter shading drives harder. The red line is the master ceiling; drag the points to move the crossovers";
        }
        EffectKind::Vocoder => {
            frequency_grid(&painter, rect);
            let shift = 2.0_f32.powf(value(effect, "shift") / 12.0);
            for f in musiccraft_dsp::creative::vocoder_freqs() {
                let x = freq_x(rect, f);
                let xs = freq_x(rect, (f * shift).min(F_HIGH));
                painter.line_segment(
                    [pos2(x, rect.bottom() - 4.0), pos2(x, rect.center().y)],
                    Stroke::new(2.0, theme::TEXT_DIM),
                );
                painter.line_segment(
                    [pos2(xs, rect.center().y), pos2(xs, rect.top() + 16.0)],
                    Stroke::new(2.0, color),
                );
            }
            let source = value(effect, "sidechain");
            painter.text(
                pos2(rect.left() + 4.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                if source < 0.5 {
                    "No modulator: set Sidechain to the insert with the voice".to_owned()
                } else {
                    format!("Modulator: insert {source:.0}")
                },
                egui::FontId::proportional(11.0),
                theme::TEXT,
            );
            hint = "16 bands: the modulator's levels (grey, from the sidechain) shape the carrier on this insert (colour)";
        }
        EffectKind::PitchShift => {
            frequency_grid(&painter, rect);
            let ratio =
                2.0_f32.powf((value(effect, "pitch") + value(effect, "fine") / 100.0) / 12.0);
            let mix = value(effect, "mix");
            let h = rect.height() * 0.8;
            for (k, f) in [220.0_f32, 440.0, 660.0].iter().enumerate() {
                let level = 1.0 / (k as f32 + 1.0);
                for (freq, height, c) in [
                    (*f, h * level * (1.0 - mix).max(0.08), theme::TEXT_DIM),
                    (*f * ratio, h * level * mix.max(0.08), color),
                ] {
                    let x = freq_x(rect, freq.clamp(F_LOW, F_HIGH));
                    painter.line_segment(
                        [
                            pos2(x, rect.bottom() - 4.0),
                            pos2(x, rect.bottom() - 4.0 - height),
                        ],
                        Stroke::new(3.0, c),
                    );
                }
            }
            hint = "A 220 Hz note and its harmonics (grey) and where they end up (colour)";
        }
        EffectKind::Stutter => {
            let names = musiccraft_dsp::modulation::SYNC_RATES;
            let pick = |name: &str| {
                names
                    .get(value(effect, name).round().max(0.0) as usize)
                    .copied()
                    .unwrap_or(1.0) as f32
            };
            let every = pick("every").max(pick("slice"));
            let slice = pick("slice");
            let portion = value(effect, "portion");
            let start = rect.left() + rect.width() * (1.0 - portion);
            let beats = every.ceil() as i32;
            for beat in 0..=beats {
                let x = rect.left() + rect.width() * beat as f32 / every;
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
            }
            painter.rect_filled(
                Rect::from_min_max(
                    pos2(start, rect.top() + 6.0),
                    pos2(rect.right(), rect.bottom() - 6.0),
                ),
                2.0,
                color.gamma_multiply(0.25),
            );
            let width_slice = rect.width() * slice / every;
            let decay = value(effect, "decay");
            let mut x = start;
            let mut level = 1.0;
            while x < rect.right() - 0.5 && width_slice > 1.0 {
                let end = (x + width_slice).min(rect.right());
                let top = rect.bottom() - 6.0 - (rect.height() - 12.0) * level;
                painter.rect_filled(
                    Rect::from_min_max(pos2(x + 1.0, top), pos2(end - 1.0, rect.bottom() - 6.0)),
                    1.0,
                    color.gamma_multiply(0.8),
                );
                x = end;
                level *= decay;
            }
            hint = "One period (grid lines are beats): the shaded end repeats the slice that starts it";
        }
        EffectKind::Convolution => {
            let file = effect.file.as_deref().unwrap_or("ir:hall");
            let rt60 = match file {
                "ir:room" => 0.6,
                "ir:chamber" => 1.1,
                "ir:plate" => 1.6,
                "ir:cathedral" => 4.5,
                _ => 2.4,
            };
            let span = 5.0;
            let predelay = value(effect, "predelay") / 1_000.0;
            let mix = value(effect, "mix");
            for second in 0..=5 {
                let x = rect.left() + rect.width() * second as f32 / span;
                painter.line_segment(
                    [pos2(x, rect.top()), pos2(x, rect.bottom())],
                    Stroke::new(1.0, theme::GRID_LINE),
                );
            }
            let points = (0..=width)
                .map(|i| {
                    let t = i as f32 / width as f32 * span - predelay;
                    let level = if t < 0.0 {
                        0.0
                    } else {
                        mix.max(0.05) * (-6.9 * t / rt60).exp()
                    };
                    pos2(
                        rect.left() + i as f32,
                        rect.bottom() - level * rect.height() * 0.9,
                    )
                })
                .collect();
            curve(&painter, rect, color, points);
            if !file.starts_with("ir:") {
                painter.text(
                    pos2(rect.right() - 4.0, rect.top() + 2.0),
                    egui::Align2::RIGHT_TOP,
                    "from file",
                    egui::FontId::proportional(10.0),
                    theme::TEXT_DIM,
                );
            }
            hint = "The tail over five seconds (a sketch for files): pick a room above or load a recorded impulse response";
        }
        // Drawn by `crate::tuner` instead.
        EffectKind::Plugin | EffectKind::PitchCorrect | EffectKind::Tuner => {}
    }

    let dragging = state
        .effect_drag
        .filter(|d| d.track == track && d.slot == slot);
    for (i, (at, _, _)) in handles.iter().enumerate() {
        handle(
            &painter,
            *at,
            dragging.is_some_and(|d| d.handle == i),
            theme::HIGHLIGHT,
        );
    }
    let response = response.hint(hint);
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let nearest = handles
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.0.distance(pointer).total_cmp(&b.1.0.distance(pointer)))
                .map(|(i, _)| i);
            if let Some(handle) = nearest {
                state.begin_gesture();
                state.effect_drag = Some(Drag {
                    track,
                    slot,
                    handle,
                });
            }
        }
    }
    if response.drag_stopped() {
        state.effect_drag = None;
    }
    // Clicking a gate step turns it on or off.
    if effect.kind == EffectKind::Gate && response.clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let index = (((pointer.x - rect.left()) / rect.width() * 16.0) as usize).min(15);
            let steps = value(effect, "steps").round() as u32 ^ (1 << index);
            actions.push(Action::Run(set_param(track, slot, "steps", steps as f32)));
        }
    }
    if let (Some(drag), Some(pointer)) = (dragging, response.interact_pointer_pos()) {
        if response.dragged() {
            if let Some((_, x_param, y_param)) = handles.get(drag.handle) {
                let mut changes = Vec::new();
                let mut push = |name: &str, raw: f32| {
                    if let Some(spec) = spec(effect, name) {
                        let new = spec.sanitize(raw);
                        if (new - value(effect, name)).abs() > 1e-6 {
                            changes.push(set_param(track, slot, name, new));
                        }
                    }
                };
                match effect.kind {
                    EffectKind::Eq => {
                        if let Some(name) = x_param {
                            push(name, x_freq(rect, pointer.x));
                        }
                        if let Some(name) = y_param {
                            push(name, y_db(rect, pointer.y, 18.0, -18.0));
                        }
                    }
                    EffectKind::Filter => {
                        push("cutoff", x_freq(rect, pointer.x));
                        let t = (rect.bottom() - pointer.y) / rect.height();
                        push("resonance", ((t - 0.15) / 0.7).clamp(0.0, 1.0));
                    }
                    EffectKind::Compressor | EffectKind::NoiseGate => {
                        let db = -60.0 + 66.0 * (pointer.x - rect.left()) / rect.width();
                        push("threshold", db);
                    }
                    EffectKind::Limiter => push("ceiling", y_db(rect, pointer.y, 6.0, -60.0)),
                    EffectKind::Dynamics => {
                        if let Some(name) = x_param {
                            push(
                                name,
                                -60.0 + 66.0 * (pointer.x - rect.left()) / rect.width(),
                            );
                        }
                        if let Some(name) = y_param {
                            push(name, y_db(rect, pointer.y, 6.0, -60.0));
                        }
                    }
                    EffectKind::MultiTap => {
                        if let Some(name) = x_param {
                            push(name, (pointer.x - rect.left()) / rect.width() * TAP_SPAN);
                        }
                        if let Some(name) = y_param {
                            push(name, tap_level(rect, pointer.y));
                        }
                    }
                    kind => {
                        let (top, bottom) = db_range(kind);
                        if let Some(name) = x_param {
                            push(name, x_freq(rect, pointer.x));
                        }
                        match *y_param {
                            Some("resonance") => {
                                let t = (rect.bottom() - pointer.y) / rect.height();
                                push("resonance", ((t - 0.15) / 0.7).clamp(0.0, 1.0));
                            }
                            Some(name) => push(name, y_db(rect, pointer.y, top, bottom)),
                            None => {}
                        }
                    }
                }
                if let Some(invocation) = batch(changes) {
                    actions.push(Action::Coalesced(invocation, state.gesture()));
                }
            }
        }
    }
    // Scrolling over the middle EQ point changes its width.
    if effect.kind == EffectKind::Eq && response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        let near_mid = handles
            .get(1)
            .zip(response.hover_pos())
            .is_some_and(|((at, _, _), pointer)| at.distance(pointer) < 20.0);
        if scroll != 0.0 && near_mid {
            if let Some(spec) = spec(effect, "mid_q") {
                let q = spec.sanitize(value(effect, "mid_q") * (1.0 + scroll * 0.004));
                actions.push(Action::Coalesced(
                    set_param(track, slot, "mid_q", q),
                    state.gesture(),
                ));
            }
        }
    }
}
