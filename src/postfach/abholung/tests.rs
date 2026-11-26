// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::cell::Cell;
use std::ffi::CStr;
use std::fs;

use clap::Parser;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;

use super::*;

const ID: &str = "be000000000000000000000000000002";
const PACKAGE: &[u8] = b"\xef\xbb\xbf\n<Example/>";
const REPLY: &str = include_str!("../../../tests/fixtures/elster/postfach-anfrage.xml");

struct Fake<'a> {
    out: &'a Path,
    queried: Cell<bool>,
    confirmed: Cell<bool>,
    fail_attachment: bool,
}

impl AuthenticatedEric for Fake<'_> {
    fn inspect<T>(
        &self,
        _: &mut Option<Vec<TraceEvent>>,
        _: impl FnOnce(&str) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        unreachable!("tested below the common Certificate preflight")
    }

    fn send<T>(
        &self,
        xml: &CStr,
        version: &CStr,
        _: &mut Option<Vec<TraceEvent>>,
        read: impl FnOnce(i32, &str, Option<String>) -> Result<T, Failure>,
    ) -> Result<T, Failure> {
        let xml = xml.to_str().unwrap();
        assert!(xml.contains("<Testmerker>700000001</Testmerker>"));
        match version {
            v if v == c"PostfachAnfrage_32" => {
                assert!(
                    !self.queried.replace(true),
                    "explicit ID already reached by the cursor"
                );
                assert!(xml.contains("einschraenkung=\"alle\""));
                let reply = REPLY.replace("Bescheid.pdf", "../Bescheid.pdf");
                read(0, &reply, None)
            }
            v if v == c"PostfachBestaetigung_32" => {
                assert!(
                    !self.fail_attachment,
                    "partial Abholung must never be confirmed"
                );
                assert!(!self.confirmed.replace(true));
                assert!(xml.contains(&format!("<Bereitstellung id=\"{ID}\"/>")));
                let dir = self.out.join(ID);
                assert_eq!(fs::read(dir.join("datenpaket.xml")).unwrap(), PACKAGE);
                assert_eq!(
                    fs::read(dir.join("anhaenge/01-.._Bescheid.pdf")).unwrap(),
                    b"abc"
                );
                read(
                    0,
                    &format!(
                        concat!(
                            "<Elster><TransferHeader/><DatenTeil><Nutzdatenblock><Nutzdaten>",
                            "<Datenabholung><PostfachBestaetigungAntwort><Bereitstellungen>",
                            "<Bereitstellung id=\"{}\"><Rueckgabe><Code>0</Code><Text>OK</Text></Rueckgabe></Bereitstellung>",
                            "</Bereitstellungen></PostfachBestaetigungAntwort></Datenabholung>",
                            "</Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"
                        ),
                        ID
                    ),
                    None,
                )
            }
            _ => panic!("unexpected operation: {version:?}"),
        }
    }
}

impl ObjectFetcher for Fake<'_> {
    fn fetch(
        &self,
        id: &str,
        transport_size: u64,
        out: &mut dyn Write,
        _: &mut Option<Vec<TraceEvent>>,
    ) -> Result<(), Failure> {
        assert!(self.queried.get());
        match id {
            "00000000-0000-0000-0000-000000000001" => {
                assert_eq!(transport_size, 600);
                out.write_all(&PACKAGE[..1]).unwrap();
                out.write_all(&PACKAGE[1..]).unwrap();
                Ok(())
            }
            "00000000-0000-0000-0000-000000000002" => {
                assert_eq!(transport_size, 800);
                if self.fail_attachment {
                    out.write_all(b"incomplete").unwrap();
                    Err(Failure::transient(
                        "otto_transport_failed",
                        "Transport interrupted",
                    ))
                } else {
                    out.write_all(b"abc").unwrap();
                    Ok(())
                }
            }
            _ => panic!("unexpected fetch: {id}"),
        }
    }
}

fn run(fake: &Fake<'_>, confirmation: &str) -> Vec<Collected> {
    run_id(fake, ID, confirmation)
}

fn run_id(fake: &Fake<'_>, id: &str, confirmation: &str) -> Vec<Collected> {
    let cli = Cli::try_parse_from([
        "kiecli",
        "postfach",
        "abholung",
        "--id",
        id,
        "--out",
        fake.out.to_str().unwrap(),
        confirmation,
        "--mit-meta",
        "--test",
    ])
    .unwrap();
    let crate::Command::Postfach {
        command: crate::postfach::Command::Abholung(options),
    } = &cli.command
    else {
        panic!("expected abholung");
    };
    let scope = options.scope(&cli).unwrap_or_else(fail);
    let rows = execute(options, &scope, fake, fake, "12345", true, &mut None).unwrap_or_else(fail);
    super::super::validate_output(
        "postfach abholung",
        super::super::Messages {
            postfach_nachrichten: rows.iter().collect(),
        },
    );
    rows
}

fn checked_outcome(rows: Vec<Collected>) -> Result<(), Failure> {
    outcome(rows).inspect_err(crate::describe::contract::stderr)
}

