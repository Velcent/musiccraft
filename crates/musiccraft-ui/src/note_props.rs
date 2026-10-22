//! The note properties window: knobs for one note's properties, or for every
//! selected note, opened by double-clicking a note in the piano roll.

use egui::{Color32, RichText, Sense, pos2, vec2};
use musiccraft_commands::Invocation;
use musiccraft_project::music::key_name;
use musiccraft_project::params::TICKS_PER_STEP;
use musiccraft_project::{DEFAULT_VELOCITY, Note, NoteProps};

use crate::hint::Hint;
use crate::piano_roll::LaneMode;
use crate::{Action, Frame, UiState, theme, widgets};

/// The knobs, in window order.
const KNOBS: [LaneMode; 6] = [
    LaneMode::Velocity,
    LaneMode::Release,
    LaneMode::Pan,
    LaneMode::Fine,
    LaneMode::ModX,
    LaneMode::ModY,
];

/// The notes the window edits: every selected note when the clicked one is
/// selected, or only the clicked one; none once it is gone.
fn targets<'a>(state: &UiState, notes: &[&'a Note], id: (u32, u8)) -> Vec<&'a Note> {
    let selection = &state.roll.selection;
    if selection.contains(&id) && selection.len() > 1 {
        notes
            .iter()
            .copied()
            .filter(|n| selection.contains(&(n.start, n.key)))
            .collect()
    } else {
        notes
            .iter()
            .copied()
            .filter(|n| (n.start, n.key) == id)
            .collect()
    }
}

/// Shows the window while a note is picked.
pub(crate) fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let Some(id) = state.roll.props_note else {
        return;
    };
    let project = frame.project;
    let channel = state.selected_channel;
    let Some(channel_id) = project.channels.get(channel).map(|c| c.id) else {
        state.roll.props_note = None;
        return;
    };
    let notes: Vec<&Note> = project.current_pattern().notes_of(channel_id).collect();
    let targets = targets(state, &notes, id);
    let Some(&first) = targets.iter().find(|n| (n.start, n.key) == id) else {
        state.roll.props_note = None;
        return;
    };
    let title = if targets.len() > 1 {
        format!("Note properties — {} notes", targets.len())
    } else {
        format!("Note properties — {}", key_name(first.key))
    };
    let mut open = true;
    let window_id = egui::Id::new("note-properties");
    let default = egui::Rect::from_min_size(pos2(360.0, 160.0), vec2(360.0, 170.0));
    crate::chrome::window(ctx, window_id, &title, state.workspace, default, false)
        .default_height(default.height())
        .show(ctx, |ui| {
            let folded = crate::chrome::Header::new(window_id, &title).show(
                ui,
                &mut open,
                &mut (),
                |_, _| {},
                |_, _| {},
            );
            if !folded {
                let own = crate::channel_color(project, channel);
                body(
                    ui,
                    state,
                    (first, &targets),
                    (channel, own),
                    actions,
                    &mut open,
                );
            }
        });
    if !open {
        state.roll.props_note = None;
    }
}

