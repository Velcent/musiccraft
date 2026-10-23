//! The automation curve shared by the automation editor and automation clips
//! in the playlist: drawing, point gestures and the point menu
//! (`docs/spec/playlist.md`, "Automation editor (UI)").

use std::collections::{BTreeMap, BTreeSet};

use egui::{Align2, Color32, FontId, PointerButton, Pos2, Rect, RichText, Stroke, pos2, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::params::TICKS_PER_STEP;
use musiccraft_project::{Automation, AutomationId, AutomationPoint, Curve, Meter};

use crate::geometry::Snap;
use crate::{Action, theme};

/// Radius of a point.
pub(crate) const POINT_RADIUS: f32 = 4.5;
/// Radius of a tension handle: half a point.
const HANDLE_RADIUS: f32 = 2.5;
/// How far outside a point or handle the pointer still grabs it.
const SLOP: f32 = 3.0;

/// The shape names the editor shows, in menu order.
pub(crate) const SHAPES: [(Curve, &str); 4] = [
    (Curve::Single, "Curve"),
    (Curve::Double, "S-curve"),
    (Curve::Hold, "Hold"),
    (Curve::Stairs, "Steps"),
];

/// Where a curve sits on screen: `rect` holds values 0 (bottom) to 1 (top),
/// and tick `first` is at its left edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct View {
    pub rect: Rect,
    pub first: f32,
    pub px_per_tick: f32,
}

impl View {
    pub(crate) fn x(&self, tick: f32) -> f32 {
        self.rect.left() + (tick - self.first) * self.px_per_tick
    }

    pub(crate) fn tick(&self, x: f32) -> f32 {
        self.first + (x - self.rect.left()) / self.px_per_tick.max(1e-6)
    }

    pub(crate) fn y(&self, value: f32) -> f32 {
        self.rect.bottom() - value * self.rect.height()
    }

    pub(crate) fn value(&self, y: f32) -> f32 {
        ((self.rect.bottom() - y) / self.rect.height().max(1.0)).clamp(0.0, 1.0)
    }

    pub(crate) fn at(&self, tick: u32, value: f32) -> Pos2 {
        pos2(self.x(tick as f32), self.y(value))
    }
}

/// What the pointer is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    /// The point at this index.
    Point(usize),
    /// The tension handle of the segment starting at this point.
    Handle(usize),
    /// Elsewhere on the segment starting at this point.
    Segment(usize),
    Empty,
}

/// Where the tension handle of the segment from `points[i]` sits.
fn handle_at(view: &View, curve: &Automation, i: usize) -> Option<Pos2> {
    let (a, b) = (curve.points.get(i)?, curve.points.get(i + 1)?);
    let mid = (a.tick + b.tick) as f32 / 2.0;
    let value = curve.value_at(f64::from(mid))?;
    Some(pos2(view.x(mid), view.y(value)))
}

