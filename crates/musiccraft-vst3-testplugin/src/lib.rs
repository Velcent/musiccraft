//! Two tiny VST3 plugins used to test the MusicCraft host through the real
//! VST3 interfaces. Built as a `cdylib` it is a normal VST3 module; linked as
//! an `rlib`, tests can call [`GetPluginFactory`] without loading a file.
//!
//! - **Test Gain** (effect): a separate processor and controller, connected
//!   through `IConnectionPoint`. Parameter 1 `Gain` is normalized 0–1 for a
//!   gain of 0–2 (default 0.5, unity), shown as `1.00x`. The processor greets
//!   the controller with a host-made message when they connect; until it has,
//!   the parameter is titled `Gain (unlinked)`. Its window (X11 embedding)
//!   asks the host frame for a size of 400×250 and registers a 1 ms timer on
//!   the host run loop; the timer moves `Gain` to 0.75 through the component
//!   handler, as if a knob in the window were turned.
//! - **Test Tone** (instrument): one object that is its own controller. A sine
//!   per held note at `Level × 0.5 × velocity` ([`TONE_PEAK`]); parameter 1
//!   `Level` (0–1, default 1). A note on key 127 makes the processor report
//!   `Level = 0.25` in its output parameter changes.
//!
//! The processors save their parameter as eight little-endian bytes; the
//! Gain controller saves its own state (default `ctl1`) and hands it back.
//!
//! Like `musiccraft-vst3`, this crate is exempt from the workspace's `unsafe`
//! ban: a VST3 plugin is a set of COM objects called through raw pointers.
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
// VST3 enum types are `int` on Windows and `unsigned int` elsewhere, so the
// same cast is needed on one platform and redundant on the other.
#![allow(clippy::unnecessary_cast)]
#![allow(non_snake_case)]

use std::ffi::{CStr, c_char, c_void};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use vst3::Steinberg::Vst::{
    BusDirection, BusDirections_, BusInfo, BusInfo_, BusTypes_, Event, Event_, IAttributeListTrait,
    IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler, IComponentTrait,
    IConnectionPoint, IConnectionPointTrait, IEditController, IEditControllerTrait,
    IEventListTrait, IHostApplication, IHostApplicationTrait, IMessage, IMessageTrait,
    IParamValueQueueTrait, IParameterChangesTrait, IoMode, MediaType, MediaTypes_, ParameterInfo,
    ParameterInfo_, ProcessData, ProcessSetup, RoutingInfo, SpeakerArr, SpeakerArrangement,
    String128, SymbolicSampleSizes_, TChar,
};
use vst3::Steinberg::{
    FIDString, FUnknown, IBStream, IBStreamTrait, IPlugFrame, IPlugFrameTrait, IPlugView,
    IPlugViewTrait, IPluginBaseTrait, IPluginFactory, IPluginFactory2, IPluginFactory2Trait,
    IPluginFactoryTrait, PClassInfo, PClassInfo_, PClassInfo2, PFactoryInfo, PFactoryInfo_, TBool,
    TUID, ViewRect, char16, int16, kInvalidArgument, kNotImplemented, kResultFalse, kResultOk,
    kResultTrue, tresult,
};
use vst3::{Class, ComPtr, ComRef, ComWrapper, Interface, uid};

/// Class id of the Test Gain processor.
pub const GAIN_CID: TUID = uid(0x4D43_5654, 0x4741_494E, 0x5052_4F43, 0x0000_0001);
/// Class id of the Test Gain controller.
pub const GAIN_CONTROLLER_CID: TUID = uid(0x4D43_5654, 0x4741_494E, 0x4354_524C, 0x0000_0002);
/// Class id of Test Tone.
pub const TONE_CID: TUID = uid(0x4D43_5654, 0x544F_4E45, 0x5052_4F43, 0x0000_0003);
/// The single parameter's id in both plugins.
pub const PARAM: u32 = 1;
/// Test Tone's peak level at `Level = 1` and full velocity.
pub const TONE_PEAK: f32 = 0.5;
/// Test Gain's controller state before anything is restored.
pub const CONTROLLER_STATE: &[u8] = b"ctl1";
/// The size Test Gain's window asks the host frame for.
pub const GAIN_VIEW_SIZE: (i32, i32) = (400, 250);
/// The value Test Gain's window timer sets `Gain` to.
pub const GAIN_VIEW_EDIT: f64 = 0.75;

// --- helpers ------------------------------------------------------------

fn copy_cstring(src: &str, dst: &mut [c_char]) {
    let n = src.len().min(dst.len().saturating_sub(1));
    for (d, s) in dst.iter_mut().zip(src.bytes().take(n)) {
        *d = s as c_char;
    }
    if let Some(end) = dst.get_mut(n) {
        *end = 0;
    }
}

fn copy_wstring(src: &str, dst: &mut [TChar]) {
    let mut n = 0;
    for (d, s) in dst.iter_mut().zip(src.encode_utf16()) {
        *d = s as TChar;
        n += 1;
    }
    let end = n.min(dst.len().saturating_sub(1));
    if let Some(end) = dst.get_mut(end) {
        *end = 0;
    }
}

