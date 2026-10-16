//! COM objects the host gives plugins: the host context (which also makes
//! messages and attribute lists), memory streams for state, and the
//! component handler that hears edits made in a plugin's window. All of them
//! are used on the main thread.

use std::collections::BTreeMap;
use std::ffi::{CStr, CString, c_char, c_void};
use std::ptr;
use std::sync::{Mutex, MutexGuard, PoisonError};

use vst3::Steinberg::Vst::{
    IAttributeList, IAttributeListTrait, IComponentHandler, IComponentHandlerTrait,
    IHostApplication, IHostApplicationTrait, IMessage, IMessageTrait, String128, TChar,
};
use vst3::Steinberg::{
    FIDString, IBStream, IBStream_, IBStreamTrait, TUID, kInvalidArgument, kNoInterface,
    kResultFalse, kResultOk, tresult,
};
use vst3::{Class, ComWrapper, Interface};

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether a class or interface id passed by a plugin names interface `I`.
///
/// # Safety
///
/// `id` must be null or point at 16 readable bytes.
unsafe fn is<I: Interface>(id: *const TUID) -> bool {
    // SAFETY: guaranteed by the caller.
    let Some(id) = (unsafe { id.as_ref() }) else {
        return false;
    };
    id.iter().zip(I::IID).all(|(a, b)| *a as u8 == b)
}

/// Hands out interface `iid` of a new object, with one reference for the
/// caller.
///
/// # Safety
///
/// `iid` must point at 16 readable bytes and `obj` be writable.
unsafe fn hand_out<C>(object: C, iid: *mut TUID, obj: *mut *mut c_void) -> tresult
where
    C: Class + 'static,
    C::Interfaces: vst3::com_scrape_types::MakeHeader<C, ComWrapper<C>>,
{
    let wrapper = ComWrapper::new(object);
    let Some(unknown) = wrapper.to_com_ptr::<vst3::Steinberg::FUnknown>() else {
        return kNoInterface;
    };
    let ptr = unknown.as_ptr();
    // SAFETY: `ptr` is a live object; `queryInterface` adds the reference the
    // caller receives (the wrapper's own reference goes when it drops).
    unsafe { ((*(*ptr).vtbl).queryInterface)(ptr, iid, obj) }
}

// --- host context -----------------------------------------------------------

/// The host context passed to `initialize`.
pub(crate) struct HostApplication;

impl Class for HostApplication {
    type Interfaces = (IHostApplication,);
}

impl IHostApplicationTrait for HostApplication {
    unsafe fn getName(&self, name: *mut String128) -> tresult {
        // SAFETY: the plugin passes a writable string.
        let Some(name) = (unsafe { name.as_mut() }) else {
            return kInvalidArgument;
        };
        let mut n = 0;
        for (d, s) in name.iter_mut().zip("MusicCraft".encode_utf16()) {
            *d = s as TChar;
            n += 1;
        }
        if let Some(end) = name.get_mut(n) {
            *end = 0;
        }
        kResultOk
    }

    unsafe fn createInstance(
        &self,
        cid: *mut TUID,
        iid: *mut TUID,
        obj: *mut *mut c_void,
    ) -> tresult {
        if obj.is_null() || iid.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: the plugin passes 16-byte ids and a writable output.
        unsafe {
            *obj = ptr::null_mut();
            if is::<IMessage>(cid) {
                hand_out(Message::default(), iid, obj)
            } else if is::<IAttributeList>(cid) {
                hand_out(AttributeList::default(), iid, obj)
            } else {
                kResultFalse
            }
        }
    }
}

// --- messages -----------------------------------------------------------------

/// A value in an attribute list.
#[derive(Debug, Clone)]
enum Attribute {
    Int(i64),
    Float(f64),
    /// UTF-16 with a terminating nul.
    Text(Vec<u16>),
    Binary(Vec<u8>),
}

/// Named values a plugin attaches to a message.
#[derive(Default)]
pub(crate) struct AttributeList {
    values: Mutex<BTreeMap<CString, Attribute>>,
}

impl Class for AttributeList {
    type Interfaces = (IAttributeList,);
}

/// An attribute id as an owned key.
///
/// # Safety
///
/// `id` must be null or a nul-terminated string.
unsafe fn key(id: *const c_char) -> Option<CString> {
    // SAFETY: guaranteed by the caller.
    (!id.is_null()).then(|| unsafe { CStr::from_ptr(id) }.to_owned())
}

impl AttributeList {
    /// Stores `value` under `id`.
    ///
    /// # Safety
    ///
    /// `id` must be null or a nul-terminated string.
    unsafe fn set(&self, id: *const c_char, value: Attribute) -> tresult {
        // SAFETY: guaranteed by the caller.
        let Some(id) = (unsafe { key(id) }) else {
            return kInvalidArgument;
        };
        lock(&self.values).insert(id, value);
        kResultOk
    }

