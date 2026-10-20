//! Opens MusicCraft's egui editors inside a plugin host's window.
//!
//! [`EguiEditor`] is a nih-plug [`Editor`]: when the host asks for the GUI it
//! opens a baseview child window with an OpenGL context in the host's window,
//! runs egui 0.36 in it every frame and paints with `egui_glow`. The plugin
//! supplies the drawing as a closure that gets the frame's [`egui::Ui`] and the
//! host's [`GuiContext`] (for parameter gestures).
//!
//! This crate may use `unsafe` (roadmap decision D1): OpenGL calls and moving
//! baseview's window handle to the thread the host drops it on.

use std::any::Any;
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{Duration, Instant};

use baseview::gl::GlConfig;
use baseview::{
    Event, EventStatus, MouseButton, MouseCursor, MouseEvent, ScrollDelta, Size, Window,
    WindowEvent, WindowHandle, WindowHandler, WindowInfo, WindowOpenOptions, WindowScalePolicy,
};
use keyboard_types::{Key, KeyState, KeyboardEvent};
use musiccraft_editors::theme;
use nih_plug::prelude::{Editor, GuiContext, ParentWindowHandle};

/// What an editor draws each frame.
pub type Draw = dyn Fn(&mut egui::Ui, &dyn GuiContext) + Send + Sync;

/// An editor of a fixed logical size that draws with egui.
pub struct EguiEditor {
    size: (u32, u32),
    /// The host's scale factor as `f32` bits; 0 until the host sets one.
    scale: AtomicU32,
    open: Arc<AtomicBool>,
    draw: Arc<Draw>,
}

impl EguiEditor {
    /// An editor `size` logical pixels big that draws with `draw`.
    pub fn new(
        size: (u32, u32),
        draw: impl Fn(&mut egui::Ui, &dyn GuiContext) + Send + Sync + 'static,
    ) -> Self {
        Self {
            size,
            scale: AtomicU32::new(0),
            open: Arc::new(AtomicBool::new(false)),
            draw: Arc::new(draw),
        }
    }

    /// Whether the editor's window is open (shared, so the plugin can skip
    /// work only the editor shows).
    #[must_use]
    pub fn open_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.open)
    }

    fn scale_factor(&self) -> Option<f32> {
        match self.scale.load(Ordering::Relaxed) {
            0 => None,
            bits => Some(f32::from_bits(bits)),
        }
    }
}

impl Editor for EguiEditor {
    fn spawn(
        &self,
        parent: ParentWindowHandle,
        context: Arc<dyn GuiContext>,
    ) -> Box<dyn Any + Send> {
        let scale = self.scale_factor();
        let draw = Arc::clone(&self.draw);
        let (width, height) = self.size;
        let options = WindowOpenOptions {
            title: "MusicCraft".to_owned(),
            size: Size::new(f64::from(width), f64::from(height)),
            // macOS reports its own scale; elsewhere the host's (or 1).
            scale: scale.map_or(WindowScalePolicy::SystemScaleFactor, |s| {
                WindowScalePolicy::ScaleFactor(f64::from(s))
            }),
            gl_config: Some(GlConfig {
                version: (3, 2),
                red_bits: 8,
                green_bits: 8,
                blue_bits: 8,
                alpha_bits: 8,
                depth_bits: 24,
                stencil_bits: 8,
                samples: None,
                // egui_glow does its own gamma and wants a plain framebuffer.
                srgb: false,
                double_buffer: true,
                vsync: false,
                ..GlConfig::default()
            }),
        };
        let size = self.size;
        // baseview panics when the system has no matching OpenGL setup; a
        // panic must not take the host down, so the editor stays empty.
        let opened = std::panic::catch_unwind(AssertUnwindSafe(|| {
            Window::open_parented(&parent, options, move |window: &mut Window<'_>| {
                EguiWindow::new(window, size, scale.unwrap_or(1.0), draw, context)
            })
        }));
        let Ok(window) = opened else {
            return Box::new(());
        };
        self.open.store(true, Ordering::Release);
        Box::new(OpenEditor {
            window,
            open: Arc::clone(&self.open),
        })
    }

    fn size(&self) -> (u32, u32) {
        self.size
    }

    fn set_scale_factor(&self, factor: f32) -> bool {
        // The window cannot follow a change while it is open.
        if self.open.load(Ordering::Acquire) || !(factor.is_finite() && factor > 0.0) {
            return false;
        }
        self.scale.store(factor.to_bits(), Ordering::Relaxed);
        true
    }

    // The window repaints every frame, so changes show without being told.
    fn param_value_changed(&self, _id: &str, _normalized_value: f32) {}

    fn param_modulation_changed(&self, _id: &str, _modulation_offset: f32) {}

    fn param_values_changed(&self) {}
}