/// Reads a whole `IBStream` (until a read returns nothing).
///
/// # Safety
///
/// `stream` must be null or a valid stream for the call.
unsafe fn read_all(stream: *mut IBStream) -> Vec<u8> {
    // SAFETY: guaranteed by the caller.
    let Some(stream) = (unsafe { ComRef::from_raw(stream) }) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut chunk = [0u8; 64];
    loop {
        let mut read = 0;
        // SAFETY: `chunk` is writable for its length.
        let result = unsafe { stream.read(chunk.as_mut_ptr().cast(), 64, &mut read) };
        let Ok(n) = usize::try_from(read) else { break };
        if result != kResultOk || n == 0 {
            break;
        }
        out.extend_from_slice(chunk.get(..n).unwrap_or_default());
    }
    out
}

/// Writes `data` to an `IBStream`.
///
/// # Safety
///
/// `stream` must be null or a valid stream for the call.
unsafe fn write_all(stream: *mut IBStream, data: &[u8]) -> tresult {
    // SAFETY: guaranteed by the caller.
    let Some(stream) = (unsafe { ComRef::from_raw(stream) }) else {
        return kInvalidArgument;
    };
    let mut written = 0;
    // SAFETY: `data` is readable for its length; the stream does not keep it.
    unsafe {
        stream.write(
            data.as_ptr().cast_mut().cast(),
            data.len() as i32,
            &mut written,
        )
    }
}

fn read_f64(data: &[u8]) -> Option<f64> {
    Some(f64::from_le_bytes(data.get(..8)?.try_into().ok()?))
}

fn stereo_bus(bus: &mut BusInfo, media: MediaType, dir: BusDirection, name: &str) {
    bus.mediaType = media;
    bus.direction = dir;
    bus.channelCount = if media == MediaTypes_::kAudio as MediaType {
        2
    } else {
        1
    };
    copy_wstring(name, &mut bus.name);
    bus.busType = BusTypes_::kMain as i32;
    bus.flags = BusInfo_::BusFlags_::kDefaultActive as u32;
}

/// A parameter's info, filled in.
fn param_info(info: &mut ParameterInfo, title: &str, default: f64) {
    info.id = PARAM;
    copy_wstring(title, &mut info.title);
    copy_wstring(title, &mut info.shortTitle);
    copy_wstring("", &mut info.units);
    info.stepCount = 0;
    info.defaultNormalizedValue = default;
    info.unitId = 0;
    info.flags = ParameterInfo_::ParameterFlags_::kCanAutomate;
}

/// The latest value for `PARAM` in a `process` call's input changes.
///
/// # Safety
///
/// `data` must be the `ProcessData` of the current `process` call.
unsafe fn param_change(data: &ProcessData) -> Option<f64> {
    // SAFETY: the host passes valid (or null) parameter changes.
    let changes = unsafe { ComRef::from_raw(data.inputParameterChanges) }?;
    let mut latest = None;
    // SAFETY: the changes object is valid for the call.
    let count = unsafe { changes.getParameterCount() };
    for i in 0..count {
        // SAFETY: `i` is below the count; the queue is valid for the call.
        let Some(queue) = (unsafe { ComRef::from_raw(changes.getParameterData(i)) }) else {
            continue;
        };
        // SAFETY: as above.
        if unsafe { queue.getParameterId() } != PARAM {
            continue;
        }
        // SAFETY: as above.
        let points = unsafe { queue.getPointCount() };
        let (mut offset, mut value) = (0, 0.0);
        // SAFETY: the last point exists when the count is positive; the
        // outputs are writable.
        if points > 0 && unsafe { queue.getPoint(points - 1, &mut offset, &mut value) } == kResultOk
        {
            latest = Some(value);
        }
    }
    latest
}

/// The stereo output (and input, for effects) channels of a `process` call.
///
/// # Safety
///
/// `buses` must be the bus array of the current `process` call, with 32-bit
/// buffers of `samples` samples, and the slices must not outlive the call.
unsafe fn channels<'a>(
    samples: i32,
    bus_count: i32,
    buses: *mut vst3::Steinberg::Vst::AudioBusBuffers,
) -> Option<[&'a mut [f32]; 2]> {
    if bus_count < 1 || buses.is_null() {
        return None;
    }
    // SAFETY: at least one bus exists.
    let bus = unsafe { &*buses };
    if bus.numChannels < 2 {
        return None;
    }
    let n = usize::try_from(samples).ok()?;
    // SAFETY: 32-bit processing was set up, so the union holds 32-bit
    // pointers, one per channel.
    let pointers = unsafe { std::slice::from_raw_parts(bus.__field0.channelBuffers32, 2) };
    // SAFETY: each channel holds `numSamples` samples; left and right are
    // different buffers.
    Some(unsafe {
        [
            std::slice::from_raw_parts_mut(pointers[0], n),
            std::slice::from_raw_parts_mut(pointers[1], n),
        ]
    })
}

// --- Test Gain: processor -------------------------------------------------

struct GainProcessor {
    /// Normalized gain, as `f64` bits.
    gain: AtomicU64,
    host: Mutex<Option<ComPtr<IHostApplication>>>,
}

impl Class for GainProcessor {
    type Interfaces = (IComponent, IAudioProcessor, IConnectionPoint);
}

