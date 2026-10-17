//! A plugin's own window. On Linux the plugin's view is embedded in an X11
//! window MusicCraft creates (through XWayland on Wayland desktops); the host
//! frame handles resize requests and runs the timers and file descriptors the
//! view registers. Other platforms do not offer VST3 windows yet.

use std::sync::{Mutex, MutexGuard, PoisonError};

use vst3::ComPtr;
use vst3::Steinberg::{IPlugFrame, IPlugFrameTrait, IPlugView, ViewRect, kResultTrue, tresult};
use vst3::{Class, ComWrapper};

use crate::instance::Shared;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A timer a view registered.
#[cfg(target_os = "linux")]
struct Timer {
    handler: ComPtr<vst3::Steinberg::Linux::ITimerHandler>,
    period: std::time::Duration,
    due: std::time::Instant,
}

/// The host frame a view lives in; on Linux also the host run loop.
#[derive(Default)]
pub(crate) struct Frame {
    /// The size the view last asked for.
    requested: Mutex<Option<(i32, i32)>>,
    #[cfg(target_os = "linux")]
    timers: Mutex<Vec<Timer>>,
    #[cfg(target_os = "linux")]
    handlers: Mutex<Vec<(ComPtr<vst3::Steinberg::Linux::IEventHandler>, i32)>>,
}

#[cfg(target_os = "linux")]
impl Class for Frame {
    type Interfaces = (IPlugFrame, vst3::Steinberg::Linux::IRunLoop);
}

#[cfg(not(target_os = "linux"))]
impl Class for Frame {
    type Interfaces = (IPlugFrame,);
}

impl IPlugFrameTrait for Frame {
    unsafe fn resizeView(&self, _view: *mut IPlugView, size: *mut ViewRect) -> tresult {
        // SAFETY: the view passes a readable rectangle.
        let Some(size) = (unsafe { size.as_ref() }) else {
            return vst3::Steinberg::kInvalidArgument;
        };
        *lock(&self.requested) = Some((size.right - size.left, size.bottom - size.top));
        kResultTrue
    }
}

#[cfg(target_os = "linux")]
mod run_loop {
    use std::time::{Duration, Instant};

    use vst3::ComRef;
    use vst3::Steinberg::Linux::{
        FileDescriptor, IEventHandler, IRunLoopTrait, ITimerHandler, TimerInterval,
    };
    use vst3::Steinberg::{kInvalidArgument, kResultOk, tresult};

    use super::{Frame, Timer, lock};

    impl IRunLoopTrait for Frame {
        unsafe fn registerEventHandler(
            &self,
            handler: *mut IEventHandler,
            fd: FileDescriptor,
        ) -> tresult {
            // SAFETY: the view passes a valid handler; `to_com_ptr` keeps a
            // reference until it is unregistered.
            let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
                return kInvalidArgument;
            };
            lock(&self.handlers).push((handler.to_com_ptr(), fd));
            kResultOk
        }

        unsafe fn unregisterEventHandler(&self, handler: *mut IEventHandler) -> tresult {
            lock(&self.handlers).retain(|(h, _)| h.as_ptr() != handler);
            kResultOk
        }

        unsafe fn registerTimer(
            &self,
            handler: *mut ITimerHandler,
            milliseconds: TimerInterval,
        ) -> tresult {
            // SAFETY: as for event handlers.
            let Some(handler) = (unsafe { ComRef::from_raw(handler) }) else {
                return kInvalidArgument;
            };
            let period = Duration::from_millis(milliseconds.max(1));
            lock(&self.timers).push(Timer {
                handler: handler.to_com_ptr(),
                period,
                due: Instant::now() + period,
            });
            kResultOk
        }

        unsafe fn unregisterTimer(&self, handler: *mut ITimerHandler) -> tresult {
            lock(&self.timers).retain(|t| t.handler.as_ptr() != handler);
            kResultOk
        }
    }
}

#[derive(Default)]
pub(crate) struct Editor {
    open: Option<Open>,
}

struct Open {
    view: ComPtr<IPlugView>,
    // Only read by the Linux run loop; held everywhere so the view's frame
    // lives until it is removed.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    frame: ComWrapper<Frame>,
    #[cfg(target_os = "linux")]
    window: Option<musiccraft_plugin_window::Window>,
}

