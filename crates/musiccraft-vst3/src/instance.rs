//! A live plugin instance: creation, connection, activation and the
//! main-thread side.

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use musiccraft_render::{LivePlugin, PluginEvent, PluginInstance, PluginParam};
use rtrb::{Consumer, Producer, RingBuffer};
use vst3::Steinberg::Vst::{
    BusDirections_, BusInfo, IAudioProcessor, IAudioProcessorTrait, IComponent, IComponentHandler,
    IComponentTrait, IConnectionPoint, IConnectionPointTrait, IEditController,
    IEditControllerTrait, IHostApplication, MediaTypes_, ParameterInfo, ParameterInfo_,
    ProcessModes_, ProcessSetup, RestartFlags_, SpeakerArr, SpeakerArrangement, String128,
    SymbolicSampleSizes_,
};
use vst3::Steinberg::{
    FUnknown, IBStream, IPluginBaseTrait, IPluginFactoryTrait, TUID, kResultOk, kResultTrue,
};
use vst3::{ComPtr, ComWrapper, Interface};

use crate::editor::Editor;
use crate::host::{ComponentHandler, HandlerEvent, HostApplication, MemoryStream};
use crate::module::{Module, wide};
use crate::processor::{Links, Vst3Processor};

/// Changes either half can report per poll before older ones drop.
const QUEUE_CAPACITY: usize = 1024;
/// The first bytes of a saved state.
const STATE_MAGIC: &[u8; 4] = b"MCV3";
const STATE_VERSION: u32 = 1;

/// What both halves of a plugin share. The plugin is torn down when the last
/// `Arc` goes, which is always on the main thread: the audio half comes back
/// through the engine's garbage queue.
pub(crate) struct Shared {
    pub component: ComPtr<IComponent>,
    pub processor: ComPtr<IAudioProcessor>,
    /// The edit controller: the component itself, or a separate object.
    pub controller: Option<ComPtr<IEditController>>,
    /// Whether the controller is a separate object (initialized on its own).
    separate: bool,
    /// The component's and controller's connection points, when connected.
    connections: Option<(ComPtr<IConnectionPoint>, ComPtr<IConnectionPoint>)>,
    /// Steps of each stepped parameter, for converting host values.
    pub steps: Vec<(u32, i32)>,
    /// Set by the audio half between the two `setProcessing` calls.
    pub processing: AtomicBool,
    active: bool,
    pub handler: ComWrapper<ComponentHandler>,
    host: ComWrapper<HostApplication>,
    // Dropped last (field order): the module must outlive every object.
    _module: Arc<Module>,
}

impl Drop for Shared {
    fn drop(&mut self) {
        if self.processing.swap(false, Ordering::AcqRel) {
            // The audio half was dropped without stopping (the engine itself
            // went away), so no audio thread is using the plugin any more and
            // this thread stands in for it.
            // SAFETY: processing was started and nothing else processes.
            unsafe { self.processor.setProcessing(0) };
        }
        if self.active {
            // SAFETY: activated in `create`; main thread.
            unsafe { self.component.setActive(0) };
        }
        if let Some((component, controller)) = self.connections.take() {
            // SAFETY: connected in `create`; main thread.
            unsafe {
                component.disconnect(controller.as_ptr());
                controller.disconnect(component.as_ptr());
            }
        }
        if let Some(controller) = &self.controller {
            // SAFETY: main thread; clearing the handler before teardown.
            unsafe { controller.setComponentHandler(ptr::null_mut()) };
            if self.separate {
                // SAFETY: initialized in `create`; main thread.
                unsafe { controller.terminate() };
            }
        }
        // SAFETY: initialized in `create`; last call before release.
        unsafe { self.component.terminate() };
        let _ = &self.host;
    }
}

