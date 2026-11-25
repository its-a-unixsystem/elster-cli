// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::ffi::{CStr, CString, c_char, c_int, c_uint, c_void};
use std::io::Write;
use std::path::Path;
use std::ptr::{self, NonNull};

use libloading::Library;

use super::{LOG_NOTE, ObjectFetcher, TraceEvent, c_path, load, symbol_error, timed};
use crate::{error::Failure, pin::Pin};

type Status = c_uint;
type LogCallback =
    unsafe extern "C" fn(*const c_char, *const c_char, c_uint, *const c_char, *mut c_void) -> c_int;
type Init = unsafe extern "C" fn(
    *const c_char,
    Option<LogCallback>,
    *mut c_void,
    *mut *mut c_void,
) -> Status;
type Fetch = unsafe extern "C" fn(
    *mut c_void,
    *const c_char,
    u32,
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    *mut c_void,
) -> Status;

pub(super) struct Api {
    init: Init,
    end: unsafe extern "C" fn(*mut c_void) -> Status,
    fetch: Fetch,
    error_text: unsafe extern "C" fn(Status) -> *const c_char,
    buffer_new: unsafe extern "C" fn(*mut c_void, *mut *mut c_void) -> Status,
    buffer_content: unsafe extern "C" fn(*mut c_void) -> *const c_char,
    buffer_size: unsafe extern "C" fn(*mut c_void) -> u64,
    buffer_free: unsafe extern "C" fn(*mut c_void) -> Status,
    _library: Library,
}

impl Api {
    pub(super) fn load(path: &Path) -> Result<Self, Failure> {
        let library = load(path)?;
        // SAFETY: these signatures match Otto's ERiC 44 ABI. Copying the function pointers
        // into Api is valid because it owns the library. The offline probe only resolves them.
        unsafe {
            Ok(Self {
                init: *library
                    .get(b"OttoInstanzErzeugen\0")
                    .map_err(symbol_error)?,
                end: *library
                    .get(b"OttoInstanzFreigeben\0")
                    .map_err(symbol_error)?,
                fetch: *library.get(b"OttoDatenAbholen\0").map_err(symbol_error)?,
                error_text: *library.get(b"OttoHoleFehlertext\0").map_err(symbol_error)?,
                buffer_new: *library
                    .get(b"OttoRueckgabepufferErzeugen\0")
                    .map_err(symbol_error)?,
                buffer_content: *library
                    .get(b"OttoRueckgabepufferInhalt\0")
                    .map_err(symbol_error)?,
                buffer_size: *library
                    .get(b"OttoRueckgabepufferGroesse\0")
                    .map_err(symbol_error)?,
                buffer_free: *library
                    .get(b"OttoRueckgabepufferFreigeben\0")
                    .map_err(symbol_error)?,
                _library: library,
            })
        }
    }

    pub(super) fn fetcher<'a>(
        &'a self,
        logs: &CStr,
        certificate: &Path,
        pin: &'a Pin,
        hersteller_id: &str,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<Fetcher<'a>, Failure> {
        let certificate = c_path(certificate)?;
        let hersteller_id = CString::new(hersteller_id).map_err(|_| {
            Failure::new(
                3,
                "hersteller_id_invalid",
                "HerstellerID must be five digits",
            )
        })?;
        let mut instance = ptr::null_mut();
        let status = timed(trace, "OttoInstanzErzeugen", || {
            // SAFETY: logs is a live C path, instance is a writable out parameter, and
            // without a callback Otto does not use the callback data pointer.
            unsafe { (self.init)(logs.as_ptr(), None, ptr::null_mut(), &mut instance) }
        });
        let fetcher = NonNull::new(instance).map(|instance| Fetcher {
            api: self,
            instance,
            certificate,
            pin,
            hersteller_id,
        });
        if status != 0 {
            return Err(self.diagnose(
                status,
                Failure::new(4, "otto_init_failed", "OttoInstanzErzeugen failed"),
            ));
        }
        fetcher.ok_or_else(|| {
            Failure::new(4, "otto_init_failed", "Otto did not return an instance").note(LOG_NOTE)
        })
    }

    fn diagnose(&self, status: Status, failure: Failure) -> Failure {
        // SAFETY: Otto returns either null or a static, NUL-terminated UTF-8 string.
        let text = unsafe {
            let text = (self.error_text)(status);
            if text.is_null() {
                None
            } else {
                CStr::from_ptr(text).to_str().ok().map(str::to_owned)
            }
        };
        failure
            .cause("otto", Some(status as i32), text)
            .note(LOG_NOTE)
    }
}

pub(super) struct Fetcher<'a> {
    api: &'a Api,
    instance: NonNull<c_void>,
    certificate: CString,
    pin: &'a Pin,
    hersteller_id: CString,
}