fn fail<T>(failure: Failure) -> T {
    panic!("{}", serde_json::to_value(failure).unwrap());
}

#[test]
fn abholung_stores_plaintext_and_confirms_after_both_files_exist() {
    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("new/nested");
    let fake = Fake {
        out: &out,
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: false,
    };
    let rows = run(&fake, "--bestaetigen");
    let json = serde_json::to_value(&rows).unwrap();
    let entry = &json[0];
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(entry["id"], ID);
    assert_eq!(entry["status"], "vollstaendig");
    assert_eq!(entry["bestaetigt"], true);
    assert_eq!(entry["bestaetigung_faellig_bis"], Value::Null);
    assert_eq!(entry["bereitstellungsdatenart"], "EPMitteilung");
    assert_eq!(entry["files"][0]["encoding"], "UTF-8");
    assert_eq!(entry["files"][0]["groesse"], PACKAGE.len());
    assert_eq!(entry["files"][1]["groesse"], 3);
    assert_eq!(
        entry["files"][1]["sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    checked_outcome(rows).unwrap_or_else(fail);
}

#[test]
fn abholung_keeps_previous_file_and_deadline_after_partial_transfer() {
    let out = tempfile::tempdir().unwrap();
    let attachments = out.path().join(ID).join("anhaenge");
    fs::create_dir_all(&attachments).unwrap();
    let attachment = attachments.join("01-.._Bescheid.pdf");
    fs::write(&attachment, b"previous bytes").unwrap();
    let fake = Fake {
        out: out.path(),
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: true,
    };
    let rows = run(&fake, "--bestaetigen");
    let json = serde_json::to_value(&rows).unwrap();
    let entry = &json[0];
    assert_eq!(entry["status"], "teilweise");
    assert_eq!(entry["bestaetigt"], false);
    let time = |key: &str| OffsetDateTime::parse(entry[key].as_str().unwrap(), &Rfc3339).unwrap();
    assert_eq!(
        time("bestaetigung_faellig_bis") - time("abgeholt_am"),
        Duration::hours(24)
    );
    assert_eq!(fs::read(attachment).unwrap(), b"previous bytes");
    assert_eq!(fs::read_dir(attachments).unwrap().count(), 1);
    assert_eq!(checked_outcome(rows).err().unwrap().exit, 7);
}

#[test]
fn abholung_does_not_follow_symlinks_at_message_path() {
    let out = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(other.path(), out.path().join(ID)).unwrap();
    let fake = Fake {
        out: out.path(),
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: false,
    };
    let rows = run(&fake, "--bestaetigen");
    let json = serde_json::to_value(&rows).unwrap();
    assert_eq!(json[0]["status"], "fehlgeschlagen");
    assert_eq!(json[0]["bestaetigung_faellig_bis"], Value::Null);
    assert_eq!(
        checked_outcome(rows).err().unwrap().code(),
        "postfach_write_failed"
    );
    assert!(fs::read_dir(other.path()).unwrap().next().is_none());
}

#[test]
fn abholung_can_defer_confirmation_of_completed_files() {
    let out = tempfile::tempdir().unwrap();
    let fake = Fake {
        out: out.path(),
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: false,
    };
    let rows = run(&fake, "--ohne-bestaetigung");
    let json = serde_json::to_value(&rows).unwrap();
    assert_eq!(json[0]["status"], "vollstaendig");
    assert_eq!(json[0]["bestaetigt"], false);
    assert!(json[0]["bestaetigung_faellig_bis"].is_string());
    assert!(!fake.confirmed.get());
    checked_outcome(rows).unwrap_or_else(fail);
}

#[test]
fn abholung_reports_a_nachricht_without_files_as_not_collected() {
    let out = tempfile::tempdir().unwrap();
    let fake = Fake {
        out: out.path(),
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: false,
    };
    let rows = run_id(&fake, "be000000000000000000000000000001", "--bestaetigen");
    let json = serde_json::to_value(&rows).unwrap();
    assert_eq!(json[0]["status"], "fehlgeschlagen");
    assert_eq!(json[0]["abgeholt_am"], Value::Null);
    assert_eq!(json[0]["bestaetigung_faellig_bis"], Value::Null);
    assert!(!fake.confirmed.get());
    assert_eq!(
        checked_outcome(rows).err().unwrap().code(),
        "postfach_nachricht_no_objects"
    );
}

#[test]
fn abholung_refuses_a_message_directory_writable_by_others() {
    use std::os::unix::fs::PermissionsExt;

    let out = tempfile::tempdir().unwrap();
    let message = out.path().join(ID);
    fs::create_dir(&message).unwrap();
    fs::set_permissions(&message, fs::Permissions::from_mode(0o770)).unwrap();
    let fake = Fake {
        out: out.path(),
        queried: Cell::default(),
        confirmed: Cell::default(),
        fail_attachment: false,
    };
    let rows = run(&fake, "--bestaetigen");
    assert_eq!(
        checked_outcome(rows).err().unwrap().code(),
        "postfach_write_failed"
    );
    assert!(!fake.confirmed.get());
    assert!(fs::read_dir(message).unwrap().next().is_none());
}
