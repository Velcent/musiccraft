//! Small modal prompts (file paths and names) and the shortcut reference.

use std::path::Path;

use egui::{Key, RichText};
use musiccraft_commands::Invocation;
use musiccraft_project::{FILE_EXTENSION, Project};

use crate::{Action, Frame, UiState, theme};

/// What a prompt is for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Purpose {
    Open,
    SaveAs,
    Export,
    AddSample,
    SetSample(usize),
    RenameChannel(usize),
    RenamePattern(usize),
    RenameTrack(usize),
}

/// An open prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dialog {
    purpose: Purpose,
    text: String,
    bits: &'static str,
    focused: bool,
}

fn project_dir(project: &Project) -> String {
    project
        .path()
        .and_then(Path::parent)
        .map(|p| format!("{}/", p.display()))
        .unwrap_or_default()
}

impl Dialog {
    fn new(purpose: Purpose, text: String) -> Self {
        Self {
            purpose,
            text,
            bits: "16",
            focused: false,
        }
    }

    pub fn open(project: &Project) -> Self {
        Self::new(Purpose::Open, project_dir(project))
    }

    pub fn save_as(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::SaveAs,
            format!("{}{name}.{FILE_EXTENSION}", project_dir(project)),
        )
    }

    pub fn export(project: &Project) -> Self {
        let name = project.name().replace(['/', '\\'], "-");
        Self::new(
            Purpose::Export,
            format!("{}{name}.wav", project_dir(project)),
        )
    }

    pub fn add_sample(project: &Project) -> Self {
        Self::new(Purpose::AddSample, project_dir(project))
    }

    pub fn set_sample(channel: usize, current: String) -> Self {
        Self::new(Purpose::SetSample(channel), current)
    }

    pub fn rename_channel(channel: usize, current: &str) -> Self {
        Self::new(Purpose::RenameChannel(channel), current.to_owned())
    }

    pub fn rename_pattern(pattern: usize, current: &str) -> Self {
        Self::new(Purpose::RenamePattern(pattern), current.to_owned())
    }

    pub fn rename_track(track: usize, current: &str) -> Self {
        Self::new(Purpose::RenameTrack(track), current.to_owned())
    }

    fn title(&self) -> &'static str {
        match self.purpose {
            Purpose::Open => "Open project",
            Purpose::SaveAs => "Save project as",
            Purpose::Export => "Export WAV",
            Purpose::AddSample => "Add WAV sample",
            Purpose::SetSample(_) => "Change sample",
            Purpose::RenameChannel(_) => "Rename channel",
            Purpose::RenamePattern(_) => "Rename pattern",
            Purpose::RenameTrack(_) => "Rename mixer track",
        }
    }

    /// The invocation to run when the prompt is confirmed.
    fn invocation(&self) -> Invocation {
        let text = self.text.trim().to_owned();
        match self.purpose {
            Purpose::Open => Invocation::new("project.load", [text]),
            Purpose::SaveAs => Invocation::new("project.save", [text]),
            Purpose::Export => Invocation::new("project.export_wav", [text, self.bits.to_owned()]),
            Purpose::AddSample => Invocation::new("channel.add_sampler", [text]),
            Purpose::SetSample(channel) => {
                Invocation::new("channel.set_sample", [(channel + 1).to_string(), text])
            }
            Purpose::RenameChannel(channel) => {
                Invocation::new("channel.rename", [(channel + 1).to_string(), text])
            }
            Purpose::RenamePattern(pattern) => {
                Invocation::new("pattern.rename", [(pattern + 1).to_string(), text])
            }
            Purpose::RenameTrack(track) => {
                Invocation::new("mixer.rename", [track.to_string(), text])
            }
        }
    }
}

impl UiState {
    /// Shows a prompt, replacing any open one.
    pub(crate) fn open_dialog(&mut self, dialog: Dialog) {
        self.dialog = Some(dialog);
    }
}

pub fn show(
    ctx: &egui::Context,
    state: &mut UiState,
    _frame: &Frame<'_>,
    actions: &mut Vec<Action>,
) {
    let Some(dialog) = &mut state.dialog else {
        return;
    };
    let mut close = false;
    let mut confirm = false;
    egui::Window::new(dialog.title())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -80.0])
        .show(ctx, |ui| {
            ui.set_min_width(420.0);
            let field = ui.add(egui::TextEdit::singleline(&mut dialog.text).desired_width(400.0));
            if !dialog.focused {
                field.request_focus();
                dialog.focused = true;
            }
            if field.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                confirm = true;
            }
            if dialog.purpose == Purpose::Export {
                ui.horizontal(|ui| {
                    ui.label("Format:");
                    for (bits, label) in
                        [("16", "16-bit"), ("24", "24-bit"), ("32f", "32-bit float")]
                    {
                        ui.selectable_value(&mut dialog.bits, bits, label);
                    }
                });
                ui.label(
                    RichText::new("Renders the pattern or song, following the play mode.")
                        .small()
                        .color(theme::TEXT_DIM),
                );
            }
            ui.horizontal(|ui| {
                if ui.button("OK").clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() || ui.input(|i| i.key_pressed(Key::Escape)) {
                    close = true;
                }
            });
        });
    if confirm && !dialog.text.trim().is_empty() {
        actions.push(Action::Run(dialog.invocation()));
        close = true;
    }
    if close {
        state.dialog = None;
    }
}

pub fn shortcuts(ctx: &egui::Context, state: &mut UiState) {
    let mut open = state.windows.shortcuts;
    egui::Window::new("Keyboard shortcuts")
        .open(&mut open)
        .default_width(320.0)
        .show(ctx, |ui| {
            egui::Grid::new("shortcut-grid")
                .striped(true)
                .show(ui, |ui| {
                    for (chord, invocation) in state.keymap.iter() {
                        ui.label(
                            RichText::new(chord.to_string())
                                .monospace()
                                .color(theme::ACCENT),
                        );
                        ui.label(invocation.to_string());
                        ui.end_row();
                    }
                });
        });
    state.windows.shortcuts = open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_build_quoted_invocations() {
        let mut dialog = Dialog::rename_channel(2, "Kick");
        dialog.text = "Big Kick".into();
        assert_eq!(
            dialog.invocation().to_string(),
            "channel.rename 3 \"Big Kick\""
        );
        let mut export = Dialog::export(&Project::default());
        export.bits = "24";
        assert_eq!(
            export.invocation().to_string(),
            "project.export_wav Untitled.wav 24"
        );
        assert_eq!(
            Dialog::save_as(&Project::default())
                .invocation()
                .to_string(),
            "project.save Untitled.mcproj"
        );
    }
}
