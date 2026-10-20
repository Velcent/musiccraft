//! The hint bar: hovering a control shows what it does in the toolbar, the
//! way pattern-based DAWs teach their interface. With the hint bar hidden the
//! same text appears as a tooltip.

use egui::{Id, Response};

fn text_id() -> Id {
    Id::new("musiccraft-hint")
}

fn bar_id() -> Id {
    Id::new("musiccraft-hint-bar")
}

/// Adds hint text to a widget.
pub trait Hint {
    /// Shows `text` in the hint bar (or a tooltip) while hovered.
    fn hint(self, text: impl Into<String>) -> Self;
}

impl Hint for Response {
    fn hint(self, text: impl Into<String>) -> Self {
        let text = text.into();
        let in_bar = self
            .ctx
            .data(|d| d.get_temp::<bool>(bar_id()))
            .unwrap_or(false);
        if self.hovered() {
            self.ctx
                .data_mut(|d| d.insert_temp(text_id(), text.clone()));
        }
        if in_bar {
            self
        } else {
            self.on_hover_text(text)
        }
    }
}

/// Starts a frame: returns the hint hovered during the previous frame and
/// clears it, and records whether hints go to the bar.
pub fn begin_frame(ctx: &egui::Context, in_bar: bool) -> Option<String> {
    ctx.data_mut(|d| {
        d.insert_temp(bar_id(), in_bar);
        d.remove_temp::<String>(text_id())
    })
}