/// What is under `pos`: points first, then handles, then segments.
pub(crate) fn hit(view: &View, curve: &Automation, pos: Pos2) -> Target {
    let nearest = |reach: f32, at: &dyn Fn(usize) -> Option<Pos2>| {
        (0..curve.points.len())
            .filter_map(|i| Some((i, at(i)?.distance(pos))))
            .filter(|(_, d)| *d <= reach)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    if let Some(i) = nearest(POINT_RADIUS + SLOP, &|i| {
        let p = curve.points[i];
        Some(view.at(p.tick, p.value))
    }) {
        return Target::Point(i);
    }
    if let Some(i) = nearest(HANDLE_RADIUS + SLOP + 1.0, &|i| handle_at(view, curve, i)) {
        return Target::Handle(i);
    }
    let tick = view.tick(pos.x);
    curve
        .points
        .windows(2)
        .position(|w| w[0].tick as f32 <= tick && tick < w[1].tick as f32)
        .map_or(Target::Empty, Target::Segment)
}

/// A gesture in progress.
#[derive(Debug, Clone, PartialEq)]
enum Drag {
    /// Points being moved: the grabbed point's tick and value, the ticks of
    /// every moving point, and the offset so far.
    Move {
        grabbed: (u32, f32),
        ticks: BTreeSet<u32>,
        dt: i64,
        dv: f32,
    },
    /// A point being placed with the right button.
    Place { tick: u32, value: f32 },
    /// A segment being bent.
    Bend {
        start: u32,
        curve: Curve,
        tension: f32,
        sign: f32,
    },
    /// A selection box.
    Band {
        origin: Pos2,
        current: Pos2,
        add: bool,
    },
    /// Step drawing: the value drawn at each tick.
    Draw(BTreeMap<u32, f32>),
}

/// Editing state for one curve view.
#[derive(Debug, Clone, Default)]
pub(crate) struct Edit {
    /// The automation the state belongs to.
    automation: Option<AutomationId>,
    /// The playlist clip being edited, as (track, start).
    pub clip: Option<(u32, u32)>,
    /// Selected points by tick.
    pub selection: BTreeSet<u32>,
    drag: Option<Drag>,
    /// The point whose menu is open.
    menu: Option<u32>,
    /// The menu opens this frame.
    open_menu: bool,
    /// The point whose value is being typed, and the text.
    typing: Option<(u32, String)>,
    /// A copied point value.
    copied: Option<f32>,
}

impl Edit {
    /// Forgets the selection when the view switches automation.
    pub(crate) fn follow(&mut self, id: AutomationId) {
        if self.automation != Some(id) {
            *self = Self {
                automation: Some(id),
                copied: self.copied,
                ..Self::default()
            };
        }
    }

    /// Whether a gesture is in progress.
    pub(crate) fn busy(&self) -> bool {
        self.drag.is_some()
    }

    /// Whether a point's value is being typed.
    pub(crate) fn typing(&self) -> bool {
        self.typing.is_some()
    }
}

/// Toggles of the editor footer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Options {
    pub snap: bool,
    pub step: bool,
    pub slide: bool,
    /// Inside a playlist clip: only gestures on points, handles and the
    /// right button belong to the curve; the rest move the clip.
    pub in_clip: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            snap: true,
            step: false,
            slide: false,
            in_clip: false,
        }
    }
}

/// The points as shown, with the gesture in progress applied.
pub(crate) fn shown(automation: &Automation, edit: &Edit) -> Automation {
    let mut curve = automation.clone();
    match &edit.drag {
        Some(Drag::Move { ticks, dt, dv, .. }) => {
            for p in &mut curve.points {
                if ticks.contains(&p.tick) {
                    p.tick = shift(p.tick, *dt);
                    p.value = (p.value + dv).clamp(0.0, 1.0);
                }
            }
            curve.sort_points();
        }
        Some(Drag::Place { tick, value }) => {
            curve.points.retain(|p| p.tick != *tick);
            curve.points.push(AutomationPoint::new(*tick, *value));
            curve.sort_points();
        }
        Some(Drag::Bend { start, tension, .. }) => {
            if let Some(p) = curve.points.iter_mut().find(|p| p.tick == *start) {
                p.tension = *tension;
            }
        }
        Some(Drag::Draw(drawn)) => curve.points = drawn_points(&curve.points, drawn),
        Some(Drag::Band { .. }) | None => {}
    }
    curve
}

fn shift(tick: u32, dt: i64) -> u32 {
    u32::try_from((i64::from(tick) + dt).max(0)).unwrap_or(u32::MAX)
}

/// `points` with the drawn stretch replacing the points between its ends.
fn drawn_points(points: &[AutomationPoint], drawn: &BTreeMap<u32, f32>) -> Vec<AutomationPoint> {
    let (Some(lo), Some(hi)) = (drawn.keys().next(), drawn.keys().next_back()) else {
        return points.to_vec();
    };
    let mut out: Vec<AutomationPoint> = points
        .iter()
        .filter(|p| p.tick < *lo || p.tick > *hi)
        .copied()
        .collect();
    out.extend(drawn.iter().map(|(t, v)| AutomationPoint::new(*t, *v)));
    out.sort_by_key(|p| p.tick);
    out
}

