//! Antiserum's wavetable editor (`docs/spec/antiserum-wavetables.md`): a
//! toolbar, a 3D stack of the frames, and tabs to draw the selected frame,
//! edit its harmonics or fill it from a formula. Like the effect editor it
//! never changes the table: it reports [`WavetableEdit`]s and the caller
//! applies them.

use egui::{Color32, Pos2, Rect, Sense, Stroke, Ui, pos2, vec2};
use musiccraft_formula::Formula;
use musiccraft_wavetables::spectrum::{Partial, partials};
use musiccraft_wavetables::{FRAME_LEN, MorphMode, Op, Table, factory};

use crate::hint::Hint;
use crate::theme;

/// Width of the editor's drawing areas.
const WIDTH: f32 = 420.0;
/// Height of the frame stack.
const STACK_HEIGHT: f32 = 180.0;
/// Height of the draw view and of the harmonics' magnitude bars.
const VIEW_HEIGHT: f32 = 140.0;
/// Height of the harmonics' phase bars.
const PHASE_HEIGHT: f32 = 50.0;
/// Most frames drawn in the stack.
const STACK_LINES: usize = 64;
/// Points per line in the stack.
const STACK_POINTS: usize = 128;
/// Harmonics shown as bars.
const BARS: usize = 64;

/// A change the user made in the editor.
#[derive(Debug, Clone, PartialEq)]
pub enum WavetableEdit {
    /// A gesture began: what follows is one undo step.
    Begin,
    /// A change to the table.
    Op(Op),
    /// Load factory table `name` (follows a [`WavetableEdit::Begin`]).
    Factory(&'static str),
}

/// The editor's tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tab {
    /// Draw the frame's waveform.
    #[default]
    Draw,
    /// Edit partials' magnitude and phase.
    Harmonics,
    /// Fill frames from a formula.
    Formula,
}

/// A toolbar button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Normalize,
    RemoveDc,
    ReverseSamples,
    ReverseFrames,
    InsertFrame,
    DeleteFrame,
    Morph,
}

/// The editor's own view state; the caller keeps it between frames.
#[derive(Debug, Clone, PartialEq)]
pub struct WavetableEditor {
    /// The selected frame.
    pub selected: usize,
    /// The open tab.
    pub tab: Tab,
    /// Toolbar operations and formulas apply to every frame.
    pub all_frames: bool,
    /// The formula being edited.
    pub formula: String,
    /// How the morph button blends.
    pub morph: MorphMode,
    /// The last formula's parse error.
    pub error: Option<String>,
    /// Sample index and value where the current draw stroke last was.
    last_draw: Option<(f32, f32)>,
}

impl Default for WavetableEditor {
    fn default() -> Self {
        Self {
            selected: 0,
            tab: Tab::Draw,
            all_frames: false,
            formula: "sin(x*pi*2) * (1-y)".to_owned(),
            morph: MorphMode::Crossfade,
            error: None,
            last_draw: None,
        }
    }
}

/// Where the editor drew its areas (for tests).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(not(test), allow(dead_code))]
struct Areas {
    stack: Rect,
    view: Rect,
    phase: Rect,
}

impl Default for Areas {
    fn default() -> Self {
        Self {
            stack: Rect::NOTHING,
            view: Rect::NOTHING,
            phase: Rect::NOTHING,
        }
    }
}

impl WavetableEditor {
    /// Draws the editor for `table`, appending what the user changed.
    pub fn show(&mut self, ui: &mut Ui, table: &Table, edits: &mut Vec<WavetableEdit>) {
        self.show_areas(ui, table, edits);
    }

    fn show_areas(&mut self, ui: &mut Ui, table: &Table, edits: &mut Vec<WavetableEdit>) -> Areas {
        self.selected = self.selected.min(table.frames() - 1);
        self.toolbar(ui, table, edits);
        let mut areas = Areas {
            stack: self.stack(ui, table),
            ..Areas::default()
        };
        ui.horizontal(|ui| {
            for (tab, label) in [
                (Tab::Draw, "Draw"),
                (Tab::Harmonics, "Harmonics"),
                (Tab::Formula, "Formula"),
            ] {
                ui.selectable_value(&mut self.tab, tab, label);
            }
        });
        let frame = table.frame(self.selected).unwrap_or(&[]);
        match self.tab {
            Tab::Draw => areas.view = self.draw_view(ui, frame, edits),
            Tab::Harmonics => (areas.view, areas.phase) = self.harmonics(ui, frame, edits),
            Tab::Formula => self.formula_tab(ui, table, edits),
        }
        areas
    }

