//! Icons drawn with vector shapes (our own artwork; no icon fonts), plus the
//! toolbar's button and display widgets built on them.

use crate::hint::Hint;
use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

use crate::theme;

/// The icons the interface uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Icon {
    Play,
    Stop,
    Record,
    Metronome,
    Keyboard,
    Playlist,
    ChannelRack,
    PianoRoll,
    Mixer,
    Browser,
    /// Draw tool (pencil).
    Pencil,
    /// Paint tool (brush).
    Brush,
    /// Delete tool (eraser).
    Eraser,
    /// Select tool (dashed box).
    Select,
    /// Mute tool (speaker with a small cross).
    Mute,
    /// Slice tool (a blade across a bar).
    Slice,
    /// Zoom tool (magnifier).
    Zoom,
    /// Window menu (three short bars).
    Menu,
    /// Collapse to header (an upward chevron over a bar).
    Fold,
    /// Expand from the header (a downward chevron over a bar).
    Unfold,
    /// Maximize (corner brackets opening outward).
    Maximize,
    /// Restore (corner brackets turned inward).
    Restore,
    /// Close (two crossing strokes).
    Close,
    /// Tools that did not fit (two small right chevrons).
    More,
    /// Refresh (three quarters of a ring ending in an arrowhead).
    Refresh,
    /// Fold all (an upward chevron over two short bars).
    FoldAll,
    /// An open marker or a list (a small downward chevron).
    ChevronDown,
    /// An open folder (the folder outline with its front lifted).
    FolderOpen,
    /// Audio (a centre line crossed by bars of different heights).
    Wave,
    /// Plugins (a 2 × 2 grid of rounded squares, one outlined).
    Plugins,
    /// Presets (a knob: a circle with a pointer line).
    Knob,
    /// An effect (a box with an S-curve corner to corner).
    Effect,
    /// Automation (a zigzag with a dot at each node).
    Automation,
    /// A controller (one period of a sine wave).
    Sine,
    /// Sound packs (a box with a lid line).
    Pack,
    /// Snap to the grid (a dot resting on a grid line between two ticks).
    Snap,
    /// Play from here (a start line with a play triangle after it).
    PlayFrom,
    /// A pattern (three staggered note dashes).
    Pattern,
    /// Add (a plus).
    Plus,
    /// A small right-pointing chevron (slot menus).
    ChevronRight,
    /// Strip details (three stacked switch dots).
    Details,
    /// The mixer's slot panel (a frame with a column at its right).
    SidePanel,
    /// Invert phase (a circle crossed by a slash).
    Phase,
    /// Swap left and right (two opposed arrows).
    Swap,
    /// Pause, and play while playing (two upright bars).
    Pause,
    /// Wait for input (an hourglass).
    Wait,
    /// Window layout (a frame split into three panes, a small chevron under it).
    Layout,
    /// Count-in (three rising ticks and a play triangle).
    CountIn,
    /// Blend recording (two offset boxes, the front one filled).
    Blend,
    /// Loop recording (an arc with an arrowhead round a dot).
    LoopRecord,
    /// Step edit (a note bar and a chevron).
    StepEdit,
    /// Note tools (a wand with a sparkle at its tip).
    Wand,
    /// A scale (a staircase rising to the right).
    Scale,
    /// A chord (three stacked, staggered note heads).
    Chord,
    /// The piano roll's Play tool (a triangle with a sound arc).
    PlayNote,
    /// Small chevrons pointing left and up (scroll bars).
    ChevronLeft,
    ChevronUp,
    /// Portamento (a quarter curve rising to the right).
    Porta,
    /// Slide (a right-angled ramp rising to the right).
    SlideRamp,
    /// Confirm (a tick).
    Check,
    /// Output (a small circle with a short arrow leaving to the right).
    Out,
    /// Input (a short arrow entering a small circle from the left).
    In,
    /// Envelope page (a filled rise, fall, shelf and fall).
    Envelope,
    /// Functions page (three small squares climbing).
    Steps,
}

fn p(rect: Rect, x: f32, y: f32) -> Pos2 {
    pos2(
        rect.left() + rect.width() * x,
        rect.top() + rect.height() * y,
    )
}

