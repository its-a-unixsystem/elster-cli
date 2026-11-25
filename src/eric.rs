// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

#[cfg(test)]
mod ci;
pub(crate) mod install;

#[cfg(not(all(target_os = "linux", target_arch = "x86_64", target_env = "gnu")))]
compile_error!("KIECLI v1 requires x86_64 Linux with glibc");

use std::ffi::{CStr, CString, c_char, c_int, c_void};
use std::fs;
use std::io::ErrorKind;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr::NonNull;
use std::time::Instant;

use libloading::{Library, Symbol, os::unix};
use serde::{Serialize, Serializer};

use crate::{error::Failure, version};

const ERIC_LIB: &str = "libericapi.so";
const OTTO_LIB: &str = "libotto.so";
const PACKAGE_NAME: &str = "Linux-x86_64";
pub(crate) const DEFAULT_DATA_DIR: &str = "kiecli/eric";
const TERMS_MARKER: &str = ".kiecli-eric-terms-accepted";
const ACTIVE_LINK: &str = ".kiecli-eric-current";
const VERSIONS_DIR: &str = "versions";
const INSTALL_HELP: &str = "Use 'kiecli eric install --accept-eric-terms' or follow the manual setup in the KIECLI README.";
pub(crate) const LOG_NOTE: &str = "Use --eric-log DIR to keep ERiC diagnostics.";
pub(crate) const SUPPORTED_MAJORS: &[u16] = &[44];

#[derive(Serialize)]
pub(crate) struct LibraryPaths {
    #[serde(serialize_with = "serialize_path")]
    pub(crate) eric: PathBuf,
    #[serde(serialize_with = "serialize_path")]
    pub(crate) otto: PathBuf,
}

pub(crate) fn serialize_path<S: Serializer>(path: &Path, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_str(&path.display())
}

#[derive(Serialize)]
pub(crate) struct TraceEvent {
    pub(crate) call: &'static str,
    pub(crate) duration_ms: u128,
}

#[derive(Serialize)]
pub(crate) struct Probe {
    pub(crate) version: Option<String>,
    pub(crate) supported: Option<bool>,
    pub(crate) terms_accepted: bool,
    pub(crate) library_paths: LibraryPaths,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) trace: Option<Vec<TraceEvent>>,
}

type Init = unsafe extern "C" fn(*const c_char, *const c_char) -> c_int;
type End = unsafe extern "C" fn() -> c_int;
type BufferNew = unsafe extern "C" fn() -> *mut c_void;
type BufferContent = unsafe extern "C" fn(*mut c_void) -> *const c_char;
type BufferFree = unsafe extern "C" fn(*mut c_void) -> c_int;
type Version = unsafe extern "C" fn(*mut c_void) -> c_int;
type ErrorText = unsafe extern "C" fn(c_int, *mut c_void) -> c_int;
type OpenCertificate = unsafe extern "C" fn(*mut u32, *mut u32, *const c_char) -> c_int;
type CloseCertificate = unsafe extern "C" fn(u32) -> c_int;
type CertificateProperties = unsafe extern "C" fn(u32, *const c_char, *mut c_void) -> c_int;

pub(crate) struct Session {
    library: Library,
    end: Option<End>,
}

struct Certificate<'lib> {
    handle: u32,
    close: Symbol<'lib, CloseCertificate>,
}

impl Drop for Certificate<'_> {
    fn drop(&mut self) {
        // SAFETY: this is the unique handle owner and the library is still loaded.
        unsafe { (self.close)(self.handle) };
    }
}

struct Buffer<'lib> {
    ptr: NonNull<c_void>,
    content: Symbol<'lib, BufferContent>,
    free: Symbol<'lib, BufferFree>,
}

impl<'lib> Buffer<'lib> {
    fn new(lib: &'lib Library) -> Result<Self, Failure> {
        // SAFETY: these are the ERiC return-buffer ABI signatures. The symbols borrow lib.
        let (new, content, free) = unsafe {
            (
                lib.get::<BufferNew>(b"EricRueckgabepufferErzeugen\0"),
                lib.get::<BufferContent>(b"EricRueckgabepufferInhalt\0"),
                lib.get::<BufferFree>(b"EricRueckgabepufferFreigeben\0"),
            )
        };
        let (new, content, free) = (
            new.map_err(symbol_error)?,
            content.map_err(symbol_error)?,
            free.map_err(symbol_error)?,
        );
        // SAFETY: we call ERiC synchronously on the main thread with an active session.
        let ptr = NonNull::new(unsafe { new() }).ok_or_else(|| {
            Failure::new(
                4,
                "eric_buffer_failed",
                "ERiC could not allocate a return buffer",
            )
            .note(LOG_NOTE)
        })?;
        Ok(Self { ptr, content, free })
    }