    /// The frames a toolbar operation or formula applies to.
    fn range(&self, frames: usize) -> (usize, usize) {
        if self.all_frames {
            (0, frames - 1)
        } else {
            (self.selected, self.selected)
        }
    }

    /// The operation a toolbar button makes.
    #[must_use]
    pub fn tool_op(&self, tool: Tool, frames: usize) -> Op {
        let (first, last) = self.range(frames);
        match tool {
            Tool::Normalize => Op::Normalize { first, last },
            Tool::RemoveDc => Op::RemoveDc { first, last },
            Tool::ReverseSamples => Op::ReverseSamples { first, last },
            Tool::ReverseFrames => Op::ReverseFrames,
            Tool::InsertFrame => Op::InsertFrame { at: self.selected },
            Tool::DeleteFrame => Op::DeleteFrame { at: self.selected },
            Tool::Morph => Op::Morph {
                first: 0,
                last: frames - 1,
                mode: self.morph,
            },
        }
    }

    fn toolbar(&mut self, ui: &mut Ui, table: &Table, edits: &mut Vec<WavetableEdit>) {
        let frames = table.frames();
        ui.horizontal_wrapped(|ui| {
            egui::ComboBox::from_id_salt("wavetable-factory")
                .selected_text("Factory tables")
                .show_ui(ui, |ui| {
                    for name in factory::names() {
                        if ui.selectable_label(false, *name).clicked() {
                            edits.push(WavetableEdit::Begin);
                            edits.push(WavetableEdit::Factory(name));
                        }
                    }
                });
            ui.label(format!("Frame {} / {frames}", self.selected + 1));
            ui.checkbox(&mut self.all_frames, "All")
                .hint("Apply the tools and formulas to every frame");
            let tools = [
                (Tool::Normalize, "Normalize", "Scale to a peak of 1"),
                (Tool::RemoveDc, "DC", "Remove DC (centre the waveform)"),
                (Tool::ReverseSamples, "Reverse", "Play the frame backwards"),
                (
                    Tool::ReverseFrames,
                    "Flip",
                    "Reverse the order of the frames",
                ),
                (
                    Tool::InsertFrame,
                    "+",
                    "Insert a copy of the frame after it",
                ),
                (Tool::DeleteFrame, "−", "Delete the frame"),
                (
                    Tool::Morph,
                    "Morph",
                    "Rebuild the frames between the first and the last as a morph",
                ),
            ];
            for (tool, label, hint) in tools {
                if ui.small_button(label).hint(hint).clicked() {
                    edits.push(WavetableEdit::Begin);
                    edits.push(WavetableEdit::Op(self.tool_op(tool, frames)));
                }
            }
            egui::ComboBox::from_id_salt("wavetable-morph")
                .selected_text(match self.morph {
                    MorphMode::Crossfade => "Crossfade",
                    MorphMode::Spectral => "Spectral",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.morph, MorphMode::Crossfade, "Crossfade");
                    ui.selectable_value(&mut self.morph, MorphMode::Spectral, "Spectral");
                });
        });
    }

    /// The frames in perspective, back to front; click a line to select it,
    /// scroll to move the selection.
    fn stack(&mut self, ui: &mut Ui, table: &Table) -> Rect {
        let (response, painter) = ui.allocate_painter(vec2(WIDTH, STACK_HEIGHT), Sense::click());
        let rect = response.rect;
        painter.rect_filled(rect, 3.0, theme::LCD_BG);
        let frames = table.frames();
        let shown = shown_frames(frames);
        let geometry = StackGeometry::new(rect);
        for &frame in shown.iter().rev() {
            let depth = depth(frame, frames);
            let samples = table.frame(frame).unwrap_or(&[]);
            let points = geometry.line(samples, depth);
            let color = if frame == self.selected {
                theme::HIGHLIGHT
            } else {
                fade(theme::ACCENT, 1.0 - 0.75 * depth)
            };
            let width = if frame == self.selected { 2.0 } else { 1.0 };
            painter.add(egui::Shape::line(points, Stroke::new(width, color)));
        }
        // The selected frame on top, even when it is not one of the shown ones.
        if !shown.contains(&self.selected) {
            let samples = table.frame(self.selected).unwrap_or(&[]);
            let points = geometry.line(samples, depth(self.selected, frames));
            painter.add(egui::Shape::line(
                points,
                Stroke::new(2.0, theme::HIGHLIGHT),
            ));
        }
        if response.clicked()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            self.selected = geometry.nearest(table, &shown, pointer);
        }
        if response.hovered() {
            // Wheel events, not smoothed scrolling: one notch, one frame.
            let scroll: f32 = ui.input(|i| {
                i.raw
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::MouseWheel { delta, .. } => Some(delta.y),
                        _ => None,
                    })
                    .sum()
            });
            if scroll > 0.0 {
                self.selected = (self.selected + 1).min(frames - 1);
            } else if scroll < 0.0 {
                self.selected = self.selected.saturating_sub(1);
            }
        }
        response.hint("Click a frame to select it; scroll to step through the frames");
        rect
    }

    /// The selected frame's waveform; dragging draws through it.
    fn draw_view(&mut self, ui: &mut Ui, frame: &[f32], edits: &mut Vec<WavetableEdit>) -> Rect {
        let (response, painter) = ui.allocate_painter(vec2(WIDTH, VIEW_HEIGHT), Sense::drag());
        let rect = response.rect;
        painter.rect_filled(rect, 3.0, theme::LCD_BG);
        painter.hline(
            rect.x_range(),
            rect.center().y,
            Stroke::new(1.0, theme::GRID_LINE),
        );
        let points: Vec<Pos2> = (0..=WIDTH as usize)
            .map(|px| {
                let t = px as f32 / WIDTH;
                let i = ((t * FRAME_LEN as f32) as usize).min(FRAME_LEN - 1);
                let v = frame.get(i).copied().unwrap_or(0.0);
                pos2(rect.left() + t * rect.width(), value_y(rect, v))
            })
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(1.5, theme::ACCENT)));
        if response.drag_started() {
            edits.push(WavetableEdit::Begin);
            self.last_draw = None;
        }
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let index =
                ((pointer.x - rect.left()) / rect.width()).clamp(0.0, 1.0) * FRAME_LEN as f32;
            let value = ((rect.center().y - pointer.y) / (rect.height() / 2.0)).clamp(-1.0, 1.0);
            let (start, values) = stroke(self.last_draw, (index, value), FRAME_LEN as f32 / WIDTH);
            self.last_draw = Some((index, value));
            edits.push(WavetableEdit::Op(Op::Draw {
                frame: self.selected,
                start,
                values,
            }));
        }
        if response.drag_stopped() {
            self.last_draw = None;
        }
        response.hint("Drag to draw the frame");
        rect
    }

    /// Bars for the first harmonics: magnitude above, phase below.
    fn harmonics(
        &mut self,
        ui: &mut Ui,
        frame: &[f32],
        edits: &mut Vec<WavetableEdit>,
    ) -> (Rect, Rect) {
        let partials = partials(frame);
        let peak = partials
            .iter()
            .take(BARS)
            .fold(1.0_f32, |m, p| m.max(p.magnitude));
        let magnitude = self.bars(
            ui,
            &partials,
            VIEW_HEIGHT,
            edits,
            |p| p.magnitude / peak,
            |p, v| Partial {
                magnitude: v * peak,
                ..p
            },
        );
        let phase = self.bars(
            ui,
            &partials,
            PHASE_HEIGHT,
            edits,
            |p| p.phase,
            |p, v| Partial { phase: v, ..p },
        );
        (magnitude, phase)
    }

    /// One row of bars; dragging sets each bar under the pointer.
    fn bars(
        &mut self,
        ui: &mut Ui,
        partials: &[Partial],
        height: f32,
        edits: &mut Vec<WavetableEdit>,
        get: impl Fn(Partial) -> f32,
        set: impl Fn(Partial, f32) -> Partial,
    ) -> Rect {
        let (response, painter) = ui.allocate_painter(vec2(WIDTH, height), Sense::drag());
        let rect = response.rect;
        painter.rect_filled(rect, 3.0, theme::LCD_BG);
        let bar = rect.width() / BARS as f32;
        for (h, partial) in partials.iter().take(BARS).enumerate() {
            let v = get(*partial).clamp(0.0, 1.0);
            let left = rect.left() + h as f32 * bar;
            let top = rect.bottom() - v * rect.height();
            let fill = if h % 2 == 0 {
                theme::ACCENT
            } else {
                theme::ACCENT_DIM
            };
            painter.rect_filled(
                Rect::from_min_max(pos2(left + 1.0, top), pos2(left + bar - 1.0, rect.bottom())),
                0.0,
                fill,
            );
        }
        if response.drag_started() {
            edits.push(WavetableEdit::Begin);
        }
        if response.dragged()
            && let Some(pointer) = response.interact_pointer_pos()
        {
            let h = (((pointer.x - rect.left()) / bar).floor().max(0.0) as usize).min(BARS - 1);
            let v = ((rect.bottom() - pointer.y) / rect.height()).clamp(0.0, 1.0);
            if let Some(&old) = partials.get(h) {
                let new = set(old, v);
                if new != old {
                    edits.push(WavetableEdit::Op(Op::SetPartial {
                        frame: self.selected,
                        harmonic: h + 1,
                        magnitude: new.magnitude,
                        phase: new.phase,
                    }));
                }
            }
        }
        response.hint("Drag the bars: harmonics 1 to 64");
        rect
    }

    fn formula_tab(&mut self, ui: &mut Ui, table: &Table, edits: &mut Vec<WavetableEdit>) {
        ui.label("x: phase in the frame (0–1), y: position across the frames (0–1)");
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.formula).desired_width(WIDTH - 70.0));
            if ui.button("Apply").clicked() {
                self.apply_formula(table.frames(), edits);
            }
        });
        if let Some(error) = &self.error {
            ui.colored_label(theme::DANGER, error);
        }
    }

    /// Checks the formula and reports it, or keeps the error to show.
    fn apply_formula(&mut self, frames: usize, edits: &mut Vec<WavetableEdit>) {
        match Formula::parse_table(&self.formula) {
            Ok(_) => {
                self.error = None;
                let (first, last) = self.range(frames);
                edits.push(WavetableEdit::Begin);
                edits.push(WavetableEdit::Op(Op::Formula {
                    text: self.formula.clone(),
                    first,
                    last,
                }));
            }
            Err(error) => self.error = Some(error),
        }
    }
}