/// The handle the host keeps while the editor is open; dropping it closes
/// the window.
struct OpenEditor {
    window: WindowHandle,
    open: Arc<AtomicBool>,
}

// SAFETY: `WindowHandle` holds platform pointers, which makes it `!Send`.
// nih-plug creates and drops this handle on the host's GUI thread (the thread
// that called `spawn`); it is only moved through `Box<dyn Any + Send>`, never
// used from another thread.
unsafe impl Send for OpenEditor {}

/// Longest wait for an X11 window's thread to finish after closing.
const CLOSE_WAIT: Duration = Duration::from_millis(500);

impl Drop for OpenEditor {
    fn drop(&mut self) {
        self.open.store(false, Ordering::Release);
        self.window.close();
        // On X11 baseview closes the window on its own thread, after this
        // returns, and the host destroys the parent window right after: a
        // frame drawn in between fails. Wait (briefly) for that thread to
        // finish. Elsewhere closing is immediate.
        let deadline = Instant::now() + CLOSE_WAIT;
        while self.window.is_open() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// OpenGL state of an open window.
struct Renderer {
    gl: Arc<glow::Context>,
    painter: egui_glow::Painter,
}

/// The baseview window handler: collects input, runs egui, paints.
struct EguiWindow {
    ctx: egui::Context,
    renderer: Option<Renderer>,
    draw: Arc<Draw>,
    gui: Arc<dyn GuiContext>,
    started: Instant,
    events: Vec<egui::Event>,
    pointer: egui::Pos2,
    modifiers: egui::Modifiers,
    /// Logical size and scale.
    size: egui::Vec2,
    scale: f32,
    cursor: MouseCursor,
}

impl EguiWindow {
    fn new(
        window: &mut Window<'_>,
        (width, height): (u32, u32),
        scale: f32,
        draw: Arc<Draw>,
        gui: Arc<dyn GuiContext>,
    ) -> Self {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        Self {
            ctx,
            renderer: Renderer::new(window),
            draw,
            gui,
            started: Instant::now(),
            events: Vec::new(),
            pointer: egui::Pos2::ZERO,
            modifiers: egui::Modifiers::NONE,
            size: egui::vec2(width as f32, height as f32),
            scale,
            cursor: MouseCursor::Default,
        }
    }

    fn input(&mut self) -> egui::RawInput {
        let mut events = vec![egui::Event::ModifiersChanged(self.modifiers)];
        events.append(&mut self.events);
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            time: Some(self.started.elapsed().as_secs_f64()),
            events,
            ..egui::RawInput::default()
        };
        if let Some(viewport) = input.viewports.get_mut(&egui::ViewportId::ROOT) {
            viewport.native_pixels_per_point = Some(self.scale);
            viewport.focused = Some(true);
        }
        input
    }
}

impl Renderer {
    /// Loads OpenGL for the window; `None` if it has no context or the
    /// painter cannot be built (the window then stays blank).
    fn new(window: &Window<'_>) -> Option<Self> {
        let context = window.gl_context()?;
        // SAFETY: the context belongs to this window and is made current on
        // the window's own thread, which is the only thread that uses it.
        unsafe { context.make_current() };
        // SAFETY: the loader returns the current context's function pointers;
        // glow only calls them while that context is current.
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function(|name| context.get_proc_address(name))
        });
        let painter = egui_glow::Painter::new(Arc::clone(&gl), "", None, false).ok();
        // SAFETY: as above; the context was made current just before.
        unsafe { context.make_not_current() };
        painter.map(|painter| Self { gl, painter })
    }
}