    fn text(&self) -> Option<&str> {
        // SAFETY: the buffer is still live and ERiC returns a NUL-terminated string or null.
        // The borrow prevents returning a string that outlives the buffer.
        unsafe {
            let text = (self.content)(self.ptr.as_ptr());
            (!text.is_null())
                .then(|| CStr::from_ptr(text).to_str().ok())
                .flatten()
        }
    }
}

impl Drop for Buffer<'_> {
    fn drop(&mut self) {
        // SAFETY: this is the unique owner, and the library is still loaded.
        unsafe { (self.free)(self.ptr.as_ptr()) };
    }
}

impl Session {
    fn start(library: Library, plugins: &Path, logs: &Path) -> Result<Self, Failure> {
        let plugins = c_path(plugins)?;
        let logs = c_path(logs)?;
        // SAFETY: these are the ERiC lifecycle ABI signatures. `end` remains valid while
        // `library` is owned by the session, including while its Drop runs.
        let (init, end) = unsafe {
            (
                library.get::<Init>(b"EricInitialisiere\0"),
                library.get::<End>(b"EricBeende\0"),
            )
        };
        let end = *end.map_err(symbol_error)?;
        // SAFETY: both paths are live C strings; this is the process's only initialization.
        let code = unsafe { init.map_err(symbol_error)?(plugins.as_ptr(), logs.as_ptr()) };
        if code != 0 {
            return Err(
                Failure::new(4, "eric_init_failed", "EricInitialisiere failed")
                    .eric_cause(code, None)
                    .help(INSTALL_HELP)
                    .note(LOG_NOTE),
            );
        }
        Ok(Self {
            library,
            end: Some(end),
        })
    }

    fn version(&self) -> Result<[u16; 4], Failure> {
        // SAFETY: the signature matches ERiC's version entry point.
        let version =
            unsafe { self.library.get::<Version>(b"EricVersion\0") }.map_err(symbol_error)?;
        let buffer = Buffer::new(&self.library)?;
        // SAFETY: this is a live return buffer and ERiC has been initialized.
        let code = unsafe { version(buffer.ptr.as_ptr()) };
        if code != 0 {
            return Err(Failure::new(4, "eric_version_failed", "EricVersion failed")
                .eric_cause(code, self.error_text(code))
                .help(INSTALL_HELP)
                .note(LOG_NOTE));
        }
        buffer
            .text()
            .and_then(|xml| version::read(xml, ERIC_LIB))
            .ok_or_else(|| {
                Failure::new(
                    4,
                    "eric_version_invalid",
                    format!("ERiC did not report a valid version for {ERIC_LIB}"),
                )
                .help(INSTALL_HELP)
                .note(LOG_NOTE)
            })
    }

    fn open_certificate(&self, path: &Path) -> Result<Certificate<'_>, Failure> {
        let path = c_path(path)?;
        // SAFETY: the checked ERiC 44 ABI uses these handle functions.
        let open = unsafe {
            self.library
                .get::<OpenCertificate>(b"EricGetHandleToCertificate\0")
        }
        .map_err(symbol_error)?;
        // SAFETY: the checked ERiC 44 ABI uses these handle functions.
        let close = unsafe {
            self.library
                .get::<CloseCertificate>(b"EricCloseHandleToCertificate\0")
        }
        .map_err(symbol_error)?;
        let (mut handle, mut pin_support) = (0, 0);
        // SAFETY: both output pointers and the NUL-terminated path are live.
        let code = unsafe { open(&mut handle, &mut pin_support, path.as_ptr()) };
        if code != 0 {
            return Err(self.certificate_error(code));
        }
        Ok(Certificate { handle, close })
    }
}

pub(crate) trait CertificateReader {
    fn inspect_certificate<T>(
        &self,
        path: &Path,
        pin: &crate::pin::Pin,
        trace: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(&str) -> Result<T, Failure>,
    ) -> Result<T, Failure>;
}