/// Paints `icon` inside `rect`.
pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let stroke = Stroke::new(1.6, color);
    let r = rect.shrink(rect.width().min(rect.height()) * 0.18);
    match icon {
        Icon::Pencil => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.75, 0.05),
                    p(r, 0.95, 0.25),
                    p(r, 0.3, 0.9),
                    p(r, 0.1, 0.7),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.1, 0.7), p(r, 0.3, 0.9), p(r, 0.02, 0.98)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Brush => {
            painter.line_segment(
                [p(r, 0.95, 0.05), p(r, 0.45, 0.55)],
                Stroke::new(2.2, color),
            );
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.35, 0.5),
                    p(r, 0.55, 0.7),
                    p(r, 0.3, 0.98),
                    p(r, 0.02, 0.98),
                    p(r, 0.12, 0.7),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Eraser => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.55, 0.1),
                    p(r, 0.95, 0.5),
                    p(r, 0.55, 0.9),
                    p(r, 0.15, 0.5),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.35, 0.3),
                    p(r, 0.75, 0.7),
                    p(r, 0.55, 0.9),
                    p(r, 0.15, 0.5),
                ],
                color,
                Stroke::NONE,
            ));
            painter.line_segment([p(r, 0.05, 0.98), p(r, 0.95, 0.98)], stroke);
        }
        Icon::Mute => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.05, 0.35),
                    p(r, 0.3, 0.35),
                    p(r, 0.55, 0.1),
                    p(r, 0.55, 0.9),
                    p(r, 0.3, 0.65),
                    p(r, 0.05, 0.65),
                ],
                color,
                Stroke::NONE,
            ));
            painter.line_segment([p(r, 0.68, 0.3), p(r, 0.98, 0.7)], stroke);
            painter.line_segment([p(r, 0.98, 0.3), p(r, 0.68, 0.7)], stroke);
        }
        Icon::Slice => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.05, 0.3), p(r, 0.95, 0.7)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([p(r, 0.6, 0.0), p(r, 0.4, 1.0)], Stroke::new(2.0, color));
        }
        Icon::Zoom => {
            painter.circle_stroke(p(r, 0.42, 0.42), r.width() * 0.3, stroke);
            painter.line_segment(
                [p(r, 0.63, 0.63), p(r, 0.95, 0.95)],
                Stroke::new(2.2, color),
            );
        }
        Icon::Select => {
            let corners = [
                p(r, 0.05, 0.1),
                p(r, 0.95, 0.1),
                p(r, 0.95, 0.9),
                p(r, 0.05, 0.9),
            ];
            for i in 0..4 {
                painter.add(Shape::dashed_line(
                    &[corners[i], corners[(i + 1) % 4]],
                    stroke,
                    3.0,
                    2.5,
                ));
            }
        }
        Icon::Play => {
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.2, 0.05), p(r, 0.95, 0.5), p(r, 0.2, 0.95)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Stop => {
            painter.rect_filled(
                Rect::from_min_max(p(r, 0.12, 0.12), p(r, 0.88, 0.88)),
                2.0,
                color,
            );
        }
        Icon::Record => {
            painter.circle_filled(r.center(), r.width().min(r.height()) * 0.42, color);
        }
        Icon::Metronome => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.3, 0.05),
                    p(r, 0.7, 0.05),
                    p(r, 0.95, 0.95),
                    p(r, 0.05, 0.95),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
            painter.line_segment([p(r, 0.5, 0.75), p(r, 0.8, 0.15)], stroke);
        }
        Icon::Keyboard => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.0, 0.2), p(r, 1.0, 0.85)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            for i in 1..5 {
                let x = i as f32 / 5.0;
                painter.line_segment([p(r, x, 0.2), p(r, x, 0.85)], Stroke::new(1.0, color));
            }
            for x in [0.2, 0.4, 0.8] {
                painter.rect_filled(
                    Rect::from_min_max(p(r, x - 0.06, 0.2), p(r, x + 0.06, 0.55)),
                    0.0,
                    color,
                );
            }
        }
        Icon::Playlist => {
            for (row, (a, b)) in [(0.0, 0.55), (0.3, 0.9), (0.1, 0.45)].iter().enumerate() {
                let y = 0.15 + row as f32 * 0.3;
                painter.rect_filled(
                    Rect::from_min_max(p(r, *a, y), p(r, *b, y + 0.2)),
                    1.5,
                    color,
                );
            }
        }
        Icon::ChannelRack => {
            for row in 0..3 {
                for col in 0..4 {
                    let x = 0.02 + col as f32 * 0.25;
                    let y = 0.1 + row as f32 * 0.3;
                    let on = (row + col) % 3 == 0;
                    let cell = Rect::from_min_max(p(r, x, y), p(r, x + 0.2, y + 0.22));
                    if on {
                        painter.rect_filled(cell, 1.0, color);
                    } else {
                        painter.rect_stroke(
                            cell,
                            1.0,
                            Stroke::new(1.0, color),
                            egui::StrokeKind::Inside,
                        );
                    }
                }
            }
        }
        Icon::PianoRoll => {
            painter.rect_filled(
                Rect::from_min_max(p(r, 0.0, 0.0), p(r, 0.22, 1.0)),
                1.0,
                color,
            );
            for (x0, x1, y) in [(0.32, 0.6, 0.15), (0.55, 0.95, 0.45), (0.35, 0.7, 0.75)] {
                painter.rect_filled(
                    Rect::from_min_max(p(r, x0, y), p(r, x1, y + 0.16)),
                    1.5,
                    color,
                );
            }
        }
        Icon::Mixer => {
            for (i, knob) in [0.65, 0.3, 0.5].iter().enumerate() {
                let x = 0.2 + i as f32 * 0.3;
                painter.line_segment([p(r, x, 0.05), p(r, x, 0.95)], Stroke::new(1.2, color));
                painter.rect_filled(
                    Rect::from_center_size(
                        p(r, x, *knob),
                        vec2(r.width() * 0.22, r.height() * 0.14),
                    ),
                    1.0,
                    color,
                );
            }
        }
        Icon::Menu => {
            for y in [0.22, 0.5, 0.78] {
                painter.line_segment([p(r, 0.12, y), p(r, 0.88, y)], stroke);
            }
        }
        Icon::Fold | Icon::Unfold => {
            let (tip, wing) = if icon == Icon::Fold {
                (0.15, 0.5)
            } else {
                (0.5, 0.15)
            };
            painter.add(Shape::line(
                vec![p(r, 0.2, wing), p(r, 0.5, tip), p(r, 0.8, wing)],
                stroke,
            ));
            painter.line_segment([p(r, 0.15, 0.82), p(r, 0.85, 0.82)], stroke);
        }
        Icon::Maximize | Icon::Restore => {
            // Each bracket is drawn at its corner; restore turns it to face
            // the centre.
            let inward = icon == Icon::Restore;
            for (cx, cy) in [(0.0, 0.0), (1.0, 0.0), (0.0, 1.0), (1.0, 1.0)] {
                let (x, y) = if inward {
                    (0.5 + (cx - 0.5) * 0.5, 0.5 + (cy - 0.5) * 0.5)
                } else {
                    (0.05 + cx * 0.9, 0.05 + cy * 0.9)
                };
                let toward = |c: f32| if c < 0.5 { 1.0 } else { -1.0 };
                let (dx, dy) = if inward {
                    (-toward(cx) * 0.25, -toward(cy) * 0.25)
                } else {
                    (toward(cx) * 0.3, toward(cy) * 0.3)
                };
                painter.add(Shape::line(
                    vec![p(r, x + dx, y), p(r, x, y), p(r, x, y + dy)],
                    stroke,
                ));
            }
        }
        Icon::Close => {
            let s = Stroke::new(1.4, color);
            painter.line_segment([p(r, 0.1, 0.1), p(r, 0.9, 0.9)], s);
            painter.line_segment([p(r, 0.9, 0.1), p(r, 0.1, 0.9)], s);
        }
        Icon::More => {
            for x in [0.15, 0.55] {
                painter.add(Shape::line(
                    vec![p(r, x, 0.2), p(r, x + 0.25, 0.5), p(r, x, 0.8)],
                    stroke,
                ));
            }
        }
        Icon::Refresh => {
            let c = r.center();
            let radius = r.width().min(r.height()) * 0.4;
            let at = |turn: f32| {
                let a = turn * std::f32::consts::TAU;
                c + vec2(a.cos(), a.sin()) * radius
            };
            let points: Vec<Pos2> = (0..=18).map(|i| at(-0.2 + i as f32 / 24.0)).collect();
            painter.add(Shape::line(points, stroke));
            let end = at(0.55);
            painter.add(Shape::convex_polygon(
                vec![
                    end + vec2(-radius * 0.45, -radius * 0.1),
                    end + vec2(radius * 0.3, -radius * 0.35),
                    end + vec2(radius * 0.05, radius * 0.4),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::FoldAll => {
            painter.add(Shape::line(
                vec![p(r, 0.2, 0.4), p(r, 0.5, 0.1), p(r, 0.8, 0.4)],
                stroke,
            ));
            painter.line_segment([p(r, 0.15, 0.65), p(r, 0.85, 0.65)], stroke);
            painter.line_segment([p(r, 0.15, 0.9), p(r, 0.85, 0.9)], stroke);
        }
        Icon::ChevronDown => {
            painter.add(Shape::line(
                vec![p(r, 0.15, 0.35), p(r, 0.5, 0.7), p(r, 0.85, 0.35)],
                stroke,
            ));
        }
        Icon::FolderOpen => {
            painter.add(Shape::line(
                vec![
                    p(r, 0.85, 0.3),
                    p(r, 0.5, 0.3),
                    p(r, 0.4, 0.15),
                    p(r, 0.0, 0.15),
                    p(r, 0.0, 0.9),
                ],
                stroke,
            ));
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.2, 0.45),
                    p(r, 1.0, 0.45),
                    p(r, 0.8, 0.9),
                    p(r, 0.0, 0.9),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
        }
        Icon::Wave => {
            painter.line_segment([p(r, 0.0, 0.5), p(r, 1.0, 0.5)], Stroke::new(1.0, color));
            for (x, h) in [
                (0.1, 0.18),
                (0.3, 0.4),
                (0.5, 0.28),
                (0.7, 0.45),
                (0.9, 0.15),
            ] {
                painter.line_segment([p(r, x, 0.5 - h), p(r, x, 0.5 + h)], stroke);
            }
        }
        Icon::Plugins => {
            for (i, (x, y)) in [(0.0, 0.0), (0.55, 0.0), (0.0, 0.55), (0.55, 0.55)]
                .into_iter()
                .enumerate()
            {
                let cell = Rect::from_min_max(p(r, x, y), p(r, x + 0.45, y + 0.45));
                if i == 1 {
                    painter.rect_stroke(cell, 1.5, stroke, egui::StrokeKind::Inside);
                } else {
                    painter.rect_filled(cell, 1.5, color);
                }
            }
        }
        Icon::Knob => {
            let radius = r.width().min(r.height()) * 0.45;
            painter.circle_stroke(r.center(), radius, stroke);
            painter.line_segment([r.center(), p(r, 0.82, 0.18)], stroke);
        }
        Icon::Effect => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.0, 0.0), p(r, 1.0, 1.0)),
                1.5,
                stroke,
                egui::StrokeKind::Inside,
            );
            let curve: Vec<Pos2> = (0..=12)
                .map(|i| {
                    let t = i as f32 / 12.0;
                    let y =
                        0.5 - 0.35 * (t * std::f32::consts::PI - std::f32::consts::FRAC_PI_2).sin();
                    p(r, 0.12 + t * 0.76, y)
                })
                .collect();
            painter.add(Shape::line(curve, stroke));
        }
        Icon::Automation => {
            let nodes = [
                p(r, 0.05, 0.8),
                p(r, 0.35, 0.25),
                p(r, 0.65, 0.65),
                p(r, 0.95, 0.15),
            ];
            painter.add(Shape::line(nodes.to_vec(), stroke));
            for node in nodes {
                painter.circle_filled(node, 1.6, color);
            }
        }
        Icon::Sine => {
            let wave: Vec<Pos2> = (0..=16)
                .map(|i| {
                    let t = i as f32 / 16.0;
                    p(r, t, 0.5 - 0.4 * (t * std::f32::consts::TAU).sin())
                })
                .collect();
            painter.add(Shape::line(wave, stroke));
        }
        Icon::Snap => {
            painter.line_segment([p(r, 0.0, 0.8), p(r, 1.0, 0.8)], stroke);
            for x in [0.1, 0.9] {
                painter.line_segment([p(r, x, 0.55), p(r, x, 1.0)], stroke);
            }
            painter.line_segment([p(r, 0.5, 0.05), p(r, 0.5, 0.45)], stroke);
            painter.circle_filled(p(r, 0.5, 0.62), r.width() * 0.14, color);
        }
        Icon::PlayFrom => {
            painter.line_segment([p(r, 0.08, 0.0), p(r, 0.08, 1.0)], stroke);
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.35, 0.15), p(r, 0.95, 0.5), p(r, 0.35, 0.85)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pattern => {
            for (x0, x1, y) in [(0.0, 0.45, 0.8), (0.3, 0.75, 0.5), (0.55, 1.0, 0.2)] {
                painter.line_segment([p(r, x0, y), p(r, x1, y)], Stroke::new(2.2, color));
            }
        }
        Icon::Plus => {
            let thick = Stroke::new(2.0, color);
            painter.line_segment([p(r, 0.5, 0.1), p(r, 0.5, 0.9)], thick);
            painter.line_segment([p(r, 0.1, 0.5), p(r, 0.9, 0.5)], thick);
        }
        Icon::ChevronRight => {
            painter.add(Shape::line(
                vec![p(r, 0.35, 0.15), p(r, 0.7, 0.5), p(r, 0.35, 0.85)],
                stroke,
            ));
        }
        Icon::Details => {
            for y in [0.15, 0.5, 0.85] {
                painter.circle_filled(p(r, 0.3, y), r.width() * 0.09, color);
                painter.line_segment([p(r, 0.5, y), p(r, 0.95, y)], stroke);
            }
        }
        Icon::SidePanel => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.0, 0.1), p(r, 1.0, 0.9)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 0.62, 0.1), p(r, 1.0, 0.9)),
                1.0,
                color,
            );
        }
        Icon::Phase => {
            painter.circle_stroke(p(r, 0.5, 0.5), r.width() * 0.32, stroke);
            painter.line_segment([p(r, 0.1, 0.9), p(r, 0.9, 0.1)], stroke);
        }
        Icon::Swap => {
            painter.line_segment([p(r, 0.05, 0.3), p(r, 0.95, 0.3)], stroke);
            painter.add(Shape::line(
                vec![p(r, 0.75, 0.1), p(r, 0.95, 0.3), p(r, 0.75, 0.5)],
                stroke,
            ));
            painter.line_segment([p(r, 0.05, 0.7), p(r, 0.95, 0.7)], stroke);
            painter.add(Shape::line(
                vec![p(r, 0.25, 0.5), p(r, 0.05, 0.7), p(r, 0.25, 0.9)],
                stroke,
            ));
        }
        Icon::Pause => {
            for x in [0.18, 0.62] {
                painter.rect_filled(
                    Rect::from_min_max(p(r, x, 0.08), p(r, x + 0.22, 0.92)),
                    1.0,
                    color,
                );
            }
        }
        Icon::Layout => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.05, 0.05), p(r, 0.95, 0.7)),
                1.0,
                stroke,
                egui::StrokeKind::Middle,
            );
            painter.line_segment([p(r, 0.45, 0.05), p(r, 0.45, 0.7)], stroke);
            painter.line_segment([p(r, 0.45, 0.38), p(r, 0.95, 0.38)], stroke);
            painter.add(Shape::line(
                vec![p(r, 0.38, 0.82), p(r, 0.5, 0.95), p(r, 0.62, 0.82)],
                stroke,
            ));
        }
        Icon::Wait => {
            painter.line_segment([p(r, 0.15, 0.03), p(r, 0.85, 0.03)], stroke);
            painter.line_segment([p(r, 0.15, 0.97), p(r, 0.85, 0.97)], stroke);
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.2, 0.1), p(r, 0.8, 0.1), p(r, 0.5, 0.5)],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.5, 0.5), p(r, 0.8, 0.9), p(r, 0.2, 0.9)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::CountIn => {
            for (x, top) in [(0.08, 0.65), (0.25, 0.45), (0.42, 0.25)] {
                painter.line_segment([p(r, x, top), p(r, x, 0.9)], stroke);
            }
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.58, 0.3), p(r, 0.98, 0.6), p(r, 0.58, 0.9)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Blend => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.0, 0.0), p(r, 0.62, 0.62)),
                2.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect_filled(
                Rect::from_min_max(p(r, 0.38, 0.38), p(r, 1.0, 1.0)),
                2.0,
                color,
            );
        }
        Icon::LoopRecord => {
            let c = p(r, 0.5, 0.5);
            let radius = r.width() * 0.42;
            let arc: Vec<Pos2> = (0..=18)
                .map(|i| {
                    let a =
                        -std::f32::consts::FRAC_PI_2 + i as f32 / 18.0 * std::f32::consts::PI * 1.5;
                    c + egui::vec2(a.cos(), a.sin()) * radius
                })
                .collect();
            let end = arc[arc.len() - 1];
            painter.add(Shape::line(arc, stroke));
            painter.add(Shape::convex_polygon(
                vec![
                    end + egui::vec2(-3.0, -3.5),
                    end + egui::vec2(3.5, 0.0),
                    end + egui::vec2(-3.0, 3.5),
                ],
                color,
                Stroke::NONE,
            ));
            painter.circle_filled(c, r.width() * 0.12, color);
        }
        Icon::StepEdit => {
            painter.rect_filled(
                Rect::from_min_max(p(r, 0.0, 0.4), p(r, 0.5, 0.62)),
                1.0,
                color,
            );
            painter.add(Shape::line(
                vec![p(r, 0.62, 0.2), p(r, 0.95, 0.51), p(r, 0.62, 0.82)],
                stroke,
            ));
        }
        Icon::Wand => {
            painter.line_segment([p(r, 0.05, 0.95), p(r, 0.62, 0.38)], stroke);
            let tip = p(r, 0.78, 0.22);
            let arm = r.width() * 0.2;
            painter.line_segment([tip - vec2(arm, 0.0), tip + vec2(arm, 0.0)], stroke);
            painter.line_segment([tip - vec2(0.0, arm), tip + vec2(0.0, arm)], stroke);
        }
        Icon::Scale => {
            painter.add(Shape::line(
                vec![
                    p(r, 0.0, 0.95),
                    p(r, 0.33, 0.95),
                    p(r, 0.33, 0.62),
                    p(r, 0.66, 0.62),
                    p(r, 0.66, 0.3),
                    p(r, 1.0, 0.3),
                ],
                stroke,
            ));
        }
        Icon::Chord => {
            for (x, y) in [(0.3, 0.82), (0.5, 0.5), (0.7, 0.18)] {
                painter.circle_filled(p(r, x, y), r.width() * 0.16, color);
            }
        }
        Icon::PlayNote => {
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.0, 0.15), p(r, 0.6, 0.5), p(r, 0.0, 0.85)],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::line(
                vec![p(r, 0.78, 0.2), p(r, 0.95, 0.5), p(r, 0.78, 0.8)],
                stroke,
            ));
        }
        Icon::ChevronLeft => {
            painter.add(Shape::line(
                vec![p(r, 0.65, 0.15), p(r, 0.3, 0.5), p(r, 0.65, 0.85)],
                stroke,
            ));
        }
        Icon::ChevronUp => {
            painter.add(Shape::line(
                vec![p(r, 0.15, 0.65), p(r, 0.5, 0.3), p(r, 0.85, 0.65)],
                stroke,
            ));
        }
        Icon::Porta => {
            painter.add(Shape::QuadraticBezier(
                egui::epaint::QuadraticBezierShape::from_points_stroke(
                    [p(r, 0.05, 0.95), p(r, 0.05, 0.05), p(r, 0.95, 0.05)],
                    false,
                    Color32::TRANSPARENT,
                    stroke,
                ),
            ));
        }
        Icon::Check => {
            painter.add(Shape::line(
                vec![p(r, 0.1, 0.55), p(r, 0.4, 0.85), p(r, 0.9, 0.15)],
                Stroke::new(1.6, color),
            ));
        }
        Icon::Out => {
            let stroke = Stroke::new(1.3, color);
            painter.circle_stroke(p(r, 0.22, 0.5), r.width() * 0.2, stroke);
            painter.line_segment([p(r, 0.42, 0.5), p(r, 0.95, 0.5)], stroke);
            painter.add(Shape::line(
                vec![p(r, 0.72, 0.28), p(r, 0.95, 0.5), p(r, 0.72, 0.72)],
                stroke,
            ));
        }
        Icon::In => {
            let stroke = Stroke::new(1.3, color);
            painter.circle_stroke(p(r, 0.78, 0.5), r.width() * 0.2, stroke);
            painter.line_segment([p(r, 0.05, 0.5), p(r, 0.56, 0.5)], stroke);
            painter.add(Shape::line(
                vec![p(r, 0.34, 0.28), p(r, 0.56, 0.5), p(r, 0.34, 0.72)],
                stroke,
            ));
        }
        Icon::Envelope => {
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.0, 0.95), p(r, 0.2, 0.1), p(r, 0.4, 0.45)],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.4, 0.95),
                    p(r, 0.4, 0.45),
                    p(r, 0.75, 0.45),
                    p(r, 1.0, 0.95),
                ],
                color,
                Stroke::NONE,
            ));
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.0, 0.95), p(r, 0.4, 0.45), p(r, 0.4, 0.95)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Steps => {
            let side = r.width() * 0.3;
            for (x, y) in [(0.0, 0.7), (0.35, 0.35), (0.7, 0.0)] {
                let min = p(r, x, y);
                painter.rect_filled(Rect::from_min_size(min, vec2(side, side)), 1.0, color);
            }
        }
        Icon::SlideRamp => {
            painter.add(Shape::convex_polygon(
                vec![p(r, 0.0, 0.95), p(r, 0.95, 0.05), p(r, 0.95, 0.95)],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Pack => {
            painter.rect_stroke(
                Rect::from_min_max(p(r, 0.05, 0.15), p(r, 0.95, 0.9)),
                1.0,
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.line_segment([p(r, 0.05, 0.35), p(r, 0.95, 0.35)], stroke);
        }
        Icon::Browser => {
            painter.add(Shape::convex_polygon(
                vec![
                    p(r, 0.0, 0.15),
                    p(r, 0.4, 0.15),
                    p(r, 0.5, 0.3),
                    p(r, 1.0, 0.3),
                    p(r, 1.0, 0.9),
                    p(r, 0.0, 0.9),
                ],
                Color32::TRANSPARENT,
                stroke,
            ));
        }
    }
}

/// A square toolbar button showing an icon; `active` lights it in `accent`.
/// At rest the glyph is plain `TEXT`, except record, which keeps its
/// `DANGER` glyph so it is always recognisable.
/// A square button that opens a list: `open` draws it pressed, `unset`
/// dims its glyph (nothing chosen) and `lit` draws the glyph in the
/// highlight tone (a choice that changes what clicks do).
/// An inert glyph of `size` points, in line with text (for example the
/// output mark before a routing summary).
pub fn glyph(ui: &mut Ui, icon: Icon, size: f32, color: Color32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint(ui.painter(), rect, icon, color);
    response
}

pub fn picker(
    ui: &mut Ui,
    icon: Icon,
    open: bool,
    (unset, lit): (bool, bool),
    hint: &str,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 22.0), Sense::click());
    let fill = if open || response.is_pointer_button_down_on() {
        theme::ACCENT_DIM
    } else if response.hovered() {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    let color = if lit {
        theme::HIGHLIGHT
    } else if unset {
        theme::TEXT_FAINT
    } else {
        theme::TEXT
    };
    paint(ui.painter(), rect, icon, color);
    response.hint(hint)
}

pub fn button(ui: &mut Ui, icon: Icon, active: bool, accent: Color32, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 22.0), Sense::click());
    let hovered = response.hovered();
    let fill = if active {
        accent.gamma_multiply(0.85)
    } else if response.is_pointer_button_down_on() {
        theme::ACCENT_DIM
    } else if hovered {
        theme::RAISED_HOVER
    } else {
        theme::RAISED
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    let color = if !ui.is_enabled() {
        theme::TEXT_FAINT
    } else if active {
        theme::BACKGROUND
    } else if icon == Icon::Record {
        theme::DANGER
    } else {
        theme::TEXT
    };
    paint(ui.painter(), rect, icon, color);
    response.hint(hint)
}