    /// The value under `id`.
    ///
    /// # Safety
    ///
    /// `id` must be null or a nul-terminated string.
    unsafe fn get(&self, id: *const c_char) -> Option<Attribute> {
        // SAFETY: guaranteed by the caller.
        let id = unsafe { key(id) }?;
        lock(&self.values).get(&id).cloned()
    }
}

impl IAttributeListTrait for AttributeList {
    unsafe fn setInt(&self, id: *const c_char, value: i64) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id.
        unsafe { self.set(id, Attribute::Int(value)) }
    }

    unsafe fn getInt(&self, id: *const c_char, value: *mut i64) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id and writable output.
        match (unsafe { self.get(id) }, unsafe { value.as_mut() }) {
            (Some(Attribute::Int(v)), Some(out)) => {
                *out = v;
                kResultOk
            }
            _ => kResultFalse,
        }
    }

    unsafe fn setFloat(&self, id: *const c_char, value: f64) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id.
        unsafe { self.set(id, Attribute::Float(value)) }
    }

    unsafe fn getFloat(&self, id: *const c_char, value: *mut f64) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id and writable output.
        match (unsafe { self.get(id) }, unsafe { value.as_mut() }) {
            (Some(Attribute::Float(v)), Some(out)) => {
                *out = v;
                kResultOk
            }
            _ => kResultFalse,
        }
    }

    unsafe fn setString(&self, id: *const c_char, string: *const TChar) -> tresult {
        if string.is_null() {
            return kInvalidArgument;
        }
        let mut text = Vec::new();
        let mut at = string;
        // SAFETY: the plugin passes a nul-terminated UTF-16 string; reading
        // stops at the terminator.
        while unsafe { *at } != 0 {
            // SAFETY: as above.
            text.push(unsafe { *at } as u16);
            // SAFETY: not past the terminator yet.
            at = unsafe { at.add(1) };
        }
        text.push(0);
        // SAFETY: the plugin passes a nul-terminated id.
        unsafe { self.set(id, Attribute::Text(text)) }
    }

    unsafe fn getString(&self, id: *const c_char, string: *mut TChar, size: u32) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id.
        let Some(Attribute::Text(text)) = (unsafe { self.get(id) }) else {
            return kResultFalse;
        };
        let room = size as usize / size_of::<TChar>();
        if string.is_null() || room == 0 {
            return kInvalidArgument;
        }
        // SAFETY: `string` holds `size` bytes, so `room` characters.
        let out = unsafe { std::slice::from_raw_parts_mut(string, room) };
        // `text` ends with its nul; a string cut short gets one in its place.
        for (d, s) in out.iter_mut().zip(&text) {
            *d = *s as TChar;
        }
        if text.len() > room {
            if let Some(last) = out.last_mut() {
                *last = 0;
            }
        }
        kResultOk
    }

    unsafe fn setBinary(&self, id: *const c_char, data: *const c_void, size: u32) -> tresult {
        let bytes = if data.is_null() || size == 0 {
            Vec::new()
        } else {
            // SAFETY: the plugin passes `size` readable bytes.
            unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size as usize) }.to_vec()
        };
        // SAFETY: the plugin passes a nul-terminated id.
        unsafe { self.set(id, Attribute::Binary(bytes)) }
    }

    unsafe fn getBinary(
        &self,
        id: *const c_char,
        data: *mut *const c_void,
        size: *mut u32,
    ) -> tresult {
        // SAFETY: the plugin passes a nul-terminated id.
        let Some(id) = (unsafe { key(id) }) else {
            return kInvalidArgument;
        };
        let values = lock(&self.values);
        // SAFETY: the plugin passes writable outputs.
        let (Some(Attribute::Binary(bytes)), Some(data), Some(size)) =
            (values.get(&id), unsafe { data.as_mut() }, unsafe {
                size.as_mut()
            })
        else {
            return kResultFalse;
        };
        // The bytes stay where they are until this attribute changes or the
        // list goes, as the interface requires.
        *data = bytes.as_ptr().cast();
        *size = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
        kResultOk
    }
}

/// A message between a plugin's component and controller.
pub(crate) struct Message {
    id: Mutex<CString>,
    attributes: ComWrapper<AttributeList>,
}

impl Default for Message {
    fn default() -> Self {
        Self {
            id: Mutex::new(CString::default()),
            attributes: ComWrapper::new(AttributeList::default()),
        }
    }
}

impl Class for Message {
    type Interfaces = (IMessage,);
}

impl IMessageTrait for Message {
    unsafe fn getMessageID(&self) -> FIDString {
        // The string lives in the message until the id is set again.
        lock(&self.id).as_ptr()
    }

    unsafe fn setMessageID(&self, id: FIDString) {
        // SAFETY: the plugin passes a nul-terminated id (or null).
        if let Some(id) = unsafe { key(id) } {
            *lock(&self.id) = id;
        }
    }

    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        // Not an added reference: the list lives as long as the message.
        self.attributes
            .as_com_ref::<IAttributeList>()
            .map_or(ptr::null_mut(), |r| r.as_ptr())
    }
}

