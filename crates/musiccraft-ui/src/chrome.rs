//! The shared frame and header row of floating windows (see
//! `docs/spec/window-layout.md`, "Window header"): the window menu at the
//! left, the window's own tools, the title, then collapse, maximize and close
//! at the right.

use crate::hint::Hint;
use crate::icons::{self, Icon};
use egui::{Rect, Sense, vec2};

use crate::theme;

/// Height of the header row.
const ROW_HEIGHT: f32 = 20.0;
/// The narrowest a title is squeezed to before the tools fold away.
const TITLE_MIN: f32 = 72.0;

fn key(id: egui::Id, what: &str) -> egui::Id {
    id.with(("chrome", what))
}

/// Whether the window `id` is collapsed to its header.
#[must_use]
pub fn collapsed(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.data_mut(|d| d.get_persisted::<bool>(id.with("collapsed")))
        .unwrap_or(false)
}

/// Whether the window `id` fills the workspace.
#[must_use]
pub fn maximized(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.data_mut(|d| d.get_persisted::<Rect>(key(id, "restore-to")))
        .is_some()
}

/// Whether the window `id` is the focused one: the topmost floating window.
#[must_use]
pub fn focused(ctx: &egui::Context, id: egui::Id) -> bool {
    ctx.memory(|m| m.areas().top_layer_id(egui::Order::Middle))
        .is_some_and(|layer| layer.id == id)
}

fn set_maximized(ctx: &egui::Context, id: egui::Id, maximize: bool) {
    let last = ctx.data_mut(|d| d.get_temp::<Rect>(key(id, "rect")));
    ctx.data_mut(|d| {
        if maximize {
            if let Some(rect) = last {
                d.insert_persisted(key(id, "restore-to"), rect);
            }
        } else if let Some(rect) = d.get_persisted::<Rect>(key(id, "restore-to")) {
            // Two frames: the first still has the maximized size, which the
            // workspace bounds would push back into the corner.
            d.insert_temp(key(id, "restore-now"), (rect, 2_u8));
            d.remove::<Rect>(key(id, "restore-to"));
        }
    });
}

/// Maximizes window `id`, or restores it when it is maximized. Returns
/// false, changing nothing, when the window has not been drawn yet (it has
/// no size to restore to).
pub fn toggle_maximized(ctx: &egui::Context, id: egui::Id) -> bool {
    let drawn = ctx.data_mut(|d| d.get_temp::<Rect>(key(id, "rect")).is_some());
    if drawn {
        set_maximized(ctx, id, !maximized(ctx, id));
    }
    drawn
}

/// Maximizes window `id`, leaving it as it is when it already is. Returns
/// false, changing nothing, when the window has not been drawn yet.
pub fn maximize(ctx: &egui::Context, id: egui::Id) -> bool {
    let drawn = ctx.data_mut(|d| d.get_temp::<Rect>(key(id, "rect")).is_some());
    if drawn && !maximized(ctx, id) {
        set_maximized(ctx, id, true);
    }
    drawn
}

/// Puts window `id` back where it first opened: unfolded, not maximized.
pub fn reset(ctx: &egui::Context, id: egui::Id) {
    ctx.data_mut(|d| {
        d.remove::<Rect>(key(id, "restore-to"));
        d.insert_persisted(id.with("collapsed"), false);
        d.insert_temp(key(id, "reset"), true);
    });
}

/// A floating window `id` with the shared frame: no title bar of its own
/// (draw [`Header`] first in its contents), barely rounded, no shadow, kept
/// inside `area`, opening at `default` (a zero width or height leaves that
/// to the contents). When maximized it fills `area`.
/// `resizable` is false for fixed-size windows, which cannot be maximized.
pub fn window<'a>(
    ctx: &egui::Context,
    id: egui::Id,
    name: &str,
    area: Rect,
    default: Rect,
    resizable: bool,
) -> egui::Window<'a> {
    let folded = collapsed(ctx, id);
    let mut window = egui::Window::new(name.to_owned())
        .id(id)
        .title_bar(false)
        .frame(theme::window_frame(focused(ctx, id)))
        .constrain_to(area)
        .default_pos(default.min);
    if default.width() > 0.0 {
        window = window.default_width(default.width());
    }
    let reset = ctx.data_mut(|d| d.remove_temp::<bool>(key(id, "reset")).is_some());
    let restore = ctx.data_mut(|d| {
        let (rect, frames) = d.get_temp::<(Rect, u8)>(key(id, "restore-now"))?;
        if frames > 1 {
            d.insert_temp(key(id, "restore-now"), (rect, frames - 1));
        } else {
            d.remove::<(Rect, u8)>(key(id, "restore-now"));
        }
        Some(rect)
    });
    if resizable && maximized(ctx, id) {
        pin(window, area).movable(false)
    } else if let Some(rect) = restore {
        pin(window, rect)
    } else if reset {
        window
            .current_pos(default.min)
            .resizable(resizable && !folded)
    } else {
        window.resizable(resizable && !folded)
    }
}