/// How far the moving points may shift in time without passing a point that
/// stays, or going before tick 0.
fn clamp_dt(points: &[AutomationPoint], moving: &BTreeSet<u32>, dt: i64) -> i64 {
    let (mut lo, mut hi) = (i64::MIN, i64::MAX);
    for (i, p) in points.iter().enumerate() {
        if !moving.contains(&p.tick) {
            continue;
        }
        let tick = i64::from(p.tick);
        lo = lo.max(-tick);
        if let Some(prev) = i.checked_sub(1).and_then(|j| points.get(j))
            && !moving.contains(&prev.tick)
        {
            lo = lo.max(i64::from(prev.tick) + 1 - tick);
        }
        if let Some(next) = points.get(i + 1)
            && !moving.contains(&next.tick)
        {
            hi = hi.min(i64::from(next.tick) - 1 - tick);
        }
    }
    if lo > hi { 0 } else { dt.clamp(lo, hi) }
}

/// Points as `automation.set_points` reads them.
pub(crate) fn points_json(points: &[AutomationPoint]) -> String {
    let rows: Vec<String> = points
        .iter()
        .map(|p| {
            format!(
                r#"{{"tick":{},"value":{},"curve":"{}","tension":{}}}"#,
                p.tick,
                p.value.clamp(0.0, 1.0),
                p.curve.name(),
                p.tension.clamp(-1.0, 1.0)
            )
        })
        .collect();
    format!("[{}]", rows.join(","))
}

/// `automation.set_points` for `number` (1-based).
pub(crate) fn set_points(number: usize, points: &[AutomationPoint]) -> Action {
    Action::Run(Invocation::new(
        "automation.set_points",
        [number.to_string(), points_json(points)],
    ))
}

fn run(actions: &mut Vec<Action>, name: &str, args: Vec<String>) {
    actions.push(Action::Run(Invocation::new(name, args)));
}

fn t(tick: u32) -> String {
    format!("{tick}t")
}

/// How the curve is drawn.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Look {
    pub line: Color32,
    pub fill: Color32,
    /// Draw points and tension handles.
    pub points: bool,
}

/// Draws the curve, its fill and (with `look.points`) its points and
/// handles, from the shown points.
pub(crate) fn draw(
    painter: &egui::Painter,
    view: &View,
    curve: &Automation,
    edit: &Edit,
    look: Look,
) {
    let rect = view.rect;
    if curve.points.is_empty() {
        return;
    }
    let steps = (rect.width() / 2.0).clamp(2.0, 2000.0) as usize;
    let line: Vec<Pos2> = (0..=steps)
        .filter_map(|i| {
            let x = rect.left() + rect.width() * i as f32 / steps as f32;
            let v = curve.value_at(f64::from(view.tick(x).max(0.0)))?;
            Some(pos2(x, view.y(v)))
        })
        .collect();
    for pair in line.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        painter.add(egui::Shape::convex_polygon(
            vec![a, b, pos2(b.x, rect.bottom()), pos2(a.x, rect.bottom())],
            look.fill,
            Stroke::NONE,
        ));
    }
    painter.add(egui::Shape::line(line, Stroke::new(1.8, look.line)));
    if !look.points {
        return;
    }
    let hovered = painter
        .ctx()
        .pointer_hover_pos()
        .filter(|p| rect.expand(POINT_RADIUS + SLOP).contains(*p))
        .map_or(Target::Empty, |p| hit(view, curve, p));
    let bending = match &edit.drag {
        Some(Drag::Bend { start, .. }) => curve.points.iter().position(|p| p.tick == *start),
        _ => None,
    };
    for i in 0..curve.points.len().saturating_sub(1) {
        if let Some(at) = handle_at(view, curve, i) {
            let lit = hovered == Target::Handle(i) || bending == Some(i);
            let (radius, color) = if lit {
                (HANDLE_RADIUS + 1.0, theme::HIGHLIGHT)
            } else {
                (HANDLE_RADIUS, look.line)
            };
            painter.circle_stroke(at, radius, Stroke::new(1.2, color));
        }
    }
    for (i, p) in curve.points.iter().enumerate() {
        let at = view.at(p.tick, p.value);
        let selected = edit.selection.contains(&p.tick);
        let mut radius = POINT_RADIUS;
        if hovered == Target::Point(i) {
            radius += 1.0;
        }
        if selected {
            radius += 1.0;
        }
        painter.circle_filled(at, radius, look.line);
        painter.circle_stroke(at, radius, Stroke::new(1.0, theme::PANEL_DARK));
        if selected {
            painter.circle_stroke(at, radius + 2.0, Stroke::new(1.5, theme::HIGHLIGHT));
        }
    }
    if let Some(Drag::Band {
        origin, current, ..
    }) = edit.drag
    {
        let area = Rect::from_two_pos(origin, current);
        painter.rect_filled(area, 0.0, theme::SELECTION_FILL);
        painter.rect_stroke(
            area,
            0.0,
            Stroke::new(1.0, theme::ACCENT),
            egui::StrokeKind::Inside,
        );
    }
}