// --- streams ------------------------------------------------------------------

/// An in-memory stream for saving and restoring state.
#[derive(Default)]
pub(crate) struct MemoryStream {
    data: Mutex<(Vec<u8>, usize)>,
}

impl Class for MemoryStream {
    type Interfaces = (IBStream,);
}

impl MemoryStream {
    pub fn with(data: &[u8]) -> Self {
        Self {
            data: Mutex::new((data.to_vec(), 0)),
        }
    }

    pub fn bytes(&self) -> Vec<u8> {
        lock(&self.data).0.clone()
    }
}

impl IBStreamTrait for MemoryStream {
    unsafe fn read(&self, buffer: *mut c_void, count: i32, read: *mut i32) -> tresult {
        let mut guard = lock(&self.data);
        let (data, at) = &mut *guard;
        let rest = data.get(*at..).unwrap_or_default();
        let n = rest.len().min(usize::try_from(count).unwrap_or(0));
        if n > 0 {
            if buffer.is_null() {
                return kInvalidArgument;
            }
            // SAFETY: the plugin gives a writable buffer of `count` bytes.
            unsafe { ptr::copy_nonoverlapping(rest.as_ptr(), buffer.cast::<u8>(), n) };
        }
        *at += n;
        // SAFETY: the plugin passes a writable count (or null).
        if let Some(read) = unsafe { read.as_mut() } {
            *read = n as i32;
        }
        kResultOk
    }

    unsafe fn write(&self, buffer: *mut c_void, count: i32, written: *mut i32) -> tresult {
        let n = usize::try_from(count).unwrap_or(0);
        if n > 0 && buffer.is_null() {
            return kInvalidArgument;
        }
        let mut guard = lock(&self.data);
        let (data, at) = &mut *guard;
        if n > 0 {
            // SAFETY: the plugin gives `count` readable bytes.
            let bytes = unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), n) };
            let end = *at + n;
            if data.len() < end {
                data.resize(end, 0);
            }
            if let Some(target) = data.get_mut(*at..end) {
                target.copy_from_slice(bytes);
            }
            *at = end;
        }
        // SAFETY: the plugin passes a writable count (or null).
        if let Some(written) = unsafe { written.as_mut() } {
            *written = n as i32;
        }
        kResultOk
    }

    unsafe fn seek(&self, pos: i64, mode: i32, result: *mut i64) -> tresult {
        let mut guard = lock(&self.data);
        let (data, at) = &mut *guard;
        let base = match mode as u32 {
            m if m == IBStream_::IStreamSeekMode_::kIBSeekSet as u32 => 0,
            m if m == IBStream_::IStreamSeekMode_::kIBSeekCur as u32 => *at as i64,
            m if m == IBStream_::IStreamSeekMode_::kIBSeekEnd as u32 => data.len() as i64,
            _ => return kInvalidArgument,
        };
        let Ok(target) = usize::try_from(base.saturating_add(pos)) else {
            return kInvalidArgument;
        };
        *at = target.min(data.len());
        // SAFETY: the plugin passes a writable result (or null).
        if let Some(result) = unsafe { result.as_mut() } {
            *result = *at as i64;
        }
        kResultOk
    }

    unsafe fn tell(&self, pos: *mut i64) -> tresult {
        // SAFETY: the plugin passes a writable position.
        let Some(pos) = (unsafe { pos.as_mut() }) else {
            return kInvalidArgument;
        };
        *pos = lock(&self.data).1 as i64;
        kResultOk
    }
}

// --- component handler ----------------------------------------------------------

/// Something the plugin's controller told the host.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HandlerEvent {
    /// A parameter was moved (normalized value).
    Edit(u32, f64),
    /// The plugin asks the host to reload something (`RestartFlags`).
    Restart(i32),
}

/// Hears edits made in a plugin's window.
#[derive(Default)]
pub(crate) struct ComponentHandler {
    events: Mutex<Vec<HandlerEvent>>,
}

impl Class for ComponentHandler {
    type Interfaces = (IComponentHandler,);
}

impl ComponentHandler {
    pub fn take(&self) -> Vec<HandlerEvent> {
        std::mem::take(&mut *lock(&self.events))
    }
}

impl IComponentHandlerTrait for ComponentHandler {
    unsafe fn beginEdit(&self, _id: u32) -> tresult {
        kResultOk
    }

    unsafe fn performEdit(&self, id: u32, value: f64) -> tresult {
        lock(&self.events).push(HandlerEvent::Edit(id, value));
        kResultOk
    }

    unsafe fn endEdit(&self, _id: u32) -> tresult {
        kResultOk
    }

    unsafe fn restartComponent(&self, flags: i32) -> tresult {
        lock(&self.events).push(HandlerEvent::Restart(flags));
        kResultOk
    }
}
