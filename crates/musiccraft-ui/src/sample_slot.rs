//! Sample slots (`docs/spec/style/dialogs.md`, "Sample slots"): the sampler's
//! sample, drum pads, zones and Antiserum's oscillators take a sound from the
//! browser (its menu, or a drag onto the slot) or from the system's file
//! dialog ("Browse files…").

use musiccraft_commands::Invocation;
use musiccraft_editors::sounds::{self, SampleChoice, Sounds};

use crate::dialogs::FilePick;
use crate::{Action, UiState};

/// The sound being dragged from the browser, if any.
pub(crate) fn dragged(ctx: &egui::Context) -> Option<String> {
    egui::DragAndDrop::payload::<crate::browser::Dragged>(ctx)
        .and_then(|d| d.sample().map(str::to_owned))
}

/// A small button that opens the slot's menu.
pub(crate) fn button(
    ui: &mut egui::Ui,
    state: &mut UiState,
    text: &str,
    hint: &str,
    clear: bool,
) -> Option<SampleChoice> {
    let browser = &mut state.browser;
    let mut sounds = Sounds {
        folders: &browser.folders,
        listings: &mut browser.listings,
    };
    sounds::button(ui, text, hint, &mut sounds, clear)
}

/// The slot's menu items, inside a menu the caller opened.
pub(crate) fn menu_items(
    ui: &mut egui::Ui,
    state: &mut UiState,
    clear: bool,
) -> Option<SampleChoice> {
    let browser = &mut state.browser;
    let mut sounds = Sounds {
        folders: &browser.folders,
        listings: &mut browser.listings,
    };
    sounds::menu_items(ui, &mut sounds, clear)
}

/// A sound dropped on `rect` from the browser.
pub(crate) fn dropped(ui: &egui::Ui, rect: egui::Rect) -> Option<String> {
    let sound = sounds::drop_target(ui, rect, dragged(ui.ctx()).as_deref())?;
    egui::DragAndDrop::clear_payload(ui.ctx());
    Some(sound)
}

/// Carries out a choice: "Browse files…" asks for `pick`; a sound runs
/// `set` with it, and "Clear" runs it with `-`.
pub(crate) fn apply(
    state: &mut UiState,
    choice: Option<SampleChoice>,
    pick: impl FnOnce() -> FilePick,
    set: impl FnOnce(String) -> Invocation,
    actions: &mut Vec<Action>,
) {
    match choice {
        Some(SampleChoice::Browse) => state.pick_file(pick()),
        Some(SampleChoice::Use(sound)) => actions.push(Action::Run(set(sound))),
        Some(SampleChoice::Clear) => actions.push(Action::Run(set("-".to_owned()))),
        None => {}
    }
}