/// The hint for what is under `pos`, and the floating readout while a
/// point or handle is dragged.
pub(crate) fn describe(
    view: &View,
    curve: &Automation,
    edit: &Edit,
    meter: &Meter,
    pos: Pos2,
) -> Option<String> {
    let point = |i: usize| {
        let p = curve.points.get(i)?;
        let at = meter.position(p.tick);
        Some(format!(
            "Point {} of {} · {:.0} % · bar {}.{}",
            i + 1,
            curve.points.len(),
            p.value * 100.0,
            at.bar,
            at.beat
        ))
    };
    let tension = |i: usize| {
        let p = curve.points.get(i)?;
        Some(format!("Tension {:+.2}", p.tension))
    };
    match &edit.drag {
        Some(Drag::Move { grabbed, dt, .. }) => {
            let tick = shift(grabbed.0, *dt);
            point(curve.points.iter().position(|p| p.tick == tick)?)
        }
        Some(Drag::Place { tick, .. }) => point(curve.points.iter().position(|p| p.tick == *tick)?),
        Some(Drag::Bend { start, .. }) => {
            tension(curve.points.iter().position(|p| p.tick == *start)?)
        }
        _ => match hit(view, curve, pos) {
            Target::Point(i) => point(i),
            Target::Handle(i) => tension(i),
            _ => None,
        },
    }
}

/// Draws the floating readout beside the pointer while dragging.
pub(crate) fn readout(painter: &egui::Painter, edit: &Edit, text: Option<&str>, pos: Pos2) {
    let dragging = matches!(
        edit.drag,
        Some(Drag::Move { .. } | Drag::Place { .. } | Drag::Bend { .. })
    );
    if let (true, Some(text)) = (dragging, text) {
        painter.text(
            pos + vec2(12.0, -14.0),
            Align2::LEFT_BOTTOM,
            text,
            FontId::monospace(theme::FONT_LABEL),
            theme::TEXT,
        );
    }
}

/// The tick under `x`, snapped unless `free`.
fn snapped(view: &View, x: f32, meter: &Meter, snap: bool) -> u32 {
    let tick = view.tick(x).max(0.0);
    if snap {
        Snap::Step.round(tick, meter)
    } else {
        tick.round() as u32
    }
}