/// A quieter toggle for showing and hiding windows: an underline marks it on.
pub fn toggle(ui: &mut Ui, icon: Icon, on: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(28.0, 24.0), Sense::click());
    let hovered = response.hovered();
    if hovered || on {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(4),
            if hovered {
                theme::RAISED_HOVER
            } else {
                theme::RAISED
            },
        );
    }
    let color = if !ui.is_enabled() {
        theme::TEXT_FAINT
    } else if on {
        theme::ACCENT
    } else {
        theme::TEXT_DIM
    };
    paint(ui.painter(), rect.shrink(1.0), icon, color);
    if on {
        let y = rect.bottom() - 1.5;
        ui.painter().line_segment(
            [pos2(rect.left() + 6.0, y), pos2(rect.right() - 6.0, y)],
            Stroke::new(2.0, theme::ACCENT),
        );
    }
    response.hint(hint)
}

/// A small square header button (window menu, collapse, maximize, close):
/// no fill and a dim glyph at rest, a raised square and a bright glyph under
/// the pointer. `pressed` keeps it pressed (its menu is open); `danger`
/// turns the hovered glyph red (close).
pub fn header_button(ui: &mut Ui, icon: Icon, pressed: bool, danger: bool, hint: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
    let hovered = response.hovered();
    if pressed || hovered {
        let fill = if response.is_pointer_button_down_on() {
            theme::ACCENT_DIM
        } else if hovered {
            theme::RAISED_HOVER
        } else {
            theme::RAISED
        };
        ui.painter().rect_filled(rect, CornerRadius::same(2), fill);
    }
    let color = if !ui.is_enabled() {
        theme::TEXT_FAINT
    } else if hovered && danger {
        theme::DANGER
    } else if pressed {
        theme::ACCENT
    } else if hovered {
        theme::TEXT
    } else {
        theme::TEXT_DIM
    };
    paint(ui.painter(), rect.shrink(1.5), icon, color);
    response.hint(hint)
}

