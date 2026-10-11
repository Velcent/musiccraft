//! Windows for mixer effects: a preset bar with bypass, a graph you can drag
//! (filter and EQ curves, dynamics transfer curves, delay taps…) and the
//! effect's knobs. Hosted plugins show their parameter panel instead.

use egui::{Color32, Pos2, Rect, Sense, Stroke, pos2, vec2};
use musiccraft_commands::{BATCH, Invocation};
use musiccraft_dsp::modulation::{SYNC_RATE_NAMES, autopan_shape, gate_step_on};
use musiccraft_dsp::{SvfMode, response};
use musiccraft_project::EffectSlot;
use musiccraft_project::params::presets::{self, Preset};
use musiccraft_project::params::{EffectKind, ParamSpec, index_of};

use crate::hint::Hint;
use crate::{Action, Frame, UiState, theme, widgets};

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
    graph(ui, state, track, slot, effect, sample_rate, actions);
    ui.add_space(4.0);
    knobs(ui, state, track, slot, effect, actions);
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
            let synced = matches!(effect.kind, EffectKind::Gate | EffectKind::Autopan);
            let text = match spec.name {
                "sidechain" if value < 0.5 => "off".to_owned(),
                "sidechain" => format!("from {value:.0}"),
                "rate" if synced => SYNC_RATE_NAMES
                    .get(value.round() as usize)
                    .copied()
                    .unwrap_or("?")
                    .to_owned(),
                "shape" => ["sine", "triangle", "square"]
                    .get(value.round() as usize)
                    .copied()
                    .unwrap_or("?")
                    .to_owned(),
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
        EffectKind::Compressor | EffectKind::Limiter => {
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
            if effect.kind == EffectKind::Compressor {
                let threshold = value(effect, "threshold");
                handles.push((map(threshold, transfer(threshold)), Some("threshold"), None));
                hint = "Input level across, output level up. Drag the knee to set the threshold";
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
        EffectKind::Distortion => {
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
                    let y = response::distortion_shape(&effect.values, x).clamp(-1.2, 1.2);
                    pos2(rect.left() + i as f32, mid.y - y * rect.height() * 0.4)
                })
                .collect();
            for pair in points.windows(2) {
                painter.line_segment([pair[0], pair[1]], Stroke::new(2.0, color));
            }
            hint = "Input across, output up: more drive bends the curve into clipping";
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
        EffectKind::Plugin => {}
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
                    EffectKind::Compressor => {
                        let db = -60.0 + 66.0 * (pointer.x - rect.left()) / rect.width();
                        push("threshold", db);
                    }
                    EffectKind::Limiter => push("ceiling", y_db(rect, pointer.y, 6.0, -60.0)),
                    _ => {}
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