impl ObjectFetcher for Fetcher<'_> {
    fn fetch(
        &self,
        object_id: &str,
        transport_size: u64,
        out: &mut dyn Write,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<(), Failure> {
        let object_id = CString::new(object_id).map_err(|_| {
            Failure::new(
                5,
                "otto_invalid_parameter",
                "The object ID contains a NUL byte",
            )
        })?;
        // Otto accepts an underestimated allocation hint. Use zero rather than truncating
        // a size beyond the ABI's 32-bit range or asking Otto to preallocate 4 GiB.
        let size_hint = u32::try_from(transport_size).unwrap_or(0);
        let buffer = Buffer::new(self)?;
        let status = timed(trace, "OttoDatenAbholen", || {
            // SAFETY: the library, instance, strings and instance-owned buffer are live
            // throughout the synchronous call. No separate Abholzertifikat is supplied.
            unsafe {
                (self.api.fetch)(
                    self.instance.as_ptr(),
                    object_id.as_ptr(),
                    size_hint,
                    self.certificate.as_ptr(),
                    self.pin.as_c_str().as_ptr(),
                    self.hersteller_id.as_ptr(),
                    ptr::null(),
                    buffer.handle.as_ptr(),
                )
            }
        });
        if status != 0 {
            let failure = match status {
                610405831 => Failure::new(3, "pin_wrong", "The Certificate PIN is incorrect"),
                610403007 => Failure::new(3, "otto_unauthorized", "OTTER refused authorization"),
                610403008 => {
                    Failure::new(5, "otto_object_not_found", "OTTER did not find the object")
                }
                610405001 => Failure::new(
                    5,
                    "otto_invalid_parameter",
                    "Otto rejected a fetch parameter",
                ),
                610403003 | 610403005 | 610403009 => {
                    Failure::transient("otto_transport_failed", "The OTTER transfer failed")
                }
                _ => Failure::new(1, "otto_fetch_failed", "OttoDatenAbholen failed"),
            };
            return Err(self.api.diagnose(status, failure));
        }
        let bytes = buffer.bytes().ok_or_else(|| {
            Failure::new(
                1,
                "otto_fetch_failed",
                "Otto returned an invalid data buffer",
            )
            .note(LOG_NOTE)
        })?;
        if !bytes.is_empty() {
            out.write_all(bytes).map_err(|err| {
                Failure::new(
                    1,
                    "postfach_write_failed",
                    format!("Could not write the retrieved Postfach object: {err}"),
                )
            })?;
        }
        Ok(())
    }
}

impl Drop for Fetcher<'_> {
    fn drop(&mut self) {
        // SAFETY: this is the unique owner. Buffers are dropped before their instance.
        unsafe { (self.api.end)(self.instance.as_ptr()) };
    }
}

struct Buffer<'a> {
    api: &'a Api,
    handle: NonNull<c_void>,
}

impl<'a> Buffer<'a> {
    fn new(fetcher: &'a Fetcher<'_>) -> Result<Self, Failure> {
        let api = fetcher.api;
        let mut handle = ptr::null_mut();
        // SAFETY: the instance is live and handle is a writable out parameter.
        let status = unsafe { (api.buffer_new)(fetcher.instance.as_ptr(), &mut handle) };
        let buffer = NonNull::new(handle).map(|handle| Self { api, handle });
        if status != 0 {
            return Err(api.diagnose(
                status,
                Failure::new(
                    1,
                    "otto_fetch_failed",
                    "Otto could not allocate a return buffer",
                ),
            ));
        }
        buffer.ok_or_else(|| {
            Failure::new(1, "otto_fetch_failed", "Otto did not return a buffer").note(LOG_NOTE)
        })
    }

    fn bytes(&self) -> Option<&[u8]> {
        // SAFETY: the instance and this buffer remain live during the shared borrow.
        let size = unsafe { (self.api.buffer_size)(self.handle.as_ptr()) };
        if size == 0 {
            return Some(&[]);
        }
        if size > isize::MAX as u64 {
            return None;
        }
        // SAFETY: the owned buffer is live. Otto guarantees that a non-null pointer has
        // size bytes, excluding the NUL terminator, until this buffer is reused or dropped.
        // It isn't reused, and the resulting slice cannot outlive the buffer.
        unsafe {
            let bytes = (self.api.buffer_content)(self.handle.as_ptr());
            (!bytes.is_null()).then(|| std::slice::from_raw_parts(bytes.cast(), size as usize))
        }
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        // SAFETY: this is the unique owner and the instance and library are still live.
        unsafe { (self.api.buffer_free)(self.handle.as_ptr()) };
    }
}
