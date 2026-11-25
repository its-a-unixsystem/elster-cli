// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::env;
use std::ffi::{CString, c_char, c_int, c_void};
use std::path::{Path, PathBuf};
use std::ptr;

use libloading::Symbol;
use serde::Deserialize;

use super::{Buffer, ERIC_LIB, Session, TERMS_MARKER, c_path, load};
use crate::error::Failure;

const VALIDATE: u32 = 1 << 1;
const SEND: u32 = 1 << 2;

#[repr(C)]
struct Encryption {
    version: u32,
    handle: u32,
    pin: *const c_char,
}

type OpenCertificate = unsafe extern "C" fn(*mut u32, *mut u32, *const c_char) -> c_int;
type CloseCertificate = unsafe extern "C" fn(u32) -> c_int;
type Process = unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    u32,
    *const c_void,
    *const Encryption,
    *mut c_void,
    *mut c_void,
) -> c_int;

struct TestCertificate<'lib> {
    handle: u32,
    close: Symbol<'lib, CloseCertificate>,
}

impl<'lib> TestCertificate<'lib> {
    fn open(session: &'lib Session, path: &Path) -> Self {
        let path = c_path(path).unwrap_or_else(fail);
        // SAFETY: these are the ERiC 44 certificate-handle ABI signatures.
        let (open, close) = unsafe {
            (
                session
                    .library
                    .get::<OpenCertificate>(b"EricGetHandleToCertificate\0"),
                session
                    .library
                    .get::<CloseCertificate>(b"EricCloseHandleToCertificate\0"),
            )
        };
        let open = open.expect("EricGetHandleToCertificate");
        let close = close.expect("EricCloseHandleToCertificate");
        let (mut handle, mut pin_support) = (0, 0);
        // SAFETY: both output pointers and the NUL-terminated path are live for the call.
        let code = unsafe { open(&mut handle, &mut pin_support, path.as_ptr()) };
        assert_eq!(code, 0, "EricGetHandleToCertificate failed");
        Self { handle, close }
    }
}

impl Drop for TestCertificate<'_> {
    fn drop(&mut self) {
        // SAFETY: this is the unique handle owner; the session still lives.
        unsafe { (self.close)(self.handle) };
    }
}

fn fail<T>(failure: Failure) -> T {
    panic!(
        "{}",
        serde_json::to_string(&failure).expect("serialize failure")
    );
}

fn with_session(test: impl FnOnce(&Session)) {
    let home = PathBuf::from(env::var_os("KIECLI_TEST_ERIC_HOME").expect("KIECLI_TEST_ERIC_HOME"));
    assert!(
        home.join(TERMS_MARKER).is_file(),
        "record ERiC terms acceptance first"
    );
    let logs = tempfile::tempdir().expect("temporary ERiC log directory");
    let library = load(&home.join("lib").join(ERIC_LIB)).unwrap_or_else(fail);
    let session =
        Session::start(library, &home.join("lib/plugins"), logs.path()).unwrap_or_else(fail);
    let [major, ..] = session.version().unwrap_or_else(fail);
    assert_eq!(major, 44, "the probe is written for the ERiC 44 ABI");
    test(&session);
    session.finish().unwrap_or_else(fail);
}