/// Creates an object of class `cid` and asks for interface `I`.
fn create_object<I: Interface>(module: &Module, cid: &TUID) -> Option<ComPtr<I>> {
    let factory = module.factory()?;
    let iid: TUID = I::IID.map(|b| b as std::ffi::c_char);
    let mut obj: *mut c_void = ptr::null_mut();
    // SAFETY: main thread; ids are 16 bytes; `obj` is writable.
    let result = unsafe { factory.createInstance(cid.as_ptr(), iid.as_ptr(), &mut obj) };
    if result != kResultOk {
        return None;
    }
    // SAFETY: the factory returned interface `I` with one reference, which
    // `ComPtr` takes over.
    unsafe { ComPtr::from_raw(obj.cast::<I>()) }
}

/// Runs `call` with a stream holding `data`.
fn with_stream(data: &[u8], call: impl FnOnce(*mut IBStream) -> i32) -> bool {
    let stream = ComWrapper::new(MemoryStream::with(data));
    let Some(pointer) = stream.as_com_ref::<IBStream>() else {
        return false;
    };
    call(pointer.as_ptr()) == kResultOk
}

/// Runs `call` with an empty stream and returns what was written.
fn read_stream(call: impl FnOnce(*mut IBStream) -> i32) -> Option<Vec<u8>> {
    let stream = ComWrapper::new(MemoryStream::default());
    let pointer = stream.as_com_ref::<IBStream>()?;
    (call(pointer.as_ptr()) == kResultOk).then(|| stream.bytes())
}

/// The component's and controller's parts of a saved state.
fn split_state(data: &[u8]) -> Option<(&[u8], &[u8])> {
    let rest = data.strip_prefix(STATE_MAGIC)?;
    let version = u32::from_le_bytes(rest.get(..4)?.try_into().ok()?);
    if version != STATE_VERSION {
        return None;
    }
    let mut rest = rest.get(4..)?;
    let mut part = || {
        let len = usize::try_from(u64::from_le_bytes(rest.get(..8)?.try_into().ok()?)).ok()?;
        let bytes = rest.get(8..8 + len)?;
        rest = rest.get(8 + len..)?;
        Some(bytes)
    };
    Some((part()?, part()?))
}

fn join_state(component: &[u8], controller: &[u8]) -> Vec<u8> {
    let mut out = STATE_MAGIC.to_vec();
    out.extend_from_slice(&STATE_VERSION.to_le_bytes());
    for part in [component, controller] {
        out.extend_from_slice(&(part.len() as u64).to_le_bytes());
        out.extend_from_slice(part);
    }
    out
}

/// Channel counts of the audio buses in one direction, main bus first, after
/// asking for stereo main buses.
fn channel_counts(component: &ComPtr<IComponent>, input: bool) -> Vec<u32> {
    let dir = if input {
        BusDirections_::kInput
    } else {
        BusDirections_::kOutput
    } as i32;
    let audio = MediaTypes_::kAudio as i32;
    // SAFETY: main thread.
    let count = unsafe { component.getBusCount(audio, dir) };
    (0..count)
        .map(|i| {
            // SAFETY: an all-zero info is a valid value of this plain struct.
            let mut info: BusInfo = unsafe { std::mem::zeroed() };
            // SAFETY: `i` is below the count; `info` is writable.
            let ok = unsafe { component.getBusInfo(audio, dir, i, &mut info) } == kResultOk;
            if ok {
                u32::try_from(info.channelCount).unwrap_or(0).min(32)
            } else {
                0
            }
        })
        .collect()
}

