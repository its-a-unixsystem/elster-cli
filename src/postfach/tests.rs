// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::cell::Cell;
use std::collections::VecDeque;
use std::ffi::CStr;

use clap::Parser;

use super::*;

const STATUS_REPLY: &str = include_str!("../../tests/fixtures/elster/postfach-status.xml");
const ANFRAGE_REPLY: &str = include_str!("../../tests/fixtures/elster/postfach-anfrage.xml");

fn scope(args: &[&str]) -> Scope {
    let cli = Cli::try_parse_from(args).unwrap();
    let crate::Command::Postfach { command } = &cli.command else {
        panic!("expected postfach");
    };
    let (options, operation) = match command {
        Command::Status(options) => (options, Operation::Status),
        Command::Anfrage(options) => (options, Operation::Anfrage),
        _ => panic!("expected status or anfrage"),
    };
    Scope::new(&cli, options, operation)
        .unwrap_or_else(|err| panic!("{}", serde_json::to_value(err).unwrap()))
}

const TEST_PROPERTIES: &str = concat!(
    "<Properties><Signaturzertifikateigenschaften>",
    "<AusgestelltAm>20240101000000Z</AusgestelltAm><GueltigBis>20260201000000Z</GueltigBis>",
    "<Testzertifikat>true</Testzertifikat>",
    "</Signaturzertifikateigenschaften></Properties>",
);

struct FakeEric<'a> {
    certificate: &'a str,
    reply: (i32, &'a str, Option<&'a str>),
    request: Cell<Option<String>>,
}

struct AnfrageEric<'a> {
    pages: std::cell::RefCell<VecDeque<&'a str>>,
    requests: std::cell::RefCell<Vec<String>>,
}

impl AuthenticatedEric for AnfrageEric<'_> {
    fn inspect<T>(
        &self,
        _: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(&str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        read(TEST_PROPERTIES)
    }

    fn send<T>(
        &self,
        xml: &CStr,
        version: &CStr,
        trace: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(i32, &str, Option<String>) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        assert_eq!(version, c"PostfachAnfrage_32");
        self.requests
            .borrow_mut()
            .push(xml.to_str().unwrap().to_owned());
        let xml = self
            .pages
            .borrow_mut()
            .pop_front()
            .expect("unexpected Anfrage");
        if let Some(events) = trace {
            events.push(TraceEvent {
                call: "EricBearbeiteVorgang",
                duration_ms: 3,
                page: None,
            });
        }
        read(0, xml, None)
    }
}

impl AuthenticatedEric for FakeEric<'_> {
    fn inspect<T>(
        &self,
        _: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(&str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        read(self.certificate)
    }

    fn send<T>(
        &self,
        xml: &CStr,
        version: &CStr,
        _: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(i32, &str, Option<String>) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        assert_eq!(version, c"PostfachStatus_32");
        assert!(
            self.request
                .replace(Some(xml.to_str().unwrap().to_owned()))
                .is_none()
        );
        let (code, xml, text) = self.reply;
        read(code, xml, text.map(str::to_owned))
    }
}

fn instant(date: &str) -> OffsetDateTime {
    OffsetDateTime::parse(date, &time::format_description::well_known::Rfc3339).unwrap()
}

fn fake<'a>(reply: (i32, &'a str, Option<&'a str>)) -> FakeEric<'a> {
    FakeEric {
        certificate: TEST_PROPERTIES,
        reply,
        request: Cell::new(None),
    }
}

fn query(scope: &Scope, eric: &FakeEric<'_>, test: bool) -> Result<Vec<Count>, Failure> {
    query_at(
        scope,
        eric,
        test,
        &mut Vec::new(),
        instant("2026-01-05T00:00:00Z"),
    )
}

fn query_at(
    scope: &Scope,
    eric: &FakeEric<'_>,
    test: bool,
    warnings: &mut Vec<config::Warning>,
    now: OffsetDateTime,
) -> Result<Vec<Count>, Failure> {
    Scope::preflight(eric, test, warnings, now, &mut None)?;
    scope.execute(eric, "12345", test, &mut None)
}