/// The frames the stack draws: all of them, or 64 evenly spaced.
fn shown_frames(frames: usize) -> Vec<usize> {
    if frames <= STACK_LINES {
        return (0..frames).collect();
    }
    let mut shown: Vec<usize> = (0..STACK_LINES)
        .map(|i| (i * (frames - 1) + (STACK_LINES - 1) / 2) / (STACK_LINES - 1))
        .collect();
    shown.dedup();
    shown
}

/// 0 for the front frame, 1 for the back one.
fn depth(frame: usize, frames: usize) -> f32 {
    if frames > 1 {
        frame as f32 / (frames - 1) as f32
    } else {
        0.0
    }
}

fn fade(color: Color32, alpha: f32) -> Color32 {
    color.gamma_multiply(alpha.clamp(0.0, 1.0))
}

/// Where each frame's line sits in the stack: later frames further back,
/// up and to the right, and narrower.
struct StackGeometry {
    rect: Rect,
    /// Horizontal and vertical shift of the back frame.
    shift: egui::Vec2,
    /// Half the height of one frame's waveform.
    amplitude: f32,
}

impl StackGeometry {
    fn new(rect: Rect) -> Self {
        let rect = rect.shrink(8.0);
        Self {
            rect,
            shift: vec2(rect.width() * 0.3, -rect.height() * 0.55),
            amplitude: rect.height() * 0.2,
        }
    }