/// Asks for stereo main buses, keeping other buses' arrangements, and returns
/// the channel counts the plugin settled on.
fn arrange(
    component: &ComPtr<IComponent>,
    processor: &ComPtr<IAudioProcessor>,
) -> (Vec<u32>, Vec<u32>) {
    let current = |input: bool, count: usize| -> Vec<SpeakerArrangement> {
        let dir = if input {
            BusDirections_::kInput
        } else {
            BusDirections_::kOutput
        } as i32;
        (0..count)
            .map(|i| {
                let mut arrangement = 0;
                // SAFETY: main thread; `arrangement` is writable.
                unsafe { processor.getBusArrangement(dir, i as i32, &mut arrangement) };
                arrangement
            })
            .collect()
    };
    let (ins, outs) = (
        channel_counts(component, true),
        channel_counts(component, false),
    );
    let mut inputs = current(true, ins.len());
    let mut outputs = current(false, outs.len());
    if let Some(main) = inputs.first_mut() {
        *main = SpeakerArr::kStereo;
    }
    if let Some(main) = outputs.first_mut() {
        *main = SpeakerArr::kStereo;
    }
    // SAFETY: main thread, inactive; the arrays hold the given counts.
    let ok = unsafe {
        processor.setBusArrangements(
            inputs.as_mut_ptr(),
            inputs.len() as i32,
            outputs.as_mut_ptr(),
            outputs.len() as i32,
        )
    } == kResultTrue;
    if ok {
        let count = |a: &SpeakerArrangement| a.count_ones().min(32);
        (
            inputs.iter().map(count).collect(),
            outputs.iter().map(count).collect(),
        )
    } else {
        (ins, outs)
    }
}

/// Activates the main audio buses and the first event input bus.
fn activate_buses(component: &ComPtr<IComponent>, inputs: usize, outputs: usize) -> bool {
    let audio = MediaTypes_::kAudio as i32;
    let event = MediaTypes_::kEvent as i32;
    let (input, output) = (
        BusDirections_::kInput as i32,
        BusDirections_::kOutput as i32,
    );
    // SAFETY: main thread, inactive.
    unsafe {
        if inputs > 0 {
            component.activateBus(audio, input, 0, 1);
        }
        if outputs > 0 {
            component.activateBus(audio, output, 0, 1);
        }
        let notes = component.getBusCount(event, input) > 0;
        if notes {
            component.activateBus(event, input, 0, 1);
        }
        notes
    }
}

/// Creates, initializes, connects, restores and activates plugin class `cid`.
pub(crate) fn create(
    module: Arc<Module>,
    cid: &TUID,
    sample_rate: u32,
    max_frames: u32,
    state: Option<&[u8]>,
) -> Result<LivePlugin, String> {
    let (instance, processor) = build(module, cid, sample_rate, max_frames, state)?;
    Ok(LivePlugin {
        instance: Box::new(instance),
        processor: Box::new(processor),
    })
}