/// Runs the curve gestures on `response` (`docs/spec/playlist.md`,
/// "Automation editor (UI)"). Returns whether the curve took the pointer
/// this frame, so a playlist clip leaves it alone.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(crate) fn interact(
    ui: &egui::Ui,
    response: &egui::Response,
    view: &View,
    (number, automation): (usize, &Automation),
    meter: &Meter,
    opts: Options,
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) -> bool {
    edit.follow(automation.id);
    let (alt, shift_key, ctrl) =
        ui.input(|i| (i.modifiers.alt, i.modifiers.shift, i.modifiers.command));
    let reach = view.rect.expand(POINT_RADIUS + SLOP);
    let origin = ui
        .input(|i| i.pointer.press_origin())
        .filter(|p| reach.contains(*p));
    let pointer = response.interact_pointer_pos();
    let points = &automation.points;
    let mut took = edit.drag.is_some();

    // Gestures starting.
    if edit.drag.is_none()
        && let Some(origin) = origin
    {
        if response.drag_started_by(PointerButton::Primary) {
            let target = hit(view, automation, origin);
            edit.drag = match target {
                Target::Point(i) if !(ctrl && !opts.in_clip) => {
                    let p = points[i];
                    if !edit.selection.contains(&p.tick) {
                        edit.selection = BTreeSet::from([p.tick]);
                    }
                    let mut ticks = edit.selection.clone();
                    if opts.slide {
                        ticks.extend(points.iter().filter(|q| q.tick > p.tick).map(|q| q.tick));
                    }
                    Some(Drag::Move {
                        grabbed: (p.tick, p.value),
                        ticks,
                        dt: 0,
                        dv: 0.0,
                    })
                }
                Target::Handle(i) => Some(bend(automation, i)),
                Target::Segment(i) if alt => Some(bend(automation, i)),
                _ if opts.in_clip => None,
                _ if opts.step && !ctrl => Some(Drag::Draw(BTreeMap::new())),
                _ => Some(Drag::Band {
                    origin,
                    current: origin,
                    add: shift_key,
                }),
            };
        } else if response.drag_started_by(PointerButton::Secondary)
            && matches!(
                hit(view, automation, origin),
                Target::Empty | Target::Segment(_)
            )
        {
            edit.drag = Some(Drag::Place {
                tick: snapped(view, origin.x, meter, opts.snap && !alt),
                value: view.value(origin.y),
            });
        }
        took |= edit.drag.is_some();
    }

    // Gestures in progress.
    if let Some(pos) = pointer.filter(|_| response.dragged()) {
        match &mut edit.drag {
            Some(Drag::Move {
                grabbed,
                ticks,
                dt,
                dv,
            }) => {
                let tick = snapped(view, pos.x, meter, opts.snap && !alt);
                *dt = if ctrl {
                    0
                } else {
                    clamp_dt(points, ticks, i64::from(tick) - i64::from(grabbed.0))
                };
                *dv = if shift_key {
                    0.0
                } else {
                    view.value(pos.y) - grabbed.1
                };
            }
            Some(Drag::Place { tick, value }) => {
                *tick = snapped(view, pos.x, meter, opts.snap && !alt);
                *value = view.value(pos.y);
            }
            Some(Drag::Bend { tension, sign, .. }) => {
                let fine = if ctrl { 4.0 } else { 1.0 };
                let dy = response.drag_delta().y;
                *tension = (*tension + dy / 80.0 / fine * *sign).clamp(-1.0, 1.0);
            }
            Some(Drag::Band { current, .. }) => *current = pos,
            Some(Drag::Draw(drawn)) => draw_to(drawn, view, meter, opts.snap && !alt, pos),
            None => {}
        }
    }

    // Gestures ending.
    if response.drag_stopped()
        && let Some(drag) = edit.drag.take()
    {
        took = true;
        finish(drag, view, (number, automation), edit, actions);
    }

    // Clicks.
    let click_at = pointer.filter(|p| reach.contains(*p));
    if let Some(pos) = click_at.filter(|_| response.clicked_by(PointerButton::Primary)) {
        match hit(view, automation, pos) {
            Target::Point(i) if alt => {
                if points.len() > 1 {
                    run(
                        actions,
                        "automation.remove_point",
                        vec![number.to_string(), t(points[i].tick)],
                    );
                    edit.selection.remove(&points[i].tick);
                }
                took = true;
            }
            Target::Point(i) => {
                let tick = points[i].tick;
                if shift_key {
                    if !edit.selection.remove(&tick) {
                        edit.selection.insert(tick);
                    }
                } else {
                    edit.selection = BTreeSet::from([tick]);
                }
                took = true;
            }
            _ if opts.in_clip => {}
            _ => edit.selection.clear(),
        }
    }
    if let Some(pos) = click_at.filter(|_| response.secondary_clicked()) {
        took = true;
        match hit(view, automation, pos) {
            Target::Point(i) => {
                edit.menu = Some(points[i].tick);
                edit.open_menu = true;
            }
            Target::Handle(i) => {
                let p = points[i];
                run(
                    actions,
                    "automation.set_curve",
                    vec![number.to_string(), t(p.tick), p.curve.name().to_owned()],
                );
            }
            _ => {
                let tick = snapped(view, pos.x, meter, opts.snap && !alt);
                let value = if shift_key {
                    automation.value_at(f64::from(tick)).unwrap_or(0.5)
                } else {
                    view.value(pos.y)
                };
                run(
                    actions,
                    "automation.add_point",
                    vec![number.to_string(), t(tick), format!("{value:.4}")],
                );
            }
        }
    }
    took
}