    fn baseline(&self, depth: f32) -> (f32, f32, f32) {
        let left = self.rect.left() + self.shift.x * depth;
        let width = self.rect.width() - self.shift.x;
        let y = self.rect.bottom() - self.amplitude + self.shift.y * depth;
        (left, width, y)
    }

    fn line(&self, samples: &[f32], depth: f32) -> Vec<Pos2> {
        let (left, width, y) = self.baseline(depth);
        (0..=STACK_POINTS)
            .map(|p| {
                let t = p as f32 / STACK_POINTS as f32;
                let i = ((t * FRAME_LEN as f32) as usize).min(FRAME_LEN - 1);
                let v = samples.get(i).copied().unwrap_or(0.0).clamp(-1.0, 1.0);
                pos2(left + t * width, y - v * self.amplitude)
            })
            .collect()
    }

    /// The shown frame whose line passes closest to `pointer`.
    fn nearest(&self, table: &Table, shown: &[usize], pointer: Pos2) -> usize {
        let frames = table.frames();
        let mut best = (f32::INFINITY, 0);
        for &frame in shown {
            let (left, width, y) = self.baseline(depth(frame, frames));
            let t = (pointer.x - left) / width;
            if !(0.0..=1.0).contains(&t) {
                continue;
            }
            let i = ((t * FRAME_LEN as f32) as usize).min(FRAME_LEN - 1);
            let v = table
                .frame(frame)
                .and_then(|f| f.get(i))
                .copied()
                .unwrap_or(0.0);
            let distance = (y - v.clamp(-1.0, 1.0) * self.amplitude - pointer.y).abs();
            if distance < best.0 {
                best = (distance, frame);
            }
        }
        best.1
    }
}

