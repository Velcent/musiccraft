//! Icons drawn with vector shapes (our own artwork; no icon fonts), plus the
//! toolbar's button and display widgets built on them.

use crate::hint::Hint;
use egui::{
    Color32, CornerRadius, Pos2, Rect, Response, RichText, Sense, Shape, Stroke, Ui, pos2, vec2,
};

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
    let color = if active {
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
    let color = if on { theme::ACCENT } else { theme::TEXT_DIM };
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
    let color = if hovered && danger {
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

/// A dark "LCD" panel showing a large value and a small caption underneath.
pub fn lcd(ui: &mut Ui, width: f32, main: &str, caption: &str, sense: Sense) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 32.0), sense);
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::LCD_BG);
    painter.rect_stroke(
        rect,
        CornerRadius::same(4),
        Stroke::new(1.0, theme::LCD_EDGE),
        egui::StrokeKind::Inside,
    );
    painter.text(
        pos2(rect.center().x, rect.top() + 13.0),
        egui::Align2::CENTER_CENTER,
        main,
        egui::FontId::monospace(theme::FONT_DISPLAY),
        theme::LCD_TEXT,
    );
    painter.text(
        pos2(rect.center().x, rect.bottom() - 6.0),
        egui::Align2::CENTER_CENTER,
        caption,
        egui::FontId::proportional(theme::FONT_CAPTION),
        theme::TEXT_DIM,
    );
    response
}

/// A two-way switch such as PAT | SONG. Returns the index clicked, if any.
pub fn segmented(ui: &mut Ui, labels: &[&str], selected: usize, hint: &str) -> Option<usize> {
    let mut clicked = None;
    let frame = egui::Frame::new()
        .fill(theme::PANEL_DARK)
        .corner_radius(4.0)
        .inner_margin(2.0);
    frame.show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.horizontal(|ui| {
            for (i, label) in labels.iter().enumerate() {
                let on = i == selected;
                let text = RichText::new(*label).strong().size(11.0).color(if on {
                    theme::BACKGROUND
                } else {
                    theme::TEXT_DIM
                });
                let button = egui::Button::new(text)
                    .fill(if on {
                        theme::HIGHLIGHT
                    } else {
                        Color32::TRANSPARENT
                    })
                    .min_size(vec2(40.0, 22.0));
                if ui.add(button).hint(hint).clicked() {
                    clicked = Some(i);
                }
            }
        });
    });
    clicked
}

/// An oscilloscope of recent output samples.
pub fn scope(ui: &mut Ui, samples: &[f32], size: egui::Vec2) -> Response {
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::LCD_BG);
    painter.line_segment(
        [rect.left_center(), rect.right_center()],
        Stroke::new(1.0, theme::LCD_EDGE),
    );
    if samples.len() > 1 {
        let step = rect.width() / (samples.len() - 1) as f32;
        let points: Vec<Pos2> = samples
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let y = rect.center().y - s.clamp(-1.0, 1.0) * rect.height() * 0.45;
                pos2(rect.left() + i as f32 * step, y)
            })
            .collect();
        painter.add(Shape::line(points, Stroke::new(1.2, theme::ACCENT)));
    }
    response
}

/// A small horizontal bar for processing load.
pub fn load_bar(ui: &mut Ui, load: f32) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(44.0, 32.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, CornerRadius::same(4), theme::LCD_BG);
    let bar = Rect::from_min_max(
        pos2(rect.left() + 4.0, rect.top() + 18.0),
        pos2(rect.right() - 4.0, rect.top() + 23.0),
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
        Rect::from_min_max(
            bar.left_top(),
            pos2(bar.left() + bar.width() * fill, bar.bottom()),
        ),
        1.0,
        color,
    );
    painter.text(
        pos2(rect.center().x, rect.top() + 9.0),
        egui::Align2::CENTER_CENTER,
        format!("{:.0}%", load * 100.0),
        egui::FontId::monospace(10.0),
        theme::TEXT,
    );
    painter.text(
        pos2(rect.center().x, rect.bottom() - 4.0),
        egui::Align2::CENTER_CENTER,
        "CPU",
        egui::FontId::proportional(8.0),
        theme::TEXT_DIM,
    );
    response.hint("Share of each audio buffer's time spent processing")
}