/// Height of a toolbar row: one button.
pub const ROW: f32 = 22.0;

/// A one-row display well: `big` digits, then `small` ones in a smaller,
/// dimmer size, and a tiny `caption` in the top-right corner.
pub fn display(
    ui: &mut Ui,
    width: f32,
    parts: (&str, &str),
    caption: &str,
    sense: Sense,
) -> Response {
    display_split(ui, width, parts, caption, sense).0
}

/// [`display`], also returning the x where the small part begins.
pub fn display_split(
    ui: &mut Ui,
    width: f32,
    (big, small): (&str, &str),
    caption: &str,
    sense: Sense,
) -> (Response, f32) {
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW), sense);
    let painter = ui.painter_at(rect);
    well(&painter, rect);
    let big = painter.layout_no_wrap(
        big.to_owned(),
        egui::FontId::monospace(theme::FONT_DISPLAY - 1.0),
        theme::LCD_TEXT,
    );
    let small = painter.layout_no_wrap(
        small.to_owned(),
        egui::FontId::monospace(theme::FONT_VALUE),
        theme::TEXT,
    );
    let x = rect.left() + 5.0;
    let base = rect.bottom() - 3.0;
    let big_w = big.size().x;
    let split = x + big_w;
    painter.galley(pos2(x, base - big.size().y), big, theme::LCD_TEXT);
    painter.galley(
        pos2(x + big_w, base - small.size().y - 1.0),
        small,
        theme::TEXT,
    );
    painter.text(
        pos2(rect.right() - 3.0, rect.top() + 1.0),
        egui::Align2::RIGHT_TOP,
        caption,
        egui::FontId::proportional(theme::FONT_CAPTION - 1.5),
        theme::TEXT_DIM,
    );
    (response, split)
}