impl IPluginBaseTrait for GainProcessor {
    unsafe fn initialize(&self, context: *mut FUnknown) -> tresult {
        // SAFETY: the host passes a valid context (or null).
        let host = unsafe { ComRef::from_raw(context) }.and_then(|c| c.cast());
        if let Ok(mut slot) = self.host.lock() {
            *slot = host;
        }
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        if let Ok(mut slot) = self.host.lock() {
            *slot = None;
        }
        kResultOk
    }
}

impl IComponentTrait for GainProcessor {
    unsafe fn getControllerClassId(&self, class_id: *mut TUID) -> tresult {
        // SAFETY: the host passes a writable class id.
        unsafe { *class_id = GAIN_CONTROLLER_CID };
        kResultOk
    }

    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    unsafe fn getBusCount(&self, media: MediaType, _dir: BusDirection) -> i32 {
        i32::from(media == MediaTypes_::kAudio as MediaType)
    }

    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        if media != MediaTypes_::kAudio as MediaType || index != 0 {
            return kInvalidArgument;
        }
        let name = if dir == BusDirections_::kInput as BusDirection {
            "Input"
        } else {
            "Output"
        };
        // SAFETY: the host passes a writable bus info.
        stereo_bus(unsafe { &mut *bus }, media, dir, name);
        kResultOk
    }

    unsafe fn getRoutingInfo(&self, _in: *mut RoutingInfo, _out: *mut RoutingInfo) -> tresult {
        kNotImplemented
    }

    unsafe fn activateBus(&self, _m: MediaType, _d: BusDirection, _i: i32, _s: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setActive(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: the host passes a valid stream.
        match read_f64(&unsafe { read_all(state) }) {
            Some(gain) => {
                self.gain.store(gain.to_bits(), Ordering::Relaxed);
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        let gain = f64::from_bits(self.gain.load(Ordering::Relaxed));
        // SAFETY: the host passes a valid stream.
        unsafe { write_all(state, &gain.to_le_bytes()) }
    }
}

impl IAudioProcessorTrait for GainProcessor {
    unsafe fn setBusArrangements(
        &self,
        inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        // SAFETY: the host passes arrays of the given lengths.
        let stereo = num_ins == 1
            && num_outs == 1
            && unsafe { *inputs == SpeakerArr::kStereo && *outputs == SpeakerArr::kStereo };
        if stereo { kResultTrue } else { kResultFalse }
    }

    unsafe fn getBusArrangement(
        &self,
        _dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: the host passes a writable arrangement.
        unsafe { *arr = SpeakerArr::kStereo };
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, size: i32) -> tresult {
        if size == SymbolicSampleSizes_::kSample32 as i32 {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        0
    }

    unsafe fn setupProcessing(&self, _setup: *mut ProcessSetup) -> tresult {
        kResultOk
    }

    unsafe fn setProcessing(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        // SAFETY: the host passes valid process data for the call.
        let Some(data) = (unsafe { data.as_ref() }) else {
            return kInvalidArgument;
        };
        // SAFETY: `data` belongs to this call.
        if let Some(gain) = unsafe { param_change(data) } {
            self.gain.store(gain.to_bits(), Ordering::Relaxed);
        }
        let gain = (f64::from_bits(self.gain.load(Ordering::Relaxed)) * 2.0) as f32;
        // SAFETY: as above; 32-bit buffers of `numSamples`.
        let input = unsafe { channels(data.numSamples, data.numInputs, data.inputs) };
        // SAFETY: as above.
        let output = unsafe { channels(data.numSamples, data.numOutputs, data.outputs) };
        if let (Some([il, ir]), Some([ol, or])) = (input, output) {
            for (o, i) in ol.iter_mut().zip(il.iter()) {
                *o = i * gain;
            }
            for (o, i) in or.iter_mut().zip(ir.iter()) {
                *o = i * gain;
            }
        }
        kResultOk
    }

    unsafe fn getTailSamples(&self) -> u32 {
        0
    }
}

impl IConnectionPointTrait for GainProcessor {
    unsafe fn connect(&self, other: *mut IConnectionPoint) -> tresult {
        let Some(host) = self.host.lock().ok().and_then(|h| h.clone()) else {
            return kResultFalse;
        };
        // SAFETY: the host passes the controller's connection point.
        let Some(other) = (unsafe { ComRef::from_raw(other) }) else {
            return kInvalidArgument;
        };
        // Ask the host for a message, as SDK-based plugins do.
        let mut cid: TUID = IMessage::IID.map(|b| b as c_char);
        let mut iid = cid;
        let mut obj: *mut c_void = ptr::null_mut();
        // SAFETY: the ids and output are valid for the call.
        if unsafe { host.createInstance(&mut cid, &mut iid, &mut obj) } != kResultOk {
            return kResultFalse;
        }
        // SAFETY: the host returned an `IMessage` with one reference, which
        // `ComPtr` now owns.
        let Some(message) = (unsafe { ComPtr::from_raw(obj.cast::<IMessage>()) }) else {
            return kResultFalse;
        };
        // SAFETY: the message is valid; strings are static.
        unsafe {
            message.setMessageID(c"gain-hello".as_ptr());
            if let Some(attributes) = ComRef::from_raw(message.getAttributes()) {
                attributes.setInt(c"value".as_ptr(), 7);
            }
            other.notify(message.as_ptr())
        }
    }

    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn notify(&self, _message: *mut IMessage) -> tresult {
        kResultOk
    }
}

// --- Test Gain: controller ------------------------------------------------

struct GainController {
    gain: Mutex<f64>,
    state: Mutex<Vec<u8>>,
    linked: AtomicBool,
    handler: Mutex<Option<ComPtr<IComponentHandler>>>,
}

impl Class for GainController {
    type Interfaces = (IEditController, IConnectionPoint);
}

impl GainController {
    fn gain(&self) -> f64 {
        self.gain.lock().map_or(0.5, |g| *g)
    }

    fn set_gain(&self, value: f64) {
        if let Ok(mut gain) = self.gain.lock() {
            *gain = value;
        }
    }
}

impl IPluginBaseTrait for GainController {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        if let Ok(mut handler) = self.handler.lock() {
            *handler = None;
        }
        kResultOk
    }
}

impl IEditControllerTrait for GainController {
    unsafe fn setComponentState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: the host passes a valid stream.
        match read_f64(&unsafe { read_all(state) }) {
            Some(gain) => {
                self.set_gain(gain);
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: the host passes a valid stream.
        let data = unsafe { read_all(state) };
        if let Ok(mut stored) = self.state.lock() {
            *stored = data;
        }
        kResultOk
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        let data = self.state.lock().map(|s| s.clone()).unwrap_or_default();
        // SAFETY: the host passes a valid stream.
        unsafe { write_all(state, &data) }
    }

    unsafe fn getParameterCount(&self) -> i32 {
        1
    }

    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        let title = if self.linked.load(Ordering::Relaxed) {
            "Gain"
        } else {
            "Gain (unlinked)"
        };
        // SAFETY: the host passes a writable info.
        param_info(unsafe { &mut *info }, title, 0.5);
        kResultOk
    }

    unsafe fn getParamStringByValue(&self, id: u32, value: f64, string: *mut String128) -> tresult {
        if id != PARAM {
            return kInvalidArgument;
        }
        // SAFETY: the host passes a writable string.
        copy_wstring(&format!("{:.2}x", value * 2.0), unsafe { &mut *string });
        kResultOk
    }

    unsafe fn getParamValueByString(&self, _id: u32, _s: *mut TChar, _v: *mut f64) -> tresult {
        kNotImplemented
    }

    unsafe fn normalizedParamToPlain(&self, _id: u32, value: f64) -> f64 {
        value * 2.0
    }

    unsafe fn plainParamToNormalized(&self, _id: u32, value: f64) -> f64 {
        value / 2.0
    }

    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == PARAM { self.gain() } else { 0.0 }
    }

    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != PARAM {
            return kInvalidArgument;
        }
        self.set_gain(value);
        kResultOk
    }

    unsafe fn setComponentHandler(&self, handler: *mut IComponentHandler) -> tresult {
        // SAFETY: the host passes a valid handler (or null).
        let handler = unsafe { ComRef::from_raw(handler) }.map(|h| h.to_com_ptr());
        if let Ok(mut slot) = self.handler.lock() {
            *slot = handler;
        }
        kResultOk
    }

    unsafe fn createView(&self, name: FIDString) -> *mut IPlugView {
        // SAFETY: the host passes a nul-terminated view name.
        if name.is_null() || unsafe { CStr::from_ptr(name) } != c"editor" {
            return ptr::null_mut();
        }
        ComWrapper::new(GainView {
            frame: Mutex::new(None),
            #[cfg(target_os = "linux")]
            handler: self.handler.lock().ok().and_then(|h| h.clone()),
            #[cfg(target_os = "linux")]
            timer: Mutex::new(None),
        })
        .to_com_ptr::<IPlugView>()
        .map_or(ptr::null_mut(), ComPtr::into_raw)
    }
}