impl WindowHandler for EguiWindow {
    fn on_frame(&mut self, window: &mut Window) {
        let input = self.input();
        let draw = Arc::clone(&self.draw);
        let gui = Arc::clone(&self.gui);
        let output = self.ctx.run_ui(input, |ui| {
            egui::Frame::new()
                .fill(theme::PANEL)
                .inner_margin(10.0)
                .show(ui, |ui| draw(ui, gui.as_ref()));
        });
        let cursor = cursor(output.platform_output.cursor_icon);
        if cursor != self.cursor {
            self.cursor = cursor;
            window.set_mouse_cursor(cursor);
        }
        let (Some(renderer), Some(context)) = (self.renderer.as_mut(), window.gl_context()) else {
            return;
        };
        let pixels_per_point = output.pixels_per_point;
        let primitives = self.ctx.tessellate(output.shapes, pixels_per_point);
        let mut textures = output.textures_delta;
        let physical = [
            (self.size.x * self.scale).round().max(1.0) as u32,
            (self.size.y * self.scale).round().max(1.0) as u32,
        ];
        // SAFETY: this window's context, made current on its own thread for
        // the duration of the frame.
        unsafe { context.make_current() };
        let [r, g, b, _] = theme::PANEL.to_normalized_gamma_f32();
        {
            use glow::HasContext as _;
            // SAFETY: plain state calls on the current context.
            unsafe {
                renderer
                    .gl
                    .viewport(0, 0, physical[0] as i32, physical[1] as i32);
                renderer.gl.clear_color(r, g, b, 1.0);
                renderer.gl.clear(glow::COLOR_BUFFER_BIT);
            }
        }
        renderer.painter.paint_and_update_textures(
            physical,
            pixels_per_point,
            &primitives,
            &mut textures,
        );
        context.swap_buffers();
        // SAFETY: as above.
        unsafe { context.make_not_current() };
    }

    fn on_event(&mut self, window: &mut Window, event: Event) -> EventStatus {
        match event {
            Event::Mouse(event) => {
                self.mouse(event);
                EventStatus::Captured
            }
            Event::Keyboard(event) => {
                if self.ctx.egui_wants_keyboard_input() {
                    self.key(&event);
                    EventStatus::Captured
                } else {
                    // Let the host have it (its computer keyboard, shortcuts).
                    EventStatus::Ignored
                }
            }
            Event::Window(WindowEvent::Resized(info)) => {
                self.resize(&info);
                EventStatus::Captured
            }
            Event::Window(WindowEvent::Focused) => {
                self.events.push(egui::Event::WindowFocused(true));
                EventStatus::Captured
            }
            Event::Window(WindowEvent::Unfocused) => {
                self.events.push(egui::Event::WindowFocused(false));
                EventStatus::Captured
            }
            Event::Window(WindowEvent::WillClose) => {
                if let Some(mut renderer) = self.renderer.take() {
                    if let Some(context) = window.gl_context() {
                        // SAFETY: this window's context, on its own thread.
                        unsafe { context.make_current() };
                        renderer.painter.destroy();
                        // SAFETY: as above.
                        unsafe { context.make_not_current() };
                    }
                }
                EventStatus::Captured
            }
        }
    }
}

impl EguiWindow {
    fn resize(&mut self, info: &WindowInfo) {
        let logical = info.logical_size();
        self.size = egui::vec2(logical.width as f32, logical.height as f32);
        self.scale = info.scale() as f32;
    }

    fn mouse(&mut self, event: MouseEvent) {
        match event {
            MouseEvent::CursorMoved {
                position,
                modifiers,
            } => {
                self.modifiers = egui_modifiers(modifiers);
                self.pointer = egui::pos2(position.x as f32, position.y as f32);
                self.events.push(egui::Event::PointerMoved(self.pointer));
            }
            MouseEvent::ButtonPressed { button, modifiers }
            | MouseEvent::ButtonReleased { button, modifiers } => {
                let pressed = matches!(event, MouseEvent::ButtonPressed { .. });
                self.modifiers = egui_modifiers(modifiers);
                if let Some(button) = pointer_button(button) {
                    self.events.push(egui::Event::PointerButton {
                        pos: self.pointer,
                        button,
                        pressed,
                        modifiers: self.modifiers,
                    });
                }
            }
            MouseEvent::WheelScrolled { delta, modifiers } => {
                self.modifiers = egui_modifiers(modifiers);
                let (unit, x, y) = match delta {
                    ScrollDelta::Lines { x, y } => (egui::MouseWheelUnit::Line, x, y),
                    ScrollDelta::Pixels { x, y } => (egui::MouseWheelUnit::Point, x, y),
                };
                self.events.push(egui::Event::MouseWheel {
                    unit,
                    delta: egui::vec2(x, y),
                    phase: egui::TouchPhase::Move,
                    modifiers: self.modifiers,
                });
            }
            MouseEvent::CursorLeft => self.events.push(egui::Event::PointerGone),
            _ => {}
        }
    }