#[test]
fn sends_one_read_only_status_request_and_filters_the_returned_counts() {
    let scope = scope(&[
        "kiecli",
        "postfach",
        "status",
        "--alle",
        "--von",
        "2025-12-31",
        "--bis",
        "2026-01-02",
        "--datenart",
        "ESB",
        "--datenart",
        "USB",
    ]);
    let eric = fake((0, STATUS_REPLY, None));
    let mut warnings = Vec::new();
    let counts = query_at(
        &scope,
        &eric,
        true,
        &mut warnings,
        instant("2026-01-05T00:00:00Z"),
    )
    .unwrap_or_else(|err| panic!("{}", serde_json::to_value(err).unwrap()));
    assert_eq!(
        eric.request.take().as_deref(),
        Some(include_str!(
            "../../tests/fixtures/requests/postfach-status.xml"
        )),
    );
    let json = crate::describe::contract::stdout(
        "postfach status",
        &Output {
            results: Results::Status(StatusCounts {
                bereitstellungsdatenarten: counts,
                einschraenkung: scope.einschraenkung,
            }),
            test: true,
            profile: None,
            warnings,
            trace: None,
        },
    );
    assert_eq!(
        json["warnings"][0]["code"],
        "certificate_expires_soon"
    );
    assert_eq!(
        json["bereitstellungsdatenarten"],
        serde_json::json!([
            {"name":"ESB", "anzahl":7}
        ])
    );
}

#[test]
fn distinguishes_elster_rejection_from_transport_failure() {
    let scope = scope(&["kiecli", "postfach", "status"]);
    let reply = concat!(
        "<Elster><TransferHeader><RC><Rueckgabe><Code>190010801</Code><Text>Allgemeiner Fehler</Text></Rueckgabe></RC></TransferHeader>",
        "<DatenTeil><Nutzdatenblock><NutzdatenHeader><RC>",
        "<Rueckgabe><Code>0</Code></Rueckgabe>",
        "<Stack><Code>190017608</Code><Text>Zu viele Treffer</Text></Stack>",
        "<Stack><Code>190017609</Code><Text>Weitere Details</Text></Stack>",
        "</RC></NutzdatenHeader><Nutzdaten/></Nutzdatenblock></DatenTeil></Elster>"
    );
    let real_properties = TEST_PROPERTIES.replace("<Testzertifikat>true</Testzertifikat>", "");
    let eric = FakeEric {
        certificate: &real_properties,
        ..fake((610101210, reply, Some("Fehler im Transferheader")))
    };
    let error = query(&scope, &eric, false).err().unwrap();
    let request = eric.request.take().unwrap();
    assert!(!request.contains("Testmerker"));
    assert!(request.contains("einschraenkung=\"neue\""));
    assert_eq!(error.exit, 5);
    crate::describe::contract::stderr(&error);
    let error = serde_json::to_value(error).unwrap();
    assert_eq!(
        error["cause"][2],
        serde_json::json!({"source":"elster", "code":190017608, "text":"Zu viele Treffer"})
    );
    assert_eq!(error["cause"][3]["code"], 190017609);
    assert!(error["help"].as_str().unwrap().contains("--von"));

    let error = query(
        &scope,
        &fake((610101283, "", Some("Zeitüberschreitung"))),
        true,
    )
    .err()
    .unwrap();
    assert_eq!(error.exit, 6);
    crate::describe::contract::stderr(&error);
    assert_eq!(serde_json::to_value(error).unwrap()["retry"], true);
}

#[test]
fn rejects_a_success_without_a_status_answer() {
    let scope = scope(&["kiecli", "postfach", "status"]);
    let eric = fake((
        0,
        "<Elster><TransferHeader><RC><Rueckgabe><Code>0</Code></Rueckgabe></RC></TransferHeader></Elster>",
        None,
    ));
    let error = query(&scope, &eric, true).err().unwrap();
    crate::describe::contract::stderr(&error);
    assert_eq!(error.exit, 1);
}

#[test]
fn blocks_requests_outside_certificate_validity_and_in_live_mode_with_a_test_certificate() {
    let scope = scope(&["kiecli", "postfach", "status"]);
    for (at, test, expected) in [
        ("2023-12-31T23:59:59Z", true, "certificate_not_yet_valid"),
        ("2026-02-01T00:00:01Z", true, "certificate_expired"),
        (
            "2026-01-05T00:00:00Z",
            false,
            "test_certificate_requires_testmerker",
        ),
    ] {
        let eric = fake((0, STATUS_REPLY, None));
        let result = query_at(&scope, &eric, test, &mut Vec::new(), instant(at));
        let failure = result.err().unwrap();
        crate::describe::contract::stderr(&failure);
        assert_eq!(failure.code(), expected);
        assert!(eric.request.take().is_none());
    }
}