/// The X11 platform type name.
#[cfg(target_os = "linux")]
fn x11() -> vst3::Steinberg::FIDString {
    vst3::Steinberg::kPlatformTypeX11EmbedWindowID
}

/// A new editor view from the plugin's controller, if it embeds in X11.
#[cfg(target_os = "linux")]
fn create_view(shared: &Shared) -> Result<ComPtr<IPlugView>, String> {
    use vst3::Steinberg::IPlugViewTrait;
    use vst3::Steinberg::Vst::IEditControllerTrait;
    let controller = shared
        .controller
        .as_ref()
        .ok_or("the plugin has no window")?;
    // SAFETY: main thread; the name is a static C string. The view comes
    // with one reference, which `ComPtr` takes over.
    let view = unsafe { ComPtr::from_raw(controller.createView(c"editor".as_ptr())) }
        .ok_or("the plugin has no window")?;
    // SAFETY: main thread; the type is a static C string.
    if unsafe { view.isPlatformTypeSupported(x11()) } != kResultTrue {
        return Err("the plugin's window type is not supported".to_owned());
    }
    Ok(view)
}

impl Editor {
    /// Whether the plugin has a window MusicCraft can show.
    pub fn supported(shared: &Shared) -> bool {
        #[cfg(target_os = "linux")]
        {
            create_view(shared).is_ok()
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = shared;
            false
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open(&mut self, shared: &Shared, title: &str) -> Result<(), String> {
        if self.open.is_some() {
            return Ok(());
        }
        #[cfg(target_os = "linux")]
        {
            use vst3::Steinberg::IPlugViewTrait;
            let view = create_view(shared)?;
            let mut size = ViewRect {
                left: 0,
                top: 0,
                right: 640,
                bottom: 480,
            };
            // SAFETY: main thread; `size` is writable.
            unsafe { view.getSize(&mut size) };
            // SAFETY: main thread.
            let resizable = unsafe { view.canResize() } == kResultTrue;
            let (width, height) = (
                u32::try_from(size.right - size.left).unwrap_or(640).max(50),
                u32::try_from(size.bottom - size.top).unwrap_or(480).max(50),
            );
            let window = musiccraft_plugin_window::Window::new(title, width, height, resizable)
                .map_err(|err| format!("cannot open a window: {err}"))?;
            let parent = u64::from(window.id());
            self.attach_view(view, parent, Some(window))?;
            if let Some(window) = self.open.as_ref().and_then(|o| o.window.as_ref()) {
                window.map();
            }
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = (shared, title);
            Err("VST3 windows are not supported on this platform yet".to_owned())
        }
    }

    /// Creates the view and attaches it to window `parent`.
    #[cfg(all(test, target_os = "linux"))]
    pub fn attach(
        &mut self,
        shared: &Shared,
        parent: u64,
        window: Option<musiccraft_plugin_window::Window>,
    ) -> Result<(), String> {
        let view = create_view(shared)?;
        self.attach_view(view, parent, window)
    }

    #[cfg(target_os = "linux")]
    fn attach_view(
        &mut self,
        view: ComPtr<IPlugView>,
        parent: u64,
        window: Option<musiccraft_plugin_window::Window>,
    ) -> Result<(), String> {
        use vst3::Steinberg::IPlugViewTrait;
        let frame = ComWrapper::new(Frame::default());
        let frame_ptr = frame
            .as_com_ref::<IPlugFrame>()
            .map_or(std::ptr::null_mut(), |r| r.as_ptr());
        // SAFETY: main thread; the frame lives in `Open` until the view is
        // removed and its frame cleared in `close`.
        unsafe { view.setFrame(frame_ptr) };
        // An X11 window id passed as the parent "pointer", as the interface
        // defines for this platform type.
        let parent_ptr = parent as usize as *mut std::ffi::c_void;
        // SAFETY: main thread; `parent` names a live window that outlives the
        // view (held in `Open`, or the caller's in tests).
        if unsafe { view.attached(parent_ptr, x11()) } != kResultTrue {
            // SAFETY: main thread; undoing `setFrame` above.
            unsafe { view.setFrame(std::ptr::null_mut()) };
            return Err("the plugin refused its window".to_owned());
        }
        self.open = Some(Open {
            view,
            frame,
            window,
        });
        Ok(())
    }

    /// The size the view last asked its frame for.
    #[cfg(all(test, target_os = "linux"))]
    pub fn requested_size(&self) -> Option<(i32, i32)> {
        self.open.as_ref().and_then(|o| *lock(&o.frame.requested))
    }

    /// Runs due timers and ready file descriptors the view registered.
    pub fn run_loop(&mut self) {
        #[cfg(target_os = "linux")]
        if let Some(open) = &self.open {
            use vst3::Steinberg::Linux::{IEventHandlerTrait, ITimerHandlerTrait};
            let now = std::time::Instant::now();
            // Collected first: a callback may register or unregister.
            let due: Vec<_> = lock(&open.frame.timers)
                .iter_mut()
                .filter(|t| t.due <= now)
                .map(|t| {
                    t.due = now + t.period;
                    t.handler.clone()
                })
                .collect();
            for handler in due {
                // SAFETY: main thread; the view registered this timer.
                unsafe { handler.onTimer() };
            }
            let handlers: Vec<_> = lock(&open.frame.handlers).clone();
            let fds: Vec<i32> = handlers.iter().map(|(_, fd)| *fd).collect();
            for fd in crate::fd::ready(&fds) {
                for (handler, _) in handlers.iter().filter(|(_, f)| *f == fd) {
                    // SAFETY: main thread; the view registered this handler.
                    unsafe { handler.onFDIsSet(fd) };
                }
            }
        }
    }

    /// Handles window events and resize requests. Returns `true` if the window
    /// closed.
    pub fn poll(&mut self, _shared: &Shared) -> bool {
        #[cfg(target_os = "linux")]
        {
            use vst3::Steinberg::IPlugViewTrait;
            let Some(open) = &self.open else {
                return false;
            };
            let requested = lock(&open.frame.requested).take();
            if let Some((w, h)) = requested {
                let mut rect = ViewRect {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                };
                // SAFETY: main thread; the view asked for this size.
                unsafe { open.view.onSize(&mut rect) };
                if let Some(window) = &open.window {
                    window.resize(
                        u32::try_from(w).unwrap_or(1).max(1),
                        u32::try_from(h).unwrap_or(1).max(1),
                    );
                }
            }
            let mut closed = false;
            if let Some(window) = &open.window {
                match window.poll() {
                    musiccraft_plugin_window::Polled::Closed => closed = true,
                    musiccraft_plugin_window::Polled::Resized(w, h) => {
                        let mut rect = ViewRect {
                            left: 0,
                            top: 0,
                            right: i32::try_from(w).unwrap_or(i32::MAX),
                            bottom: i32::try_from(h).unwrap_or(i32::MAX),
                        };
                        // SAFETY: main thread; `rect` is writable.
                        let resizable = unsafe { open.view.canResize() } == kResultTrue;
                        if resizable {
                            // SAFETY: as above.
                            unsafe {
                                open.view.checkSizeConstraint(&mut rect);
                                open.view.onSize(&mut rect);
                            }
                        } else {
                            // SAFETY: as above.
                            unsafe { open.view.getSize(&mut rect) };
                        }
                        window.resize(
                            u32::try_from(rect.right - rect.left).unwrap_or(1).max(1),
                            u32::try_from(rect.bottom - rect.top).unwrap_or(1).max(1),
                        );
                    }
                    musiccraft_plugin_window::Polled::Nothing => {}
                }
            }
            if closed {
                self.close();
            }
            closed
        }
        #[cfg(not(target_os = "linux"))]
        false
    }

    pub fn close(&mut self) {
        let Some(open) = self.open.take() else {
            return;
        };
        use vst3::Steinberg::IPlugViewTrait;
        // SAFETY: main thread; the view is attached.
        unsafe {
            open.view.removed();
            open.view.setFrame(std::ptr::null_mut());
        }
        // The view is released, then our X window destroyed, as `open` drops.
        drop(open);
    }
}