/// A recessed display well.
fn well(painter: &egui::Painter, rect: Rect) {
    painter.rect_filled(rect, CornerRadius::same(3), theme::LCD_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(3),
        Stroke::new(1.0, theme::LCD_EDGE),
        egui::StrokeKind::Inside,
    );
}

/// The pattern/song switch, one button wide with its two labels stacked.
/// Returns the response and which half (0 pattern, 1 song) the pointer is
/// over.
pub fn mode_switch(ui: &mut Ui, song: bool, hint: &str) -> (Response, Option<usize>) {
    let (rect, response) = ui.allocate_exact_size(vec2(34.0, ROW), Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(3), theme::PANEL_DARK);
    let half = rect.height() / 2.0;
    let pointer = response.hover_pos();
    let mut over = None;
    for (i, label) in ["Ptn", "Song"].into_iter().enumerate() {
        let part = Rect::from_min_size(
            pos2(rect.left(), rect.top() + half * i as f32),
            vec2(rect.width(), half),
        );
        let lit = (i == 1) == song;
        let hovered = pointer.is_some_and(|p| part.contains(p));
        if hovered {
            over = Some(i);
        }
        if lit {
            painter.rect_filled(part.shrink(1.0), CornerRadius::same(2), theme::HIGHLIGHT);
        }
        let color = if lit {
            theme::BACKGROUND
        } else if hovered {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        };
        painter.text(
            part.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(theme::FONT_CAPTION + 0.5),
            color,
        );
    }
    (response.hint(hint), over)
}