fn postfach_status(
    session: &Session,
    since: &str,
    hersteller_id: &str,
    certificate: Option<&TestCertificate<'_>>,
) -> (c_int, Option<u32>) {
    assert!(since.len() == 8 && since.bytes().all(|byte| byte.is_ascii_digit()));
    // 700000004 is valid locally but deliberately rejected by ELSTER through ERiC.
    let xml = CString::new(format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">
<TransferHeader version="11">
<Verfahren>ElsterDatenabholung</Verfahren><DatenArt>PostfachStatus</DatenArt><Vorgang>send-Auth</Vorgang>
<Testmerker>700000004</Testmerker><HerstellerID>{hersteller_id}</HerstellerID><DatenLieferant>KIECLI CI</DatenLieferant>
<Datei><Verschluesselung>CMSEncryptedData</Verschluesselung><Kompression>GZIP</Kompression><TransportSchluessel/></Datei>
</TransferHeader>
<DatenTeil><Nutzdatenblock>
<NutzdatenHeader version="11"><NutzdatenTicket>1</NutzdatenTicket><Empfaenger id="L">CS</Empfaenger>
<Hersteller><ProduktName>KIECLI</ProduktName><ProduktVersion>0.1.0</ProduktVersion></Hersteller></NutzdatenHeader>
<Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32">
<PostfachStatus einschraenkung="neue" rolle="inhaber" von="{since}"/>
</Datenabholung></Nutzdaten>
</Nutzdatenblock></DatenTeil>
</Elster>"#
    )).expect("NUL-free probe XML");
    // SAFETY: the checked ERiC 44 major uses this signature, without a transfer handle.
    let process = unsafe { session.library.get::<Process>(b"EricBearbeiteVorgang\0") }
        .expect("EricBearbeiteVorgang");
    let result = Buffer::new(&session.library).unwrap_or_else(fail);
    let server = Buffer::new(&session.library).unwrap_or_else(fail);
    let encryption = certificate.map(|certificate| Encryption {
        version: 3,
        handle: certificate.handle,
        pin: c"123456".as_ptr(),
    });
    let flags = VALIDATE | if certificate.is_some() { SEND } else { 0 };
    // SAFETY: all pointers are live until the synchronous call returns. Null print parameters
    // and a null encryption parameter for local validation are supported by ERiC.
    let code = unsafe {
        process(
            xml.as_ptr(),
            c"PostfachStatus_32".as_ptr(),
            flags,
            ptr::null(),
            encryption.as_ref().map_or(ptr::null(), ptr::from_ref),
            result.ptr.as_ptr(),
            server.ptr.as_ptr(),
        )
    };
    (code, server.text().and_then(server_code))
}

fn server_code(xml: &str) -> Option<u32> {
    #[derive(Deserialize)]
    struct Reply {
        #[serde(rename = "TransferHeader")]
        header: Header,
    }
    #[derive(Deserialize)]
    struct Header {
        #[serde(rename = "RC")]
        rc: ReturnCode,
    }
    #[derive(Deserialize)]
    struct ReturnCode {
        #[serde(rename = "Rueckgabe")]
        result: ResultCode,
    }
    #[derive(Deserialize)]
    struct ResultCode {
        #[serde(rename = "Code")]
        code: u32,
    }
    quick_xml::de::from_str::<Reply>(xml)
        .ok()
        .map(|reply| reply.header.rc.result.code)
}

#[test]
#[ignore = "requires official ERiC 44; does not contact ELSTER"]
fn validates_placeholder_hersteller_id_offline() {
    with_session(|session| {
        assert_eq!(
            postfach_status(session, "20261124", "12345", None),
            (0, None)
        );
        assert_eq!(
            postfach_status(session, "20261124", "74931", None),
            (610301202, None),
        );
    });
}

#[test]
#[ignore = "one-off read-only ELSTER test-system request; requires official ERiC and test Certificate"]
fn elster_test_system_is_reachable() {
    let since = env::var("KIECLI_TEST_POSTFACH_SINCE").expect("KIECLI_TEST_POSTFACH_SINCE");
    let cert =
        PathBuf::from(env::var_os("KIECLI_TEST_CERTIFICATE").expect("KIECLI_TEST_CERTIFICATE"));
    with_session(|session| {
        let cert = TestCertificate::open(session, &cert);
        assert_eq!(
            postfach_status(session, &since, "12345", Some(&cert)),
            (610101210, Some(601015001)),
            "expected a TransferHeader rejection from ELSTER, not a local or transport error",
        );
    });
}