#[test]
fn anfrage_merges_pages_in_id_order_and_preserves_metadata() {
    let scope = scope(&[
        "kiecli",
        "postfach",
        "anfrage",
        "--alle",
        "--von",
        "2025-12-31",
        "--bis",
        "2026-01-02",
        "--datenart",
        "ESB",
        "--datenart",
        "EPMitteilung",
    ]);
    let last_page = r#"<Elster><TransferHeader/><DatenTeil><Nutzdatenblock><Nutzdaten>
        <Datenabholung><PostfachAnfrageAntwort einschraenkung="alle">
          <DatenartBereitstellung name="ESB">
            <Bereitstellungen><Bereitstellung id="be000000000000000000000000000003" groesse="0"/></Bereitstellungen>
          </DatenartBereitstellung>
          <DatenartBereitstellung name="ESB" rolle="vertreter">
            <Bereitstellungen><Bereitstellung id="be000000000000000000000000000004" groesse="0"/></Bereitstellungen>
          </DatenartBereitstellung>
        </PostfachAnfrageAntwort></Datenabholung>
        </Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#;
    let eric = AnfrageEric {
        pages: VecDeque::from([ANFRAGE_REPLY, last_page]).into(),
        requests: Default::default(),
    };
    let mut trace = Some(Vec::new());
    let nachrichten = anfrage::execute(&scope, &eric, "12345", true, &mut trace)
        .unwrap_or_else(|err| panic!("{}", serde_json::to_value(err).unwrap()));
    let golden = include_str!("../../tests/fixtures/requests/postfach-anfrage.xml");
    assert_eq!(
        eric.requests.into_inner(),
        [
            golden.to_owned(),
            golden.replace(
                "max=\"10000\"",
                "max=\"10000\" paginierungLetzteBestId=\"be000000000000000000000000000002\""
            ),
        ]
    );
    let json = crate::describe::contract::stdout(
        "postfach anfrage",
        &Output {
            results: Results::Anfrage(Messages {
                postfach_nachrichten: nachrichten,
            }),
            test: true,
            profile: None,
            warnings: Vec::new(),
            trace,
        },
    );
    assert_eq!(json["trace"][0]["page"], 1);
    assert_eq!(json["trace"][1]["page"], 2);
    let messages = json["postfach_nachrichten"].as_array().unwrap();
    assert_eq!(
        messages
            .iter()
            .map(|message| message["id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "be000000000000000000000000000001",
            "be000000000000000000000000000002",
            "be000000000000000000000000000003"
        ],
    );
    assert_eq!(messages[0]["datenpaket"], serde_json::Value::Null);
    assert_eq!(messages[0]["anschreiben"], serde_json::Value::Null);
    assert_eq!(
        messages[1],
        serde_json::json!({
            "id": "be000000000000000000000000000002",
            "bereitstellungsdatenart": "EPMitteilung",
            "bereitstellungsticket": "et000000000000000000000000000002",
            "verfuegbar_bis": "2026-06-30",
            "groesse": 1400,
            "meta": {"extra": "A & B", "verfuegbarbis": "20260630"},
            "anschreiben": {"sendername": "Finanzamt Beispiel", "bezeichnung": "Nachricht", "text": "Grüße!"},
            "meta_maps": [{"typ": "DIVA2", "meta": [{"name": "dokumenttyp", "value": "BEISPIEL"}],
                "meta_lists": [{"typ": "EXTRA", "elements": [{"name": "Eintrag", "value": "Wert"}]}]}],
            "datenpaket": {"objektid": "00000000-0000-0000-0000-000000000001", "encoding": "UTF-8", "groesse": 600},
            "anhaenge": [{"position": 1, "dateiname": "Bescheid.pdf", "dateibezeichnung": "Bescheid", "dateityp": "PDF",
                "objektid": "00000000-0000-0000-0000-000000000002", "groesse": 800,
                "meta": [{"name": "Dokument", "value": "Beispiel"}]}]
        })
    );
}

#[test]
fn anfrage_defaults_to_all_supported_datenarten_and_rejects_a_stalled_cursor() {
    let scope = scope(&["kiecli", "postfach", "anfrage", "--alle"]);
    let eric = AnfrageEric {
        pages: VecDeque::from([ANFRAGE_REPLY, ANFRAGE_REPLY]).into(),
        requests: Default::default(),
    };
    let error = anfrage::execute(&scope, &eric, "12345", true, &mut None)
        .err()
        .unwrap();
    assert_eq!(error.code(), "postfach_anfrage_invalid");
    crate::describe::contract::stderr(&error);
    let requests = eric.requests.into_inner();
    assert_eq!(requests[0].matches("<DatenartBereitstellung ").count(), 49);
    assert!(requests[0].contains(r#"name="DivaBescheidUSt""#));
    assert!(!requests[0].contains(r#"name="USTGS""#));
}