/// [`create`], returning the concrete halves.
pub(crate) fn build(
    module: Arc<Module>,
    cid: &TUID,
    sample_rate: u32,
    max_frames: u32,
    state: Option<&[u8]>,
) -> Result<(Vst3Instance, Vst3Processor), String> {
    let component =
        create_object::<IComponent>(&module, cid).ok_or("the plugin refused to start")?;
    let host = ComWrapper::new(HostApplication);
    let context = host
        .as_com_ref::<IHostApplication>()
        .map_or(ptr::null_mut(), |r| r.as_ptr().cast::<FUnknown>());
    // SAFETY: main thread; the host context lives in `Shared` for as long as
    // the plugin.
    if unsafe { component.initialize(context) } != kResultOk {
        return Err("the plugin failed to initialize".to_owned());
    }
    let processor = match component.cast::<IAudioProcessor>() {
        Some(processor) => processor,
        None => {
            // SAFETY: initialized above; main thread.
            unsafe { component.terminate() };
            return Err("the plugin does not process audio".to_owned());
        }
    };
    let (controller, separate) = match component.cast::<IEditController>() {
        Some(controller) => (Some(controller), false),
        None => {
            let mut controller_id: TUID = [0; 16];
            // SAFETY: main thread; the id is writable.
            let named = unsafe { component.getControllerClassId(&mut controller_id) } == kResultOk;
            let controller = named
                .then(|| create_object::<IEditController>(&module, &controller_id))
                .flatten()
                // SAFETY: main thread; the context outlives the controller.
                .filter(|c| unsafe { c.initialize(context) } == kResultOk);
            (controller, true)
        }
    };
    let connections = if separate {
        controller.as_ref().and_then(|controller| {
            let a = component.cast::<IConnectionPoint>()?;
            let b = controller.cast::<IConnectionPoint>()?;
            // SAFETY: main thread; both objects are initialized.
            unsafe {
                a.connect(b.as_ptr());
                b.connect(a.as_ptr());
            }
            Some((a, b))
        })
    } else {
        None
    };
    let handler = ComWrapper::new(ComponentHandler::default());
    if let (Some(controller), Some(pointer)) =
        (&controller, handler.as_com_ref::<IComponentHandler>())
    {
        // SAFETY: main thread; the handler lives in `Shared`.
        unsafe { controller.setComponentHandler(pointer.as_ptr()) };
    }
    let mut shared = Shared {
        component,
        processor,
        controller,
        separate,
        connections,
        steps: Vec::new(),
        processing: AtomicBool::new(false),
        active: false,
        handler,
        host,
        _module: module,
    };
    // From here on, dropping `shared` tears everything down.
    restore(&shared, state);
    let (inputs, outputs) = arrange(&shared.component, &shared.processor);
    let notes = activate_buses(&shared.component, inputs.len(), outputs.len());
    // SAFETY: main thread, inactive.
    let float = unsafe {
        shared
            .processor
            .canProcessSampleSize(SymbolicSampleSizes_::kSample32 as i32)
    } == kResultTrue;
    if !float {
        return Err("the plugin cannot process 32-bit audio".to_owned());
    }
    let mut setup = ProcessSetup {
        processMode: ProcessModes_::kRealtime as i32,
        symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
        maxSamplesPerBlock: max_frames as i32,
        sampleRate: f64::from(sample_rate),
    };
    // SAFETY: main thread, inactive; `setup` outlives the call.
    if unsafe { shared.processor.setupProcessing(&mut setup) } != kResultOk {
        return Err("the plugin rejected the audio setup".to_owned());
    }
    // SAFETY: main thread; set up above.
    if unsafe { shared.component.setActive(1) } != kResultOk {
        return Err("the plugin failed to activate".to_owned());
    }
    shared.active = true;
    shared.steps = param_infos(&shared)
        .iter()
        .filter(|p| p.stepCount > 0)
        .map(|p| (p.id, p.stepCount))
        .collect();

    let shared = Arc::new(shared);
    let (reported, reported_rx) = RingBuffer::new(QUEUE_CAPACITY);
    let (host_set, host_set_rx) = RingBuffer::new(QUEUE_CAPACITY);
    let (edits_tx, edits) = RingBuffer::new(QUEUE_CAPACITY);
    let processor = Vst3Processor::new(
        Arc::clone(&shared),
        &inputs,
        &outputs,
        notes,
        max_frames as usize,
        f64::from(sample_rate),
        Links {
            reported,
            host_set,
            edits,
        },
    );
    let instance = Vst3Instance {
        shared,
        reported: reported_rx,
        host_set: Mutex::new(host_set_rx),
        edits: edits_tx,
        editor: Editor::default(),
    };
    Ok((instance, processor))
}