impl IConnectionPointTrait for GainController {
    unsafe fn connect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn disconnect(&self, _other: *mut IConnectionPoint) -> tresult {
        kResultOk
    }

    unsafe fn notify(&self, message: *mut IMessage) -> tresult {
        // SAFETY: the sender passes a valid message for the call.
        let Some(message) = (unsafe { ComRef::from_raw(message) }) else {
            return kInvalidArgument;
        };
        // SAFETY: as above; the id is nul-terminated.
        let id = unsafe { message.getMessageID() };
        // SAFETY: as above.
        if id.is_null() || unsafe { CStr::from_ptr(id) } != c"gain-hello" {
            return kResultFalse;
        }
        let mut value = 0;
        // SAFETY: as above; the output is writable.
        let attributes = unsafe { ComRef::from_raw(message.getAttributes()) };
        // SAFETY: as above.
        let ok = attributes
            .is_some_and(|a| unsafe { a.getInt(c"value".as_ptr(), &mut value) } == kResultOk);
        self.linked.store(ok && value == 7, Ordering::Relaxed);
        kResultOk
    }
}

// --- Test Gain: window ----------------------------------------------------

/// Turns the `Gain` knob once the host runs its timer.
#[cfg(target_os = "linux")]
struct GainTimer {
    handler: Option<ComPtr<IComponentHandler>>,
}