/// Starts bending the segment from `points[i]`, towards the pointer.
fn bend(automation: &Automation, i: usize) -> Drag {
    let a = automation.points[i];
    let rising = automation
        .points
        .get(i + 1)
        .is_none_or(|b| b.value >= a.value);
    Drag::Bend {
        start: a.tick,
        curve: a.curve,
        tension: a.tension,
        // Dragging down on a rising segment slows its start.
        sign: if rising { 1.0 } else { -1.0 },
    }
}

/// Adds the drawn value at every snap line between the last drawn tick and
/// the pointer.
fn draw_to(drawn: &mut BTreeMap<u32, f32>, view: &View, meter: &Meter, snap: bool, pos: Pos2) {
    let tick = snapped(view, pos.x, meter, snap);
    let value = view.value(pos.y);
    let step = if snap { TICKS_PER_STEP } else { 1 };
    let last = drawn.iter().next_back().map(|(t, v)| (*t, *v));
    if let Some((from, from_value)) = last.filter(|(from, _)| from.abs_diff(tick) > step) {
        let (lo, hi) = (from.min(tick), from.max(tick));
        let mut at = lo + step;
        while at < hi {
            let share = (at as f32 - from as f32) / (tick as f32 - from as f32);
            drawn.insert(at, from_value + (value - from_value) * share);
            at += step;
        }
    }
    drawn.insert(tick, value);
}

/// Turns a finished gesture into its command.
fn finish(
    drag: Drag,
    view: &View,
    (number, automation): (usize, &Automation),
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) {
    let points = &automation.points;
    match drag {
        Drag::Move {
            grabbed,
            ticks,
            dt,
            dv,
        } => {
            if dt == 0 && dv == 0.0 {
                return;
            }
            if ticks.len() == 1 {
                let value = (grabbed.1 + dv).clamp(0.0, 1.0);
                run(
                    actions,
                    "automation.move_point",
                    vec![
                        number.to_string(),
                        t(grabbed.0),
                        t(shift(grabbed.0, dt)),
                        format!("{value:.4}"),
                    ],
                );
            } else {
                let moved = shown(
                    automation,
                    &Edit {
                        drag: Some(Drag::Move {
                            grabbed,
                            ticks: ticks.clone(),
                            dt,
                            dv,
                        }),
                        ..Edit::default()
                    },
                );
                actions.push(set_points(number, &moved.points));
            }
            edit.selection = edit.selection.iter().map(|t| shift(*t, dt)).collect();
        }
        Drag::Place { tick, value } => run(
            actions,
            "automation.add_point",
            vec![number.to_string(), t(tick), format!("{value:.4}")],
        ),
        Drag::Bend {
            start,
            curve,
            tension,
            ..
        } => run(
            actions,
            "automation.set_curve",
            vec![
                number.to_string(),
                t(start),
                curve.name().to_owned(),
                format!("{tension:.3}"),
            ],
        ),
        Drag::Band {
            origin,
            current,
            add,
        } => {
            let area = Rect::from_two_pos(origin, current);
            if !add {
                edit.selection.clear();
            }
            edit.selection.extend(
                points
                    .iter()
                    .filter(|p| area.contains(view.at(p.tick, p.value)))
                    .map(|p| p.tick),
            );
        }
        Drag::Draw(drawn) => {
            if !drawn.is_empty() {
                actions.push(set_points(number, &drawn_points(points, &drawn)));
            }
        }
    }
}