fn body(
    ui: &mut egui::Ui,
    state: &mut UiState,
    (first, targets): (&Note, &[&Note]),
    (channel, own): (usize, Color32),
    actions: &mut Vec<Action>,
    open: &mut bool,
) {
    ui.horizontal(|ui| {
        for mode in KNOBS {
            let t = mode.value(first);
            let default = mode.value(&neutral(first));
            let knob = widgets::labeled_knob(
                ui,
                mode.label(),
                t,
                default,
                mode.centred(),
                &mode.shown(first),
            );
            if knob.started {
                state.begin_gesture();
            }
            if let Some(t) = knob.changed {
                let lines: Vec<Invocation> = targets
                    .iter()
                    .filter_map(|n| mode.invocation(channel, n, t))
                    .collect();
                if let Some(batch) = batch(lines) {
                    actions.push(Action::Coalesced(batch, state.gesture()));
                }
            }
        }
    });
    ui.horizontal(|ui| {
        let ch = (channel + 1).to_string();
        let each = |name: &str, prop: Option<&str>, on: bool| -> Option<Invocation> {
            let value = if on { "on" } else { "off" };
            batch(
                targets
                    .iter()
                    .map(|n| {
                        let mut args = vec![ch.clone(), format!("{}t", n.start), n.key.to_string()];
                        args.extend(prop.map(str::to_owned));
                        args.push(value.to_owned());
                        Invocation::new(name, args)
                    })
                    .collect(),
            )
        };
        let mut slide = first.slide;
        if widgets::led_switch(ui, &mut slide, "Slide")
            .hint("Glide from the sounding note instead of starting a new one")
            .clicked()
        {
            actions.extend(each("note.set_slide", None, slide).map(Action::Run));
        }
        let mut porta = first.props.porta;
        if widgets::led_switch(ui, &mut porta, "Porta")
            .hint("Glide from the previous note, starting it again")
            .clicked()
        {
            actions.extend(each("note.set_prop", Some("porta"), porta).map(Action::Run));
        }
        ui.add_space(8.0);
        ui.label(RichText::new("Colour").color(theme::TEXT_DIM));
        let group = first.props.color;
        let (rect, swatch) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::click());
        ui.painter()
            .rect_filled(rect, 2.0, crate::piano_roll::group_color(group, own));
        let swatch = swatch.hint(format!("Colour group {} · click to change", group + 1));
        egui::Popup::menu(&swatch).show(|ui| {
            if let Some(picked) = crate::piano_roll::colour_grid(ui, group, own) {
                let lines = targets
                    .iter()
                    .filter(|n| n.props.color != picked)
                    .map(|n| {
                        Invocation::new(
                            "note.set_prop",
                            [
                                ch.clone(),
                                format!("{}t", n.start),
                                n.key.to_string(),
                                "color".to_owned(),
                                picked.to_string(),
                            ],
                        )
                    })
                    .collect();
                actions.extend(batch(lines).map(Action::Run));
                ui.close();
            }
        });
    });
    if targets.len() == 1 {
        let meter = &state.roll.meter;
        let start = crate::geometry::position_label(f64::from(first.start), meter);
        let steps = f64::from(first.length) / f64::from(TICKS_PER_STEP);
        ui.label(
            RichText::new(format!("Start {start} · length {steps:.2} steps"))
                .color(theme::TEXT_DIM),
        );
    }
    ui.separator();
    ui.horizontal(|ui| {
        if ui
            .button("Reset")
            .hint("Every property back to its default")
            .clicked()
        {
            actions.extend(reset(channel, targets).map(Action::Run));
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("OK").clicked() {
                *open = false;
            }
        });
    });
}

/// `note` with every property at its default.
fn neutral(note: &Note) -> Note {
    Note {
        velocity: DEFAULT_VELOCITY,
        pan: 0,
        slide: false,
        props: NoteProps::default(),
        ..*note
    }
}

/// The commands that put every property of `notes` back to its default, as
/// one batch.
fn reset(channel: usize, notes: &[&Note]) -> Option<Invocation> {
    let mut lines = Vec::new();
    for n in notes {
        let plain = neutral(n);
        for mode in KNOBS {
            lines.extend(mode.invocation(channel, n, mode.value(&plain)));
        }
        let at = |name: &str, rest: &[&str]| {
            let mut args = vec![
                (channel + 1).to_string(),
                format!("{}t", n.start),
                n.key.to_string(),
            ];
            args.extend(rest.iter().map(|s| (*s).to_owned()));
            Invocation::new(name, args)
        };
        if n.slide {
            lines.push(at("note.set_slide", &["off"]));
        }
        if n.props.porta {
            lines.push(at("note.set_prop", &["porta", "off"]));
        }
        if n.props.color != 0 {
            lines.push(at("note.set_prop", &["color", "0"]));
        }
    }
    batch(lines)
}

fn batch(lines: Vec<Invocation>) -> Option<Invocation> {
    match lines.len() {
        0 => None,
        1 => lines.into_iter().next(),
        _ => Some(Invocation::new(
            musiccraft_commands::BATCH,
            lines.iter().map(ToString::to_string),
        )),
    }
}