#[cfg(target_os = "linux")]
impl Class for GainTimer {
    type Interfaces = (vst3::Steinberg::Linux::ITimerHandler,);
}

#[cfg(target_os = "linux")]
impl vst3::Steinberg::Linux::ITimerHandlerTrait for GainTimer {
    unsafe fn onTimer(&self) {
        use vst3::Steinberg::Vst::IComponentHandlerTrait;
        if let Some(handler) = &self.handler {
            // SAFETY: the handler stays valid while the controller holds it.
            unsafe {
                handler.beginEdit(PARAM);
                handler.performEdit(PARAM, GAIN_VIEW_EDIT);
                handler.endEdit(PARAM);
            }
        }
    }
}

struct GainView {
    frame: Mutex<Option<ComPtr<IPlugFrame>>>,
    // The window's knob and timer only exist where the view can be embedded.
    #[cfg(target_os = "linux")]
    handler: Option<ComPtr<IComponentHandler>>,
    #[cfg(target_os = "linux")]
    timer: Mutex<Option<ComWrapper<GainTimer>>>,
}

impl Class for GainView {
    type Interfaces = (IPlugView,);
}

impl GainView {
    fn frame(&self) -> Option<ComPtr<IPlugFrame>> {
        self.frame.lock().ok().and_then(|f| f.clone())
    }
}

impl IPlugViewTrait for GainView {
    unsafe fn isPlatformTypeSupported(&self, kind: FIDString) -> tresult {
        // SAFETY: the host passes a nul-terminated platform type.
        let x11 = !kind.is_null() && unsafe { CStr::from_ptr(kind) } == c"X11EmbedWindowID";
        if x11 { kResultTrue } else { kResultFalse }
    }

    unsafe fn attached(&self, parent: *mut c_void, kind: FIDString) -> tresult {
        // SAFETY: as in `isPlatformTypeSupported`.
        if parent.is_null() || unsafe { self.isPlatformTypeSupported(kind) } != kResultTrue {
            return kResultFalse;
        }
        let Some(frame) = self.frame() else {
            return kResultFalse;
        };
        let mut size = ViewRect {
            left: 0,
            top: 0,
            right: GAIN_VIEW_SIZE.0,
            bottom: GAIN_VIEW_SIZE.1,
        };
        // SAFETY: the frame is valid while set; `size` outlives the call. A
        // view passes itself; this one is not reachable as a pointer here,
        // and hosts identify views by their own records.
        unsafe { frame.resizeView(ptr::null_mut(), &mut size) };
        #[cfg(target_os = "linux")]
        {
            use vst3::Steinberg::Linux::{IRunLoop, IRunLoopTrait, ITimerHandler};
            let Some(run_loop) = frame.cast::<IRunLoop>() else {
                return kResultFalse;
            };
            let timer = ComWrapper::new(GainTimer {
                handler: self.handler.clone(),
            });
            let Some(pointer) = timer.as_com_ref::<ITimerHandler>() else {
                return kResultFalse;
            };
            // SAFETY: the run loop and timer are valid; the timer stays
            // alive (held below) until it is unregistered in `removed`.
            if unsafe { run_loop.registerTimer(pointer.as_ptr(), 1) } != kResultOk {
                return kResultFalse;
            }
            if let Ok(mut slot) = self.timer.lock() {
                *slot = Some(timer);
            }
        }
        kResultOk
    }

    unsafe fn removed(&self) -> tresult {
        #[cfg(target_os = "linux")]
        {
            use vst3::Steinberg::Linux::{IRunLoop, IRunLoopTrait, ITimerHandler};
            let timer = self.timer.lock().ok().and_then(|mut t| t.take());
            if let (Some(timer), Some(run_loop)) =
                (timer, self.frame().and_then(|f| f.cast::<IRunLoop>()))
            {
                if let Some(pointer) = timer.as_com_ref::<ITimerHandler>() {
                    // SAFETY: registered in `attached`.
                    unsafe { run_loop.unregisterTimer(pointer.as_ptr()) };
                }
            }
        }
        kResultOk
    }

    unsafe fn onWheel(&self, _distance: f32) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyDown(&self, _key: char16, _code: int16, _mods: int16) -> tresult {
        kResultFalse
    }

    unsafe fn onKeyUp(&self, _key: char16, _code: int16, _mods: int16) -> tresult {
        kResultFalse
    }

    unsafe fn getSize(&self, size: *mut ViewRect) -> tresult {
        // SAFETY: the host passes a writable rectangle.
        let Some(size) = (unsafe { size.as_mut() }) else {
            return kInvalidArgument;
        };
        *size = ViewRect {
            left: 0,
            top: 0,
            right: 300,
            bottom: 200,
        };
        kResultOk
    }

    unsafe fn onSize(&self, _size: *mut ViewRect) -> tresult {
        kResultOk
    }