impl CertificateReader for Session {
    fn inspect_certificate<T>(
        &self,
        path: &Path,
        pin: &crate::pin::Pin,
        trace: &mut Option<Vec<TraceEvent>>,
        parse: impl FnOnce(&str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let cert = timed(trace, "EricGetHandleToCertificate", || {
            self.open_certificate(path)
        })?;
        // SAFETY: this is the ERiC 44 certificate properties ABI.
        let properties = unsafe {
            self.library
                .get::<CertificateProperties>(b"EricHoleZertifikatEigenschaften\0")
        }
        .map_err(symbol_error)?;
        let buffer = Buffer::new(&self.library)?;
        let code = timed(trace, "EricHoleZertifikatEigenschaften", || {
            // SAFETY: the certificate, PIN and buffer are live for this synchronous call.
            unsafe { properties(cert.handle, pin.as_c_str().as_ptr(), buffer.ptr.as_ptr()) }
        });
        timed(trace, "EricCloseHandleToCertificate", || drop(cert));
        if code != 0 {
            return Err(self.certificate_error(code));
        }
        parse(buffer.text().ok_or_else(|| {
            Failure::new(
                1,
                "certificate_properties_invalid",
                "ERiC did not return Certificate properties",
            )
            .note(LOG_NOTE)
        })?)
    }
}

impl Session {
    fn certificate_error(&self, code: c_int) -> Failure {
        let (exit, code_name, message) = match code {
            610201106 => (3, "pin_wrong", "The Certificate PIN is incorrect"),
            610201107 => (3, "pin_locked", "The Certificate PIN is locked"),
            610201101..=610201212 => (
                3,
                "certificate_unreadable",
                "ERiC could not open the Certificate",
            ),
            _ => (
                1,
                "certificate_inspection_failed",
                "ERiC could not inspect the Certificate",
            ),
        };
        Failure::new(exit, code_name, message)
            .eric_cause(code, self.error_text(code))
            .note(LOG_NOTE)
    }

    fn error_text(&self, code: c_int) -> Option<String> {
        // SAFETY: the signature matches ERiC's error-text entry point.
        let describe = unsafe { self.library.get::<ErrorText>(b"EricHoleFehlerText\0") }.ok()?;
        let buffer = Buffer::new(&self.library).ok()?;
        // SAFETY: this is a live return buffer and ERiC has been initialized.
        (unsafe { describe(code, buffer.ptr.as_ptr()) } == 0)
            .then(|| buffer.text().map(str::to_owned))
            .flatten()
    }

    fn finish(mut self) -> Result<(), Failure> {
        let code = self.end();
        if code == 0 {
            Ok(())
        } else {
            // The session is no longer usable for error text lookup.
            Err(Failure::new(4, "eric_end_failed", "EricBeende failed")
                .eric_cause(code, None)
                .help(INSTALL_HELP)
                .note(LOG_NOTE))
        }
    }

    fn end(&mut self) -> c_int {
        self.end.take().map_or(0, |end| {
            // SAFETY: start succeeded, the library is live, and teardown runs only once.
            unsafe { end() }
        })
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.end();
    }
}

impl Probe {
    pub(crate) fn new(dir: &Path, verbose: bool) -> Self {
        let lib_dir = dir.join("lib");
        Self {
            version: None,
            supported: None,
            terms_accepted: false,
            library_paths: LibraryPaths {
                eric: lib_dir.join(ERIC_LIB),
                otto: lib_dir.join(OTTO_LIB),
            },
            trace: verbose.then(Vec::new),
        }
    }

    pub(crate) fn check(&mut self, dir: &Path, log_dir: Option<&Path>) -> Result<(), Failure> {
        self.with_session(dir, log_dir, |_, _| Ok(()))
    }

    pub(crate) fn with_session<T>(
        &mut self,
        dir: &Path,
        log_dir: Option<&Path>,
        run: impl FnOnce(&Session, &mut Option<Vec<TraceEvent>>) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let marker = dir.join(TERMS_MARKER);
        self.terms_accepted = marker.is_file();
        require_library(&self.library_paths.eric)?;
        if !self.terms_accepted {
            return Err(Failure::new(
                4,
                "eric_terms_not_accepted",
                format!(
                    "ERiC terms acceptance is not recorded in '{}'",
                    marker.display()
                ),
            )
            .help(INSTALL_HELP));
        }
        let library = timed(&mut self.trace, "load ERiC", || {
            load(&self.library_paths.eric)
        })?;
        let temporary;
        let logs = match log_dir {
            Some(path) => {
                fs::create_dir_all(path).map_err(|err| log_error(path, err))?;
                path
            }
            None => {
                temporary = tempfile::Builder::new()
                    .prefix("kiecli-eric-")
                    .tempdir()
                    .map_err(|err| log_error(&std::env::temp_dir(), err))?;
                temporary.path()
            }
        };
        let logs = std::path::absolute(logs).map_err(|err| log_error(logs, err))?;
        let plugins = dir.join("lib/plugins");
        let session = timed(&mut self.trace, "EricInitialisiere", || {
            Session::start(library, &plugins, &logs)
        })?;
        let [major, minor, patch, build] =
            timed(&mut self.trace, "EricVersion", || session.version())?;
        let version = self
            .version
            .insert(format!("{major}.{minor}.{patch}.{build}"));
        let supported = SUPPORTED_MAJORS.contains(&major);
        self.supported = Some(supported);
        if !supported {
            return Err(Failure::new(
                4,
                "eric_version_unsupported",
                format!("ERiC {version} is not supported; supported majors: {SUPPORTED_MAJORS:?}"),
            )
            .help(INSTALL_HELP));
        }
        require_library(&self.library_paths.otto)?;
        timed(&mut self.trace, "load Otto", || {
            check_otto(&self.library_paths.otto)
        })?;
        let result = run(&session, &mut self.trace);
        let finished = timed(&mut self.trace, "EricBeende", || session.finish());
        let value = result?;
        finished?;
        Ok(value)
    }
}

pub(crate) fn installed_dir(root: &Path) -> Result<Option<PathBuf>, Failure> {
    use std::path::Component::Normal;

    let pointer = root.join(ACTIVE_LINK);
    let target = match fs::read_link(&pointer) {
        Ok(target) => target,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(Failure::new(
                4,
                "eric_installation_invalid",
                format!("Cannot read '{}': {err}", pointer.display()),
            )
            .help(INSTALL_HELP));
        }
    };
    let mut parts = target.components();
    if matches!(parts.next(), Some(Normal(name)) if name == VERSIONS_DIR)
        && matches!(parts.next(), Some(Normal(_)))
        && parts.next().is_none()
    {
        Ok(Some(root.join(target)))
    } else {
        Err(Failure::new(
            4,
            "eric_installation_invalid",
            format!(
                "'{}' does not point to a managed ERiC version",
                pointer.display()
            ),
        )
        .help(INSTALL_HELP))
    }
}