/// Gives the plugin its saved state, or (without one) lets a separate
/// controller hear the component's own state.
fn restore(shared: &Shared, state: Option<&[u8]>) {
    let parts = state.and_then(split_state);
    if state.is_some() && parts.is_none() {
        eprintln!("VST3 plugin: the saved state is not in MusicCraft's format; ignored");
    }
    let component_state = match parts {
        Some((component, _)) => {
            // SAFETY: main thread; the stream outlives the call.
            let ok = with_stream(component, |s| unsafe { shared.component.setState(s) });
            if !ok {
                eprintln!("VST3 plugin: its saved state could not be restored");
            }
            Some(component.to_vec())
        }
        // SAFETY: main thread; the stream outlives the call.
        None => read_stream(|s| unsafe { shared.component.getState(s) }),
    };
    let Some(controller) = &shared.controller else {
        return;
    };
    if let Some(bytes) = &component_state {
        // SAFETY: main thread; the stream outlives the call.
        with_stream(bytes, |s| unsafe { controller.setComponentState(s) });
    }
    if let Some((_, controller_state)) = parts {
        if !controller_state.is_empty() {
            // SAFETY: main thread; the stream outlives the call.
            with_stream(controller_state, |s| unsafe { controller.setState(s) });
        }
    }
}

/// Every parameter's info from the controller.
fn param_infos(shared: &Shared) -> Vec<ParameterInfo> {
    let Some(controller) = &shared.controller else {
        return Vec::new();
    };
    // SAFETY: main thread.
    let count = unsafe { controller.getParameterCount() };
    (0..count)
        .filter_map(|i| {
            // SAFETY: an all-zero info is a valid value of this plain struct.
            let mut info: ParameterInfo = unsafe { std::mem::zeroed() };
            // SAFETY: `i` is below the count; `info` is writable.
            (unsafe { controller.getParameterInfo(i, &mut info) } == kResultOk).then_some(info)
        })
        .collect()
}

/// The main-thread side of a plugin.
pub(crate) struct Vst3Instance {
    shared: Arc<Shared>,
    /// Parameter changes the processor reported (normalized).
    reported: Consumer<(u32, f64)>,
    /// Values the host set, not yet passed to the controller (normalized).
    host_set: Mutex<Consumer<(u32, f64)>>,
    /// Edits from the plugin's window, for the processor (normalized).
    edits: Producer<(u32, f64)>,
    editor: Editor,
}

impl std::fmt::Debug for Vst3Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vst3Instance").finish_non_exhaustive()
    }
}

impl Vst3Instance {
    /// Steps of parameter `id` (0 for continuous ones).
    fn steps(&self, id: u32) -> i32 {
        self.shared
            .steps
            .iter()
            .find(|(p, _)| *p == id)
            .map_or(0, |(_, s)| *s)
    }

    /// A normalized value in MusicCraft's units for parameter `id`.
    fn to_host(&self, id: u32, normalized: f64) -> f64 {
        match self.steps(id) {
            0 => normalized,
            steps => (normalized * f64::from(steps)).round(),
        }
    }

    /// Passes values the host set to the controller.
    fn sync_controller(&self) {
        let Some(controller) = &self.shared.controller else {
            return;
        };
        let mut queue = self.host_set.lock().unwrap_or_else(PoisonError::into_inner);
        while let Ok((id, value)) = queue.pop() {
            // SAFETY: main thread.
            unsafe { controller.setParamNormalized(id, value) };
        }
    }
}

impl PluginInstance for Vst3Instance {
    fn params(&self) -> Vec<PluginParam> {
        self.sync_controller();
        param_infos(&self.shared)
            .iter()
            .map(|info| {
                use ParameterInfo_::ParameterFlags_ as F;
                let steps = info.stepCount.max(0);
                let max = if steps > 0 { f64::from(steps) } else { 1.0 };
                PluginParam {
                    id: info.id,
                    name: wide(&info.title),
                    module: String::new(),
                    min: 0.0,
                    max,
                    default: if steps > 0 {
                        (info.defaultNormalizedValue * max).round()
                    } else {
                        info.defaultNormalizedValue
                    },
                    stepped: steps > 0,
                    hidden: info.flags & F::kIsHidden != 0,
                    read_only: info.flags & F::kIsReadOnly != 0,
                }
            })
            .collect()
    }

    fn param_value(&self, id: u32) -> Option<f64> {
        self.sync_controller();
        let controller = self.shared.controller.as_ref()?;
        // SAFETY: main thread.
        let normalized = unsafe { controller.getParamNormalized(id) };
        Some(self.to_host(id, normalized))
    }