    unsafe fn onFocus(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setFrame(&self, frame: *mut IPlugFrame) -> tresult {
        // SAFETY: the host passes a valid frame (or null to clear it).
        let frame = unsafe { ComRef::from_raw(frame) }.map(|f| f.to_com_ptr());
        if let Ok(mut slot) = self.frame.lock() {
            *slot = frame;
        }
        kResultOk
    }

    unsafe fn canResize(&self) -> tresult {
        kResultFalse
    }

    unsafe fn checkSizeConstraint(&self, _rect: *mut ViewRect) -> tresult {
        kResultOk
    }
}

// --- Test Tone --------------------------------------------------------------

#[derive(Clone, Copy)]
struct Voice {
    key: i16,
    velocity: f32,
    phase: f32,
}

struct ToneState {
    voices: Vec<Voice>,
    rate: f64,
}

struct Tone {
    /// Normalized level, as `f64` bits.
    level: AtomicU64,
    audio: Mutex<ToneState>,
}

impl Class for Tone {
    type Interfaces = (IComponent, IAudioProcessor, IEditController);
}

impl Tone {
    fn level(&self) -> f64 {
        f64::from_bits(self.level.load(Ordering::Relaxed))
    }

    fn set_level(&self, value: f64) {
        self.level.store(value.to_bits(), Ordering::Relaxed);
    }
}

impl IPluginBaseTrait for Tone {
    unsafe fn initialize(&self, _context: *mut FUnknown) -> tresult {
        kResultOk
    }

    unsafe fn terminate(&self) -> tresult {
        kResultOk
    }
}

impl IComponentTrait for Tone {
    unsafe fn getControllerClassId(&self, _class_id: *mut TUID) -> tresult {
        kNotImplemented
    }

    unsafe fn setIoMode(&self, _mode: IoMode) -> tresult {
        kResultOk
    }

    unsafe fn getBusCount(&self, media: MediaType, dir: BusDirection) -> i32 {
        let output = dir == BusDirections_::kOutput as BusDirection;
        let audio = media == MediaTypes_::kAudio as MediaType;
        let event = media == MediaTypes_::kEvent as MediaType;
        i32::from((audio && output) || (event && !output))
    }

    unsafe fn getBusInfo(
        &self,
        media: MediaType,
        dir: BusDirection,
        index: i32,
        bus: *mut BusInfo,
    ) -> tresult {
        // SAFETY: plain query on this object.
        if index != 0 || unsafe { self.getBusCount(media, dir) } == 0 {
            return kInvalidArgument;
        }
        let name = if media == MediaTypes_::kAudio as MediaType {
            "Output"
        } else {
            "Notes"
        };
        // SAFETY: the host passes a writable bus info.
        stereo_bus(unsafe { &mut *bus }, media, dir, name);
        kResultOk
    }

    unsafe fn getRoutingInfo(&self, _in: *mut RoutingInfo, _out: *mut RoutingInfo) -> tresult {
        kNotImplemented
    }

    unsafe fn activateBus(&self, _m: MediaType, _d: BusDirection, _i: i32, _s: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setActive(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn setState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: the host passes a valid stream.
        match read_f64(&unsafe { read_all(state) }) {
            Some(level) => {
                self.set_level(level);
                kResultOk
            }
            None => kResultFalse,
        }
    }

    unsafe fn getState(&self, state: *mut IBStream) -> tresult {
        // SAFETY: the host passes a valid stream.
        unsafe { write_all(state, &self.level().to_le_bytes()) }
    }
}

impl IAudioProcessorTrait for Tone {
    unsafe fn setBusArrangements(
        &self,
        _inputs: *mut SpeakerArrangement,
        num_ins: i32,
        outputs: *mut SpeakerArrangement,
        num_outs: i32,
    ) -> tresult {
        // SAFETY: the host passes an array of `num_outs` arrangements.
        let stereo = num_ins == 0 && num_outs == 1 && unsafe { *outputs == SpeakerArr::kStereo };
        if stereo { kResultTrue } else { kResultFalse }
    }

    unsafe fn getBusArrangement(
        &self,
        _dir: BusDirection,
        index: i32,
        arr: *mut SpeakerArrangement,
    ) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: the host passes a writable arrangement.
        unsafe { *arr = SpeakerArr::kStereo };
        kResultOk
    }

    unsafe fn canProcessSampleSize(&self, size: i32) -> tresult {
        if size == SymbolicSampleSizes_::kSample32 as i32 {
            kResultTrue
        } else {
            kResultFalse
        }
    }

    unsafe fn getLatencySamples(&self) -> u32 {
        0
    }

    unsafe fn setupProcessing(&self, setup: *mut ProcessSetup) -> tresult {
        // SAFETY: the host passes a valid setup.
        let Some(setup) = (unsafe { setup.as_ref() }) else {
            return kInvalidArgument;
        };
        if let Ok(mut audio) = self.audio.lock() {
            audio.rate = setup.sampleRate;
        }
        kResultOk
    }

    unsafe fn setProcessing(&self, _state: TBool) -> tresult {
        kResultOk
    }

