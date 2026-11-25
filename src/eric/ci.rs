// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::env;
use std::ffi::CString;
use std::path::PathBuf;

use serde::Deserialize;

use super::{Certificate, ERIC_LIB, Session, TERMS_MARKER, load};
use crate::error::Failure;

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
    certificate: Option<&Certificate<'_>>,
) -> (i32, Option<u32>) {
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
    process_request(session, &xml, c"PostfachStatus_32", certificate)
}

fn process_request(
    session: &Session,
    xml: &std::ffi::CStr,
    version: &std::ffi::CStr,
    certificate: Option<&Certificate<'_>>,
) -> (i32, Option<u32>) {
    session
        .process(
            xml,
            version,
            certificate.map(|cert| (cert, c"123456")),
            &mut None,
            |code, server, _| Ok((code, server_code(server))),
        )
        .unwrap_or_else(fail)
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
        let golden = include_str!("../../tests/fixtures/requests/postfach-status.xml");
        let valid = CString::new(golden).unwrap();
        let blocked =
            CString::new(golden.replace("<HerstellerID>12345", "<HerstellerID>74931")).unwrap();
        assert_eq!(
            process_request(session, &valid, c"PostfachStatus_32", None),
            (0, None)
        );
        assert_eq!(
            process_request(session, &blocked, c"PostfachStatus_32", None),
            (610301202, None),
        );

        let anfrage = CString::new(include_str!(
            "../../tests/fixtures/requests/postfach-anfrage.xml"
        ))
        .unwrap();
        assert_eq!(
            process_request(session, &anfrage, c"PostfachAnfrage_32", None),
            (0, None)
        );

        let bestaetigung = CString::new(include_str!(
            "../../tests/fixtures/requests/postfach-bestaetigung.xml"
        ))
        .unwrap();
        assert_eq!(
            process_request(session, &bestaetigung, c"PostfachBestaetigung_32", None),
            (0, None)
        );
    });
}

#[test]
#[ignore = "requires official ERiC 44; does not contact ELSTER"]
fn converts_a_bescheid_steuernummer_offline() {
    with_session(|session| {
        let mut trace = None;
        let (number, bufa) = session
            .convert_steuernummer("199/262/11407", "09", None, &mut trace)
            .unwrap_or_else(fail);
        assert_eq!(number, "9199026211407");
        assert_eq!(bufa, "9199");
    });
}

#[test]
#[ignore = "one-off read-only ELSTER test-system request; requires official ERiC and test Certificate"]
fn elster_test_system_is_reachable() {
    let since = env::var("KIECLI_TEST_POSTFACH_SINCE").expect("KIECLI_TEST_POSTFACH_SINCE");
    let cert =
        PathBuf::from(env::var_os("KIECLI_TEST_CERTIFICATE").expect("KIECLI_TEST_CERTIFICATE"));
    with_session(|session| {
        let cert = session.open_certificate(&cert).unwrap_or_else(fail);
        assert_eq!(
            postfach_status(session, &since, "12345", Some(&cert)),
            (610101210, Some(601015001)),
            "expected a TransferHeader rejection from ELSTER, not a local or transport error",
        );
    });
}