/// The y of value `v` (-1..1) in `rect`.
fn value_y(rect: Rect, v: f32) -> f32 {
    rect.center().y - v.clamp(-1.0, 1.0) * rect.height() / 2.0
}

/// The samples a draw stroke writes from the previous point to `to`
/// (sample index, value), interpolated, at least `span` samples wide so a
/// single point covers its pixel. Returns the first index and the values.
fn stroke(from: Option<(f32, f32)>, to: (f32, f32), span: f32) -> (usize, Vec<f32>) {
    let (from_i, from_v) = from.unwrap_or((to.0 - span / 2.0, to.1));
    let (a, b) = if from_i <= to.0 {
        ((from_i, from_v), to)
    } else {
        (to, (from_i, from_v))
    };
    let start = a.0.floor().clamp(0.0, (FRAME_LEN - 1) as f32) as usize;
    let end = (b.0.ceil().max(a.0 + span / 2.0) as usize).clamp(start + 1, FRAME_LEN);
    let values = (start..end)
        .map(|i| {
            let t = if b.0 > a.0 {
                ((i as f32 - a.0) / (b.0 - a.0)).clamp(0.0, 1.0)
            } else {
                0.0
            };
            a.1 + (b.1 - a.1) * t
        })
        .collect();
    (start, values)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, RawInput};

    fn frame(
        ctx: &egui::Context,
        editor: &mut WavetableEditor,
        table: &Table,
        events: Vec<Event>,
    ) -> (Vec<WavetableEdit>, Areas) {
        let mut edits = Vec::new();
        let mut areas = Areas::default();
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600.0, 700.0))),
            events,
            ..RawInput::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            areas = editor.show_areas(ui, table, &mut edits);
        });
        output.textures_delta.clear();
        (edits, areas)
    }

    fn button(pos: Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn gesture(
        ctx: &egui::Context,
        editor: &mut WavetableEditor,
        table: &Table,
        points: &[Pos2],
    ) -> Vec<WavetableEdit> {
        let mut edits = Vec::new();
        let (first, rest) = points.split_first().unwrap();
        edits.extend(
            frame(
                ctx,
                editor,
                table,
                vec![Event::PointerMoved(*first), button(*first, true)],
            )
            .0,
        );
        for p in rest {
            edits.extend(frame(ctx, editor, table, vec![Event::PointerMoved(*p)]).0);
        }
        let last = *points.last().unwrap();
        edits.extend(frame(ctx, editor, table, vec![button(last, false)]).0);
        edits
    }

    #[test]
    fn clicking_the_stack_selects_a_frame() {
        let ctx = egui::Context::default();
        let table = factory::table("basic").unwrap();
        let mut editor = WavetableEditor::default();
        let (_, areas) = frame(&ctx, &mut editor, &table, vec![]);
        // A point on frame 40's line.
        let geometry = StackGeometry::new(areas.stack);
        let line = geometry.line(table.frame(40).unwrap(), depth(40, 64));
        let at = line[STACK_POINTS / 8];
        let edits = gesture(&ctx, &mut editor, &table, &[at]);
        assert!(edits.is_empty(), "{edits:?}");
        assert_eq!(editor.selected, 40);
    }

    #[test]
    fn drawing_is_one_gesture_on_the_selected_frame() {
        let ctx = egui::Context::default();
        let table = Table::from_fn(4, |_, _| 0.0);
        let mut editor = WavetableEditor {
            selected: 2,
            ..WavetableEditor::default()
        };
        let (_, areas) = frame(&ctx, &mut editor, &table, vec![]);
        let v = areas.view;
        let edits = gesture(
            &ctx,
            &mut editor,
            &table,
            &[
                pos2(v.left() + 50.0, v.top() + 10.0),
                pos2(v.left() + 80.0, v.top() + 10.0),
                pos2(v.left() + 120.0, v.top() + 10.0),
            ],
        );
        assert_eq!(
            edits.iter().filter(|e| **e == WavetableEdit::Begin).count(),
            1
        );
        assert_eq!(edits.first(), Some(&WavetableEdit::Begin));
        let mut drawn = table.clone();
        for edit in &edits {
            if let WavetableEdit::Op(op) = edit {
                assert!(matches!(op, Op::Draw { frame: 2, .. }), "{op:?}");
                op.apply(&mut drawn).unwrap();
            }
        }
        // The samples under the stroke are high, with no gap between points.
        let f = drawn.frame(2).unwrap();
        let per_px = FRAME_LEN as f32 / WIDTH;
        let from = (60.0 * per_px) as usize;
        let to = (110.0 * per_px) as usize;
        assert!(f[from..to].iter().all(|&x| x > 0.8), "a gap in the stroke");
        assert!(f[FRAME_LEN - 10..].iter().all(|&x| x == 0.0));
    }

    #[test]
    fn dragging_a_harmonic_bar_sets_that_partial() {
        let ctx = egui::Context::default();
        let table = Table::from_fn(1, |_, x| (x * std::f32::consts::TAU).sin());
        let mut editor = WavetableEditor {
            tab: Tab::Harmonics,
            ..WavetableEditor::default()
        };
        let (_, areas) = frame(&ctx, &mut editor, &table, vec![]);
        let v = areas.view;
        let bar = v.width() / BARS as f32;
        // Harmonic 3 to half height.
        let at = pos2(v.left() + 2.5 * bar, v.center().y);
        let edits = gesture(&ctx, &mut editor, &table, &[at, at + vec2(0.0, 1.0)]);
        let set: Vec<&Op> = edits
            .iter()
            .filter_map(|e| match e {
                WavetableEdit::Op(op) => Some(op),
                _ => None,
            })
            .collect();
        assert!(!set.is_empty());
        for op in set {
            let Op::SetPartial {
                harmonic,
                magnitude,
                ..
            } = op
            else {
                panic!("{op:?}");
            };
            assert_eq!(*harmonic, 3);
            assert!((magnitude - 0.5).abs() < 0.02, "{magnitude}");
        }
        assert!(areas.phase.height() > 0.0);
    }

    #[test]
    fn formulas_report_once_or_show_their_error() {
        let mut editor = WavetableEditor {
            all_frames: true,
            ..WavetableEditor::default()
        };
        let mut edits = Vec::new();
        editor.apply_formula(8, &mut edits);
        assert_eq!(edits.len(), 2);
        assert!(matches!(
            &edits[1],
            WavetableEdit::Op(Op::Formula {
                first: 0,
                last: 7,
                ..
            })
        ));
        edits.clear();
        editor.formula = "sin(t)".to_owned();
        editor.apply_formula(8, &mut edits);
        assert!(edits.is_empty());
        assert!(editor.error.is_some());
    }

    #[test]
    fn tools_apply_to_the_selection_or_every_frame() {
        let mut editor = WavetableEditor {
            selected: 3,
            ..WavetableEditor::default()
        };
        assert_eq!(
            editor.tool_op(Tool::Normalize, 10),
            Op::Normalize { first: 3, last: 3 }
        );
        editor.all_frames = true;
        assert_eq!(
            editor.tool_op(Tool::RemoveDc, 10),
            Op::RemoveDc { first: 0, last: 9 }
        );
        assert_eq!(
            editor.tool_op(Tool::DeleteFrame, 10),
            Op::DeleteFrame { at: 3 }
        );
        editor.morph = MorphMode::Spectral;
        assert_eq!(
            editor.tool_op(Tool::Morph, 10),
            Op::Morph {
                first: 0,
                last: 9,
                mode: MorphMode::Spectral
            }
        );
    }

    #[test]
    fn the_stack_shows_at_most_64_frames_including_both_ends() {
        let shown = shown_frames(256);
        assert_eq!(shown.len(), 64);
        assert_eq!((shown[0], shown[63]), (0, 255));
        assert_eq!(shown_frames(3), [0, 1, 2]);
    }

    #[test]
    fn a_single_point_draws_its_pixel() {
        let (start, values) = stroke(None, (100.0, 0.5), 4.0);
        assert_eq!(start, 98);
        assert!(values.len() >= 2 && values.iter().all(|&v| v == 0.5));
        let (start, values) = stroke(Some((10.0, 0.0)), (20.0, 1.0), 4.0);
        assert_eq!(start, 10);
        assert_eq!(values.len(), 10);
        assert!((values[5] - 0.5).abs() < 1e-6);
    }
}