/// Removes the selected points (never the last one). One undo step.
pub(crate) fn delete_selection(
    (number, automation): (usize, &Automation),
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) {
    let kept: Vec<AutomationPoint> = automation
        .points
        .iter()
        .filter(|p| !edit.selection.contains(&p.tick))
        .copied()
        .collect();
    if kept.is_empty() || kept.len() == automation.points.len() {
        return;
    }
    actions.push(set_points(number, &kept));
    edit.selection.clear();
}

/// Selects every point.
pub(crate) fn select_all(automation: &Automation, edit: &mut Edit) {
    edit.selection = automation.points.iter().map(|p| p.tick).collect();
}

/// The point menu and the type-in field, anchored on `response`. Call every
/// frame after [`interact`].
pub(crate) fn menus(
    ui: &egui::Ui,
    response: &egui::Response,
    view: &View,
    (number, automation): (usize, &Automation),
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) {
    let id = response.id.with("point-menu");
    let opened = std::mem::take(&mut edit.open_menu);
    let at = edit
        .menu
        .and_then(|tick| automation.points.iter().find(|p| p.tick == tick))
        .map_or(view.rect.center(), |p| {
            view.at(p.tick, p.value) + vec2(6.0, 6.0)
        });
    egui::Popup::menu(response)
        .id(id)
        .open_memory(opened.then_some(egui::SetOpenCommand::Bool(true)))
        .anchor(egui::PopupAnchor::Position(at))
        .show(|ui| point_menu(ui, (number, automation), edit, actions));
    type_in(ui, view, (number, automation), edit, actions);
}

fn point_menu(
    ui: &mut egui::Ui,
    (number, automation): (usize, &Automation),
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) {
    let points = &automation.points;
    let Some(i) = edit
        .menu
        .and_then(|tick| points.iter().position(|p| p.tick == tick))
    else {
        ui.close();
        return;
    };
    let p = points[i];
    let dim = |text: &str| {
        RichText::new(text)
            .size(theme::FONT_LABEL)
            .color(theme::TEXT_DIM)
    };
    ui.label(dim(&format!("Point {} of {}", i + 1, points.len())));
    if crate::widgets::add_enabled(ui, points.len() > 1, egui::Button::new("Delete")).clicked() {
        run(
            actions,
            "automation.remove_point",
            vec![number.to_string(), t(p.tick)],
        );
        edit.selection.remove(&p.tick);
        ui.close();
    }
    // The shape belongs to the segment ending at this point, which the
    // point before it starts.
    if let Some(before) = i.checked_sub(1).and_then(|j| points.get(j)) {
        ui.separator();
        ui.label(dim("Shape of the segment before this point"));
        for (curve, label) in SHAPES {
            if ui.radio(before.curve == curve, label).clicked() {
                run(
                    actions,
                    "automation.set_curve",
                    vec![
                        number.to_string(),
                        t(before.tick),
                        curve.name().to_owned(),
                        format!("{:.3}", before.tension),
                    ],
                );
                ui.close();
            }
        }
        if crate::widgets::add_enabled(
            ui,
            before.tension != 0.0,
            egui::Button::new("Reset tension"),
        )
        .clicked()
        {
            run(
                actions,
                "automation.set_curve",
                vec![
                    number.to_string(),
                    t(before.tick),
                    before.curve.name().to_owned(),
                ],
            );
            ui.close();
        }
    }
    ui.separator();
    if ui.button("Copy value").clicked() {
        edit.copied = Some(p.value);
        ui.close();
    }
    if crate::widgets::add_enabled(ui, edit.copied.is_some(), egui::Button::new("Paste value"))
        .clicked()
        && let Some(value) = edit.copied
    {
        run(
            actions,
            "automation.move_point",
            vec![
                number.to_string(),
                t(p.tick),
                t(p.tick),
                format!("{value:.4}"),
            ],
        );
        ui.close();
    }
    if ui.button("Type value…").clicked() {
        edit.typing = Some((p.tick, format!("{:.0}", p.value * 100.0)));
        ui.close();
    }
}