/// The output monitor: a scope beside two horizontal peak bars (left on
/// top), in one well. Silent output shows only the scope's centre line.
pub fn output_monitor(
    ui: &mut Ui,
    samples: &[f32],
    (left, right): (f32, f32),
    width: f32,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, ROW), Sense::click());
    let painter = ui.painter_at(rect);
    well(&painter, rect);
    let scope = Rect::from_min_max(
        rect.min + vec2(3.0, 2.0),
        pos2(rect.center().x - 2.0, rect.bottom() - 2.0),
    );
    painter.line_segment(
        [scope.left_center(), scope.right_center()],
        Stroke::new(1.0, theme::LCD_EDGE),
    );
    if samples.len() > 1 {
        let step = scope.width() / (samples.len() - 1) as f32;
        let points: Vec<Pos2> = samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                pos2(
                    scope.left() + i as f32 * step,
                    scope.center().y - s.clamp(-1.0, 1.0) * scope.height() * 0.48,
                )
            })
            .collect();
        painter.add(Shape::line(points, Stroke::new(1.0, theme::ACCENT)));
    }
    let bars = Rect::from_min_max(
        pos2(rect.center().x + 2.0, rect.top() + 5.0),
        pos2(rect.right() - 4.0, rect.bottom() - 5.0),
    );
    let h = (bars.height() - 2.0) / 2.0;
    for (i, level) in [left, right].into_iter().enumerate() {
        let bar = Rect::from_min_size(
            pos2(bars.left(), bars.top() + (h + 2.0) * i as f32),
            vec2(bars.width(), h),
        );
        painter.rect_filled(bar, 1.0, theme::METER_BG);
        let fill = level.clamp(0.0, 1.0);
        let color = if level >= 1.0 {
            theme::DANGER
        } else if fill > 0.7 {
            theme::HIGHLIGHT
        } else {
            theme::ACCENT
        };
        painter.rect_filled(
            Rect::from_min_max(bar.min, pos2(bar.left() + bar.width() * fill, bar.bottom())),
            1.0,
            color,
        );
        // The held peak as a thin line, and a clip mark at the end that
        // stays for a few seconds after the level reached full scale.
        let (peak, clipped) =
            musiccraft_editors::widgets::held_peak(ui, response.id.with(("peak", i)), level);
        let x = bar.left() + bar.width() * peak.clamp(0.0, 1.0);
        if peak > 0.001 {
            painter.line_segment(
                [pos2(x, bar.top()), pos2(x, bar.bottom())],
                Stroke::new(1.0, theme::TEXT),
            );
        }
        if clipped {
            painter.rect_filled(
                Rect::from_min_max(pos2(bar.right() - 3.0, bar.top()), bar.max),
                0.0,
                theme::DANGER,
            );
        }
    }
    response
}