    unsafe fn process(&self, data: *mut ProcessData) -> tresult {
        // SAFETY: the host passes valid process data for the call.
        let Some(data) = (unsafe { data.as_ref() }) else {
            return kInvalidArgument;
        };
        // SAFETY: `data` belongs to this call.
        if let Some(level) = unsafe { param_change(data) } {
            self.set_level(level);
        }
        let Ok(mut audio) = self.audio.lock() else {
            return kResultFalse;
        };
        // SAFETY: as above.
        let events = unsafe { ComRef::from_raw(data.inputEvents) };
        // SAFETY: as above.
        let count = events.map_or(0, |e| unsafe { e.getEventCount() });
        let mut next = 0;
        let level = self.level() as f32 * TONE_PEAK;
        let step = (440.0 / audio.rate.max(1.0)) as f32;
        let mut report = false;
        // SAFETY: as above; 32-bit buffers of `numSamples`.
        let Some([left, right]) =
            (unsafe { channels(data.numSamples, data.numOutputs, data.outputs) })
        else {
            return kResultOk;
        };
        for (i, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
            while next < count {
                // SAFETY: an all-zero event is a valid value of this plain
                // struct.
                let mut event: Event = unsafe { std::mem::zeroed() };
                let got = events.is_some_and(|e| {
                    // SAFETY: `next` is below the count; the output is
                    // writable; the list is valid for the call.
                    unsafe { e.getEvent(next, &mut event) == kResultOk }
                });
                if got && usize::try_from(event.sampleOffset).unwrap_or(0) > i {
                    break;
                }
                next += 1;
                if !got {
                    continue;
                }
                let kind = event.r#type as u32;
                if kind == Event_::EventTypes_::kNoteOnEvent as u32 {
                    // SAFETY: the type says the union holds a note-on.
                    let on = unsafe { event.__field0.noteOn };
                    if on.pitch == 127 {
                        report = true;
                    }
                    audio.voices.push(Voice {
                        key: on.pitch,
                        velocity: on.velocity,
                        phase: 0.0,
                    });
                } else if kind == Event_::EventTypes_::kNoteOffEvent as u32 {
                    // SAFETY: the type says the union holds a note-off.
                    let off = unsafe { event.__field0.noteOff };
                    audio.voices.retain(|v| v.key != off.pitch);
                }
            }
            let mut sum = 0.0;
            for voice in &mut audio.voices {
                let ratio = 2.0_f32.powf((f32::from(voice.key) - 69.0) / 12.0);
                sum += (voice.phase * std::f32::consts::TAU).sin() * level * voice.velocity;
                voice.phase = (voice.phase + step * ratio).fract();
            }
            *l = sum;
            *r = sum;
        }
        if report {
            // SAFETY: as above.
            if let Some(out) = unsafe { ComRef::from_raw(data.outputParameterChanges) } {
                let mut index = 0;
                // SAFETY: the id and output are valid for the call.
                let queue = unsafe { out.addParameterData(&PARAM, &mut index) };
                // SAFETY: the host returns a valid queue (or null).
                if let Some(queue) = unsafe { ComRef::from_raw(queue) } {
                    let mut point = 0;
                    // SAFETY: as above.
                    unsafe { queue.addPoint(0, 0.25, &mut point) };
                }
            }
        }
        kResultOk
    }

    unsafe fn getTailSamples(&self) -> u32 {
        0
    }
}

impl IEditControllerTrait for Tone {
    unsafe fn setComponentState(&self, _state: *mut IBStream) -> tresult {
        // The component and controller are one object: nothing to copy.
        kResultOk
    }

    unsafe fn setState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getState(&self, _state: *mut IBStream) -> tresult {
        kResultOk
    }

    unsafe fn getParameterCount(&self) -> i32 {
        1
    }

    unsafe fn getParameterInfo(&self, index: i32, info: *mut ParameterInfo) -> tresult {
        if index != 0 {
            return kInvalidArgument;
        }
        // SAFETY: the host passes a writable info.
        param_info(unsafe { &mut *info }, "Level", 1.0);
        kResultOk
    }

    unsafe fn getParamStringByValue(
        &self,
        _id: u32,
        value: f64,
        string: *mut String128,
    ) -> tresult {
        // SAFETY: the host passes a writable string.
        copy_wstring(&format!("{:.0}%", value * 100.0), unsafe { &mut *string });
        kResultOk
    }

    unsafe fn getParamValueByString(&self, _id: u32, _s: *mut TChar, _v: *mut f64) -> tresult {
        kNotImplemented
    }

    unsafe fn normalizedParamToPlain(&self, _id: u32, value: f64) -> f64 {
        value
    }

    unsafe fn plainParamToNormalized(&self, _id: u32, value: f64) -> f64 {
        value
    }

    unsafe fn getParamNormalized(&self, id: u32) -> f64 {
        if id == PARAM { self.level() } else { 0.0 }
    }

    unsafe fn setParamNormalized(&self, id: u32, value: f64) -> tresult {
        if id != PARAM {
            return kInvalidArgument;
        }
        self.set_level(value);
        kResultOk
    }

    unsafe fn setComponentHandler(&self, _handler: *mut IComponentHandler) -> tresult {
        kResultOk
    }