/// The small field over a point while its value is typed (0–100).
fn type_in(
    ui: &egui::Ui,
    view: &View,
    (number, automation): (usize, &Automation),
    edit: &mut Edit,
    actions: &mut Vec<Action>,
) {
    let Some((tick, text)) = &mut edit.typing else {
        return;
    };
    let Some(p) = automation.points.iter().find(|p| p.tick == *tick).copied() else {
        edit.typing = None;
        return;
    };
    let id = egui::Id::new("automation-type-value");
    let mut done = None;
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fixed_pos(view.at(p.tick, p.value) + vec2(8.0, -10.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let field = ui.add(
                        egui::TextEdit::singleline(text)
                            .desired_width(44.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    ui.label(RichText::new("%").color(theme::TEXT_DIM));
                    if !field.has_focus() && !field.lost_focus() {
                        field.request_focus();
                    }
                    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if field.lost_focus() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                        done = Some(enter);
                    }
                });
            });
        });
    if let Some(apply) = done {
        if apply && let Ok(percent) = text.trim().trim_end_matches('%').trim().parse::<f32>() {
            let value = (percent / 100.0).clamp(0.0, 1.0);
            run(
                actions,
                "automation.move_point",
                vec![
                    number.to_string(),
                    t(p.tick),
                    t(p.tick),
                    format!("{value:.4}"),
                ],
            );
        }
        edit.typing = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pts(rows: &[(u32, f32)]) -> Vec<AutomationPoint> {
        rows.iter()
            .map(|(t, v)| AutomationPoint::new(*t, *v))
            .collect()
    }

    #[test]
    fn moving_points_cannot_pass_the_points_that_stay() {
        let points = pts(&[(0, 0.0), (96, 0.5), (192, 1.0)]);
        let middle = BTreeSet::from([96]);
        assert_eq!(clamp_dt(&points, &middle, 500), 95);
        assert_eq!(clamp_dt(&points, &middle, -500), -95);
        // Moving the last two together only stops at the first point.
        let tail = BTreeSet::from([96, 192]);
        assert_eq!(clamp_dt(&points, &tail, 1000), 1000);
        assert_eq!(clamp_dt(&points, &tail, -500), -95);
        // The first point stays at 0 or later.
        assert_eq!(clamp_dt(&points, &BTreeSet::from([0]), -10), 0);
    }

    #[test]
    fn a_drawn_stretch_replaces_the_points_inside_it() {
        let points = pts(&[(0, 0.0), (96, 0.5), (192, 1.0), (384, 0.2)]);
        let drawn = BTreeMap::from([(48, 0.9), (72, 0.8), (200, 0.1)]);
        let out = drawn_points(&points, &drawn);
        let ticks: Vec<u32> = out.iter().map(|p| p.tick).collect();
        assert_eq!(ticks, [0, 48, 72, 200, 384]);
    }

    #[test]
    fn points_json_reads_back_through_the_command() {
        let mut points = pts(&[(0, 0.25), (96, 1.0)]);
        points[0].curve = Curve::Hold;
        points[0].tension = 0.5;
        let mut s = musiccraft_commands::Session::new(
            musiccraft_project::Project::default(),
            musiccraft_commands::Registry::with_builtin(),
        );
        s.run_line("automation.create mixer:0:volume").unwrap();
        s.run(&Invocation::new(
            "automation.set_points",
            ["1".to_owned(), points_json(&points)],
        ))
        .unwrap();
        assert_eq!(s.project().automations[0].points, points);
    }
}