/// The performance panel: a CPU bar and figure over the voice count.
pub fn performance(ui: &mut Ui, load: f32, voices: u32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(84.0, ROW), Sense::hover());
    let painter = ui.painter_at(rect);
    well(&painter, rect);
    let bar = Rect::from_min_max(
        pos2(rect.left() + 4.0, rect.top() + 5.0),
        pos2(rect.left() + 34.0, rect.top() + 9.0),
    );
    painter.rect_filled(bar, 1.0, theme::PANEL);
    let fill = load.clamp(0.0, 1.0);
    let color = if fill > 0.8 {
        theme::DANGER
    } else if fill > 0.5 {
        theme::HIGHLIGHT
    } else {
        theme::ACCENT
    };
    painter.rect_filled(
        Rect::from_min_max(bar.min, pos2(bar.left() + bar.width() * fill, bar.bottom())),
        1.0,
        color,
    );
    let font = egui::FontId::monospace(theme::FONT_CAPTION + 0.5);
    painter.text(
        pos2(rect.right() - 4.0, rect.top() + 7.0),
        egui::Align2::RIGHT_CENTER,
        format!("{:.0}% CPU", load * 100.0),
        font.clone(),
        theme::TEXT,
    );
    painter.text(
        pos2(rect.right() - 4.0, rect.bottom() - 6.0),
        egui::Align2::RIGHT_CENTER,
        format!("{voices} VOICES"),
        font,
        theme::TEXT_DIM,
    );
    response
}