    unsafe fn createView(&self, _name: FIDString) -> *mut IPlugView {
        ptr::null_mut()
    }
}

// --- factory ----------------------------------------------------------------

struct Factory;

impl Class for Factory {
    type Interfaces = (IPluginFactory2,);
}

/// (class id, category, name, sub-categories)
const CLASSES: [(TUID, &str, &str, &str); 3] = [
    (GAIN_CID, "Audio Module Class", "Test Gain", "Fx"),
    (
        GAIN_CONTROLLER_CID,
        "Component Controller Class",
        "Test Gain",
        "",
    ),
    (
        TONE_CID,
        "Audio Module Class",
        "Test Tone",
        "Instrument|Synth",
    ),
];

impl IPluginFactoryTrait for Factory {
    unsafe fn getFactoryInfo(&self, info: *mut PFactoryInfo) -> tresult {
        // SAFETY: the host passes a writable info.
        let Some(info) = (unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };
        copy_cstring("MusicCraft tests", &mut info.vendor);
        copy_cstring("", &mut info.url);
        copy_cstring("", &mut info.email);
        info.flags = PFactoryInfo_::FactoryFlags_::kUnicode as i32;
        kResultOk
    }

    unsafe fn countClasses(&self) -> i32 {
        CLASSES.len() as i32
    }

    unsafe fn getClassInfo(&self, index: i32, info: *mut PClassInfo) -> tresult {
        let Some((cid, category, name, _)) =
            usize::try_from(index).ok().and_then(|i| CLASSES.get(i))
        else {
            return kInvalidArgument;
        };
        // SAFETY: the host passes a writable info.
        let Some(info) = (unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };
        info.cid = *cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as i32;
        copy_cstring(category, &mut info.category);
        copy_cstring(name, &mut info.name);
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        cid: FIDString,
        iid: FIDString,
        obj: *mut *mut c_void,
    ) -> tresult {
        if cid.is_null() || iid.is_null() || obj.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: class ids are 16 bytes.
        let cid = unsafe { *cid.cast::<TUID>() };
        let instance = if cid == GAIN_CID {
            ComWrapper::new(GainProcessor {
                gain: AtomicU64::new(0.5f64.to_bits()),
                host: Mutex::new(None),
            })
            .to_com_ptr::<FUnknown>()
        } else if cid == GAIN_CONTROLLER_CID {
            ComWrapper::new(GainController {
                gain: Mutex::new(0.5),
                state: Mutex::new(CONTROLLER_STATE.to_vec()),
                linked: AtomicBool::new(false),
                handler: Mutex::new(None),
            })
            .to_com_ptr::<FUnknown>()
        } else if cid == TONE_CID {
            ComWrapper::new(Tone {
                level: AtomicU64::new(1.0f64.to_bits()),
                audio: Mutex::new(ToneState {
                    voices: Vec::with_capacity(128),
                    rate: 48_000.0,
                }),
            })
            .to_com_ptr::<FUnknown>()
        } else {
            None
        };
        let Some(instance) = instance else {
            return kInvalidArgument;
        };
        let ptr = instance.as_ptr();
        // SAFETY: `ptr` is a live object; `queryInterface` adds the
        // reference the caller receives.
        unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid.cast_mut().cast(), obj) }
    }
}

impl IPluginFactory2Trait for Factory {
    unsafe fn getClassInfo2(&self, index: i32, info: *mut PClassInfo2) -> tresult {
        let Some((cid, category, name, sub)) =
            usize::try_from(index).ok().and_then(|i| CLASSES.get(i))
        else {
            return kInvalidArgument;
        };
        // SAFETY: the host passes a writable info.
        let Some(info) = (unsafe { info.as_mut() }) else {
            return kInvalidArgument;
        };
        info.cid = *cid;
        info.cardinality = PClassInfo_::ClassCardinality_::kManyInstances as i32;
        copy_cstring(category, &mut info.category);
        copy_cstring(name, &mut info.name);
        info.classFlags = 0;
        copy_cstring(sub, &mut info.subCategories);
        copy_cstring("MusicCraft tests", &mut info.vendor);
        copy_cstring("1.0.0", &mut info.version);
        copy_cstring("VST 3.7", &mut info.sdkVersion);
        kResultOk
    }
}

// --- module entry points -----------------------------------------------------

/// Linux module entry.
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "system" fn ModuleEntry(_library: *mut c_void) -> bool {
    true
}

/// Linux module exit.
#[cfg(target_os = "linux")]
#[unsafe(no_mangle)]
pub extern "system" fn ModuleExit() -> bool {
    true
}

/// Windows module entry.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn InitDll() -> bool {
    true
}

/// Windows module exit.
#[cfg(windows)]
#[unsafe(no_mangle)]
pub extern "system" fn ExitDll() -> bool {
    true
}

/// macOS bundle entry.
#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub extern "system" fn BundleEntry(_bundle: *mut c_void) -> bool {
    true
}

/// macOS bundle exit.
#[cfg(target_os = "macos")]
#[unsafe(no_mangle)]
pub extern "system" fn BundleExit() -> bool {
    true
}

/// The module's plugin factory, with one reference for the caller.
#[unsafe(no_mangle)]
pub extern "system" fn GetPluginFactory() -> *mut IPluginFactory {
    ComWrapper::new(Factory)
        .to_com_ptr::<IPluginFactory>()
        .map_or(ptr::null_mut(), ComPtr::into_raw)
}
