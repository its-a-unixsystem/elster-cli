// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::cell::Cell;
use std::ffi::CStr;

use clap::Parser;

use super::*;

const STATUS_REPLY: &str = include_str!("../../tests/fixtures/elster/postfach-status.xml");

fn scope(args: &[&str]) -> Scope {
    let cli = Cli::try_parse_from(args).unwrap();
    let crate::Command::Postfach {
        command: Command::Status(options),
    } = &cli.command
    else {
        panic!("expected postfach status");
    };
    Scope::new(&cli, options).unwrap_or_else(|err| panic!("{}", serde_json::to_value(err).unwrap()))
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
    scope.execute(
        eric,
        "12345",
        test,
        &mut Vec::new(),
        instant("2026-01-05T00:00:00Z"),
        &mut None,
    )
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
    let counts = scope
        .execute(
            &eric,
            "12345",
            true,
            &mut warnings,
            instant("2026-01-05T00:00:00Z"),
            &mut None,
        )
        .unwrap_or_else(|err| panic!("{}", serde_json::to_value(err).unwrap()));
    assert_eq!(
        eric.request.take().as_deref(),
        Some(include_str!(
            "../../tests/fixtures/requests/postfach-status.xml"
        )),
    );
    assert_eq!(
        serde_json::to_value(warnings).unwrap()[0]["code"],
        "certificate_expires_soon"
    );
    assert_eq!(
        serde_json::to_value(counts).unwrap(),
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
        let result = scope.execute(
            &eric,
            "12345",
            test,
            &mut Vec::new(),
            instant(at),
            &mut None,
        );
        assert_eq!(result.err().map(|err| err.code()), Some(expected));
        assert!(eric.request.take().is_none());
    }
}