fn check_otto(path: &Path) -> Result<(), Failure> {
    let library = load(path)?;
    // Resolve opaque addresses only: the status probe never calls Otto.
    for symbol in [
        c"OttoInstanzErzeugen",
        c"OttoInstanzFreigeben",
        c"OttoDatenAbholen",
        c"OttoHoleFehlertext",
        c"OttoRueckgabepufferErzeugen",
        c"OttoRueckgabepufferInhalt",
        c"OttoRueckgabepufferGroesse",
        c"OttoRueckgabepufferFreigeben",
    ] {
        // SAFETY: we look up each symbol as an opaque pointer and don't dereference or call it.
        unsafe { library.get::<*const c_void>(symbol.to_bytes_with_nul()) }
            .map_err(symbol_error)?;
    }
    Ok(())
}

fn timed<T>(trace: &mut Option<Vec<TraceEvent>>, call: &'static str, f: impl FnOnce() -> T) -> T {
    if let Some(trace) = trace {
        let start = Instant::now();
        let result = f();
        trace.push(TraceEvent {
            call,
            duration_ms: start.elapsed().as_millis(),
        });
        result
    } else {
        f()
    }
}

fn require_library(path: &Path) -> Result<(), Failure> {
    match fs::metadata(path) {
        Ok(meta) if meta.is_file() => Ok(()),
        Err(err) if err.kind() != ErrorKind::NotFound => Err(Failure::new(
            4,
            "eric_load_failed",
            format!("Cannot read '{}': {err}", path.display()),
        )
        .help(INSTALL_HELP)
        .note(LOG_NOTE)),
        _ => Err(Failure::new(
            4,
            "eric_missing",
            format!("No {PACKAGE_NAME} ERiC library at '{}'", path.display()),
        )
        .help(INSTALL_HELP)),
    }
}

fn load(path: &Path) -> Result<Library, Failure> {
    // SAFETY: only load from the configured, terms-accepted ERiC installation. Its shared
    // libraries and dependencies are trusted native code supplied by the caller.
    let library = unsafe { unix::Library::open(Some(path), unix::RTLD_NOW | unix::RTLD_LOCAL) };
    library.map(Library::from).map_err(|err| {
        let detail = err.to_string();
        if detail.starts_with("libgcc_s.so.1:") {
            Failure::new(
                4,
                "eric_libgcc_missing",
                format!("Could not load '{}': {detail}", path.display()),
            )
            .help("Install libgcc-s1 on Debian/Ubuntu or libgcc on Fedora/Arch.")
            .note(LOG_NOTE)
        } else {
            Failure::new(
                4,
                "eric_load_failed",
                format!("Could not load '{}': {detail}", path.display()),
            )
            .help(INSTALL_HELP)
            .note(LOG_NOTE)
        }
    })
}

fn symbol_error(err: libloading::Error) -> Failure {
    Failure::new(4, "eric_symbol_missing", err.to_string())
        .help(INSTALL_HELP)
        .note(LOG_NOTE)
}

fn c_path(path: &Path) -> Result<CString, Failure> {
    CString::new(path.as_os_str().as_bytes()).map_err(|_| {
        Failure::new(
            3,
            "path_invalid",
            format!("Path '{}' contains a NUL byte", path.display()),
        )
    })
}

fn log_error(path: &Path, err: std::io::Error) -> Failure {
    Failure::new(
        3,
        "eric_log_unavailable",
        format!("Cannot use log directory '{}': {err}", path.display()),
    )
}