    fn format_param(&self, id: u32, value: f64) -> Option<String> {
        let controller = self.shared.controller.as_ref()?;
        let normalized = match self.steps(id) {
            0 => value,
            steps => value / f64::from(steps),
        };
        let mut text: String128 = [0; 128];
        // SAFETY: main thread; `text` is writable.
        let ok =
            unsafe { controller.getParamStringByValue(id, normalized, &mut text) } == kResultOk;
        ok.then(|| {
            let units: Vec<u16> = text.iter().map(|c| *c as u16).collect();
            wide(&units)
        })
    }

    fn save_state(&mut self) -> Option<Vec<u8>> {
        self.sync_controller();
        // SAFETY: main thread; the stream outlives the call.
        let component = read_stream(|s| unsafe { self.shared.component.getState(s) })?;
        let controller = match (&self.shared.controller, self.shared.separate) {
            (Some(controller), true) => {
                // SAFETY: main thread; the stream outlives the call.
                read_stream(|s| unsafe { controller.getState(s) }).unwrap_or_default()
            }
            _ => Vec::new(),
        };
        Some(join_state(&component, &controller))
    }

    fn poll(&mut self) -> Vec<PluginEvent> {
        self.sync_controller();
        let mut events = Vec::new();
        self.editor.run_loop();
        for event in self.shared.handler.take() {
            match event {
                HandlerEvent::Edit(id, value) => {
                    let _ = self.edits.push((id, value));
                    events.push(PluginEvent::Param {
                        id,
                        value: self.to_host(id, value),
                    });
                }
                HandlerEvent::Restart(flags) => {
                    use RestartFlags_ as R;
                    if flags & (R::kParamValuesChanged | R::kParamTitlesChanged) != 0 {
                        events.push(PluginEvent::ParamsChanged);
                    }
                    if flags & !(R::kParamValuesChanged | R::kParamTitlesChanged) != 0 {
                        eprintln!("VST3 plugin asked for a restart ({flags:#x}); ignored");
                    }
                }
            }
        }
        while let Ok((id, value)) = self.reported.pop() {
            if let Some(controller) = &self.shared.controller {
                // SAFETY: main thread.
                unsafe { controller.setParamNormalized(id, value) };
            }
            events.push(PluginEvent::Param {
                id,
                value: self.to_host(id, value),
            });
        }
        if self.editor.poll(&self.shared) {
            events.push(PluginEvent::EditorClosed);
        }
        events
    }

    fn has_editor(&self) -> bool {
        Editor::supported(&self.shared)
    }

    fn editor_open(&self) -> bool {
        self.editor.is_open()
    }

    fn set_editor_open(&mut self, open: bool, title: &str) -> Result<(), String> {
        if open {
            self.editor.open(&self.shared, title)
        } else {
            self.editor.close();
            Ok(())
        }
    }
}

impl Vst3Instance {
    /// Attaches the plugin's view to an existing X11 parent window (tests).
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn attach_for_test(&mut self, parent: u64) -> Result<(), String> {
        self.editor.attach(&self.shared, parent, None)
    }

    /// The size the plugin last asked its frame for.
    #[cfg(all(test, target_os = "linux"))]
    pub(crate) fn requested_size(&self) -> Option<(i32, i32)> {
        self.editor.requested_size()
    }
}

impl Drop for Vst3Instance {
    fn drop(&mut self) {
        self.editor.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_parts_round_trip() {
        let joined = join_state(b"abc", b"");
        assert_eq!(split_state(&joined), Some((&b"abc"[..], &b""[..])));
        assert_eq!(split_state(b"MCV3"), None);
        assert_eq!(split_state(b"other"), None);
        let mut wrong = joined.clone();
        wrong[4] = 9;
        assert_eq!(split_state(&wrong), None, "unknown version");
    }
}