    fn key(&mut self, event: &KeyboardEvent) {
        let pressed = event.state == KeyState::Down;
        self.modifiers = egui_modifiers(event.modifiers);
        if let Key::Character(text) = &event.key {
            if pressed && !self.modifiers.ctrl && !self.modifiers.mac_cmd {
                self.events.push(egui::Event::Text(text.clone()));
            }
            if let Some(key) = egui::Key::from_name(text) {
                self.push_key(key, pressed, event.repeat);
            }
        } else if let Some(key) = named_key(&event.key) {
            self.push_key(key, pressed, event.repeat);
        }
    }

    fn push_key(&mut self, key: egui::Key, pressed: bool, repeat: bool) {
        self.events.push(egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat,
            modifiers: self.modifiers,
        });
    }
}

fn egui_modifiers(modifiers: keyboard_types::Modifiers) -> egui::Modifiers {
    use keyboard_types::Modifiers as M;
    let ctrl = modifiers.contains(M::CONTROL);
    let meta = modifiers.contains(M::META);
    egui::Modifiers {
        alt: modifiers.contains(M::ALT),
        ctrl,
        shift: modifiers.contains(M::SHIFT),
        mac_cmd: cfg!(target_os = "macos") && meta,
        command: if cfg!(target_os = "macos") {
            meta
        } else {
            ctrl
        },
    }
}

fn pointer_button(button: MouseButton) -> Option<egui::PointerButton> {
    Some(match button {
        MouseButton::Left => egui::PointerButton::Primary,
        MouseButton::Right => egui::PointerButton::Secondary,
        MouseButton::Middle => egui::PointerButton::Middle,
        MouseButton::Back => egui::PointerButton::Extra1,
        MouseButton::Forward => egui::PointerButton::Extra2,
        MouseButton::Other(_) => return None,
    })
}

fn named_key(key: &Key) -> Option<egui::Key> {
    Some(match key {
        Key::ArrowDown => egui::Key::ArrowDown,
        Key::ArrowLeft => egui::Key::ArrowLeft,
        Key::ArrowRight => egui::Key::ArrowRight,
        Key::ArrowUp => egui::Key::ArrowUp,
        Key::Backspace => egui::Key::Backspace,
        Key::Delete => egui::Key::Delete,
        Key::End => egui::Key::End,
        Key::Enter => egui::Key::Enter,
        Key::Escape => egui::Key::Escape,
        Key::Home => egui::Key::Home,
        Key::Insert => egui::Key::Insert,
        Key::PageDown => egui::Key::PageDown,
        Key::PageUp => egui::Key::PageUp,
        Key::Tab => egui::Key::Tab,
        _ => return None,
    })
}

fn cursor(icon: egui::CursorIcon) -> MouseCursor {
    use egui::CursorIcon as C;
    match icon {
        C::PointingHand => MouseCursor::Hand,
        C::Grab => MouseCursor::Hand,
        C::Grabbing => MouseCursor::HandGrabbing,
        C::Text => MouseCursor::Text,
        C::Move | C::AllScroll => MouseCursor::Move,
        C::ResizeHorizontal | C::ResizeColumn => MouseCursor::EwResize,
        C::ResizeVertical | C::ResizeRow => MouseCursor::NsResize,
        C::NotAllowed => MouseCursor::NotAllowed,
        C::Crosshair => MouseCursor::Crosshair,
        C::None => MouseCursor::Hidden,
        _ => MouseCursor::Default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modifiers_map_command_per_platform() {
        let ctrl = egui_modifiers(keyboard_types::Modifiers::CONTROL);
        assert!(ctrl.ctrl);
        assert_eq!(ctrl.command, !cfg!(target_os = "macos"));
        let shift = egui_modifiers(keyboard_types::Modifiers::SHIFT);
        assert!(shift.shift && !shift.command);
    }

    #[test]
    fn buttons_and_keys_map() {
        assert_eq!(
            pointer_button(MouseButton::Left),
            Some(egui::PointerButton::Primary)
        );
        assert_eq!(pointer_button(MouseButton::Other(9)), None);
        assert_eq!(named_key(&Key::Enter), Some(egui::Key::Enter));
        assert_eq!(
            cursor(egui::CursorIcon::Grabbing),
            MouseCursor::HandGrabbing
        );
    }

    #[test]
    fn scale_is_refused_while_open() {
        let editor = EguiEditor::new((100, 80), |_, _| {});
        assert_eq!(editor.size(), (100, 80));
        assert!(editor.set_scale_factor(2.0));
        assert_eq!(editor.scale_factor(), Some(2.0));
        assert!(!editor.set_scale_factor(f32::NAN));
        editor.open.store(true, Ordering::Release);
        assert!(!editor.set_scale_factor(1.5));
        assert_eq!(editor.scale_factor(), Some(2.0));
    }
}