/// Places `window` exactly over `rect` (its outer edge) for this frame.
fn pin(window: egui::Window<'_>, rect: Rect) -> egui::Window<'_> {
    window
        .current_pos(rect.min)
        .resizable(true)
        .min_size(rect.size())
        .max_size(rect.size())
}

/// Remembers where window `id` is drawn, so maximize can restore it.
/// Call it with the window's response after showing it.
pub fn remember(ctx: &egui::Context, id: egui::Id, response: Option<&egui::Response>) {
    if let Some(response) = response {
        if !maximized(ctx, id) {
            ctx.data_mut(|d| d.insert_temp(key(id, "rect"), response.rect));
        }
    }
}

/// The header row of window `id`.
pub struct Header<'a> {
    id: egui::Id,
    title: &'a str,
    title_at_end: bool,
    maximize: bool,
}

impl<'a> Header<'a> {
    /// A header titled `title` for the window `id`.
    #[must_use]
    pub fn new(id: egui::Id, title: &'a str) -> Self {
        Self {
            id,
            title,
            title_at_end: false,
            maximize: true,
        }
    }

    /// Puts the title at the right end, before the window buttons (mixer).
    #[must_use]
    pub fn title_at_end(mut self) -> Self {
        self.title_at_end = true;
        self
    }

    /// Leaves out maximize, for fixed-size windows.
    #[must_use]
    pub fn fixed_size(mut self) -> Self {
        self.maximize = false;
        self
    }

    /// Draws the header. `menu` adds the window's own menu items (the common
    /// items follow them), `tools` the window's tools; both get `data`. Close
    /// clears `open`. Returns whether the window is collapsed.
    pub fn show<T>(
        self,
        ui: &mut egui::Ui,
        open: &mut bool,
        data: &mut T,
        menu: impl FnOnce(&mut egui::Ui, &mut T),
        tools: impl FnOnce(&mut egui::Ui, &mut T),
    ) -> bool {
        self.show_with_extra(ui, open, data, menu, tools, |_, _| {})
    }

    /// Like [`Header::show`], with a second, lower-priority tool group
    /// `extra` after the title. When the row is too narrow, `extra` folds
    /// into the "more" button first, then `tools`.
    pub fn show_with_extra<T>(
        self,
        ui: &mut egui::Ui,
        open: &mut bool,
        data: &mut T,
        menu: impl FnOnce(&mut egui::Ui, &mut T),
        tools: impl FnOnce(&mut egui::Ui, &mut T),
        extra: impl FnOnce(&mut egui::Ui, &mut T),
    ) -> bool {
        let id = self.id;
        let ctx = ui.ctx().clone();
        let mut folded = collapsed(&ctx, id);
        let maximized = maximized(&ctx, id);
        let focused = focused(&ctx, id);
        if maximized && self.maximize {
            // Windows take their contents' height; a maximized one fills
            // the workspace whatever it shows.
            ui.set_min_size(ui.max_rect().size());
        }
        band(ui, focused);
        let width_of = |what: &str| {
            ctx.data_mut(|d| d.get_temp::<f32>(key(id, what)))
                .unwrap_or(0.0)
        };
        let (tools_width, extra_width) = (width_of("tools-width"), width_of("extra-width"));
        let title_width = title_galley(ui, self.title, focused).size().x;
        let buttons = if self.maximize { 3.0 } else { 2.0 } * 20.0;
        let room = ui.available_width() - 24.0 - buttons - title_width.min(TITLE_MIN) - 12.0;
        let fold_tools = tools_width > room;
        let fold_extra = tools_width + extra_width > room - 24.0;
        // Registered before the buttons, so they keep their clicks.
        let row_rect = Rect::from_min_size(ui.cursor().min, vec2(ui.available_width(), ROW_HEIGHT));
        let empty = ui.interact(row_rect, key(id, "header-empty"), Sense::click());
        if self.maximize && empty.double_clicked() {
            set_maximized(&ctx, id, !maximized);
        }
        ui.horizontal(|ui| {
            ui.set_min_height(ROW_HEIGHT);
            ui.spacing_mut().item_spacing.x = 2.0;
            let mut menu = Some(menu);
            window_menu(ui, id, self.maximize, maximized, folded, &mut menu, data);
            ui.add_space(4.0);
            let mut tools = Some(tools);
            let mut extra = Some(extra);
            if !fold_tools {
                group(ui, id, "tools-width", &mut tools, data);
                ui.add_space(6.0);
            }
            if !self.title_at_end {
                let reserve = if fold_extra { 0.0 } else { extra_width + 6.0 };
                title(ui, self.title, focused, buttons + 24.0 + reserve);
            }
            if !fold_extra {
                ui.add_space(6.0);
                group(ui, id, "extra-width", &mut extra, data);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::header_button(ui, Icon::Close, false, true, "Close").clicked() {
                    *open = false;
                }
                if self.maximize {
                    let (icon, hint) = if maximized {
                        (Icon::Restore, "Restore")
                    } else {
                        (Icon::Maximize, "Maximize")
                    };
                    if icons::header_button(ui, icon, false, false, hint).clicked() {
                        set_maximized(&ctx, id, !maximized);
                    }
                }
                let (icon, hint) = if folded {
                    (Icon::Unfold, "Expand")
                } else {
                    (Icon::Fold, "Collapse to header")
                };
                if icons::header_button(ui, icon, false, false, hint).clicked() {
                    folded = !folded;
                }
                if fold_tools || fold_extra {
                    more(ui, id, &mut tools, &mut extra, data);
                }
                if self.title_at_end {
                    ui.add_space(4.0);
                    title(ui, self.title, focused, 0.0);
                }
            });
        });
        ui.data_mut(|d| d.insert_persisted(id.with("collapsed"), folded));
        folded
    }
}

/// Draws a tool group in the header row and remembers its width under `what`.
fn group<T>(
    ui: &mut egui::Ui,
    id: egui::Id,
    what: &str,
    tools: &mut Option<impl FnOnce(&mut egui::Ui, &mut T)>,
    data: &mut T,
) {
    let shown = ui.scope(|ui| {
        if let Some(tools) = tools.take() {
            tools(ui, data);
        }
    });
    let width = shown.response.rect.width();
    ui.data_mut(|d| d.insert_temp(key(id, what), width));
}

/// Paints the header band behind the row, out to the window's edges.
fn band(ui: &egui::Ui, focused: bool) {
    let rect = ui.max_rect();
    let margin = 6.0;
    let band = Rect::from_min_max(
        rect.left_top() - vec2(margin - 1.0, margin - 1.0),
        egui::pos2(rect.right() + margin - 1.0, rect.top() + ROW_HEIGHT + 3.0),
    );
    let mut painter = ui.painter().clone();
    painter.set_clip_rect(band);
    painter.rect_filled(
        band,
        egui::CornerRadius {
            nw: 2,
            ne: 2,
            sw: 0,
            se: 0,
        },
        if focused {
            theme::HEADER_ACTIVE
        } else {
            theme::HEADER
        },
    );
}

fn title_galley(ui: &egui::Ui, text: &str, focused: bool) -> std::sync::Arc<egui::Galley> {
    ui.painter().layout_no_wrap(
        text.to_owned(),
        egui::FontId::proportional(theme::FONT_TITLE),
        if focused {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    )
}

/// The title, cut short with an ellipsis so `reserve` points stay free.
fn title(ui: &mut egui::Ui, text: &str, focused: bool, reserve: f32) {
    let width = (ui.available_width() - reserve).max(24.0);
    let label = egui::Label::new(egui::RichText::new(text).size(theme::FONT_TITLE).color(
        if focused {
            theme::TEXT
        } else {
            theme::TEXT_DIM
        },
    ))
    .truncate()
    .selectable(false);
    ui.scope(|ui| {
        ui.set_max_width(width);
        ui.add(label);
    });
}

fn window_menu<T>(
    ui: &mut egui::Ui,
    id: egui::Id,
    can_maximize: bool,
    maximized: bool,
    folded: bool,
    items: &mut Option<impl FnOnce(&mut egui::Ui, &mut T)>,
    data: &mut T,
) {
    let popup = key(id, "window-menu");
    let open = egui::Popup::is_id_open(ui.ctx(), popup);
    let response = icons::header_button(ui, Icon::Menu, open, false, "Window menu");
    let toggle = (response.clicked() || response.secondary_clicked())
        .then_some(egui::SetOpenCommand::Toggle);
    egui::Popup::menu(&response)
        .id(popup)
        .open_memory(toggle)
        .show(|ui| {
            let ctx = ui.ctx().clone();
            if let Some(items) = items.take() {
                let before = ui.next_widget_position();
                items(ui, data);
                if ui.next_widget_position() != before {
                    ui.separator();
                }
            }
            if can_maximize {
                let label = if maximized { "Restore" } else { "Maximize" };
                if ui.button(label).clicked() {
                    set_maximized(&ctx, id, !maximized);
                }
            }
            let label = if folded {
                "Expand"
            } else {
                "Collapse to header"
            };
            if ui.button(label).clicked() {
                ctx.data_mut(|d| d.insert_persisted(id.with("collapsed"), !folded));
            }
            if ui
                .button("Reset position")
                .hint("Put the window back where it first opened")
                .clicked()
            {
                reset(&ctx, id);
            }
        });
}

/// The "more" button that holds the tool groups that did not fit.
fn more<T>(
    ui: &mut egui::Ui,
    id: egui::Id,
    tools: &mut Option<impl FnOnce(&mut egui::Ui, &mut T)>,
    extra: &mut Option<impl FnOnce(&mut egui::Ui, &mut T)>,
    data: &mut T,
) {
    let popup = key(id, "more");
    let open = egui::Popup::is_id_open(ui.ctx(), popup);
    let response = icons::header_button(ui, Icon::More, open, false, "More tools");
    egui::Popup::menu(&response)
        .id(popup)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            ui.set_max_width(380.0);
            if let Some(tools) = tools.take() {
                ui.horizontal_wrapped(|ui| tools(ui, data));
            }
            if let Some(extra) = extra.take() {
                ui.horizontal_wrapped(|ui| extra(ui, data));
            }
        });
}
