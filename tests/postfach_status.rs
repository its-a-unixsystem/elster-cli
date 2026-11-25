// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::env;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn cli(home: &TempDir) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kiecli"));
    command
        .env_clear()
        .env("HOME", home.path())
        .args(["postfach", "status"]);
    command
}

fn error(output: Output, exit: i32) -> Value {
    assert!(output.stdout.is_empty());
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<Value>(&output.stderr).unwrap()["error"].take()
}

#[test]
fn corrects_datenart_case_before_resolving_credentials() {
    let home = TempDir::new().unwrap();
    let output = cli(&home)
        .args([
            "--test",
            "--von",
            "2026-01-02",
            "--datenart",
            "DivaBescheidUst",
            "--datenart",
            "ESB",
            "--profile",
            "test company",
        ])
        .output()
        .unwrap();
    let value = error(output, 2);
    assert_eq!(value["code"], "datenart_unknown");
    assert_eq!(
        value["suggestion"],
        "kiecli postfach status --test --von 2026-01-02 --datenart=DivaBescheidUSt --datenart=ESB --profile='test company'"
    );
}

#[test]
fn anfrage_correction_keeps_the_requested_command_and_scope() {
    let home = TempDir::new().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_kiecli"))
        .env_clear()
        .env("HOME", home.path())
        .args([
            "postfach",
            "anfrage",
            "--alle",
            "--datenart",
            "epmitteilung",
        ])
        .output()
        .unwrap();
    let value = error(output, 2);
    assert_eq!(value["code"], "datenart_unknown");
    assert_eq!(
        value["suggestion"],
        "kiecli postfach anfrage --alle --datenart=EPMitteilung"
    );
}

#[test]
fn correction_does_not_echo_a_pin_command() {
    let home = TempDir::new().unwrap();
    let output = cli(&home)
        .args([
            "--pin-command",
            "printf secret-marker",
            "--datenart",
            "DivaBescheidUst",
        ])
        .output()
        .unwrap();
    let value = error(output, 2);
    assert!(value.get("suggestion").is_none());
    assert!(!value.to_string().contains("secret-marker"));
    assert!(value["help"].as_str().unwrap().contains("DivaBescheidUSt"));
}

#[test]
fn refuses_conflicting_scope_flags_and_invalid_calendar_ranges() {
    let home = TempDir::new().unwrap();
    for args in [
        &["--alle", "--neue"][..],
        &["--von", "2025-02-29"][..],
        &["--von", "2025-03-01", "--bis", "2025-02-28"][..],
    ] {
        let output = cli(&home).args(args).output().unwrap();
        assert_eq!(error(output, 2)["code"], "usage_error");
    }
}

#[test]
fn abholung_requires_an_explicit_confirmation_choice_and_safe_selection() {
    let home = TempDir::new().unwrap();
    for args in [
        &["--neue", "--out", "unused"][..],
        &[
            "--neue",
            "--out",
            "unused",
            "--bestaetigen",
            "--ohne-bestaetigung",
        ][..],
        &[
            "--neue",
            "--id",
            "be_example",
            "--out",
            "unused",
            "--bestaetigen",
        ][..],
        &["--id", "../escape", "--out", "unused", "--bestaetigen"][..],
        &["--neue", "--bestaetigen"][..],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_kiecli"))
            .env_clear()
            .env("HOME", home.path())
            .args(["postfach", "abholung"])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(error(output, 2)["code"], "usage_error");
    }
    assert_eq!(
        error(
            Command::new(env!("CARGO_BIN_EXE_kiecli"))
                .env_clear()
                .env("HOME", home.path())
                .args(["postfach", "bestaetigung"])
                .output()
                .unwrap(),
            2,
        )["code"],
        "usage_error"
    );
}

#[test]
#[ignore = "requires official ERiC 44 and a test Certificate; rejection must be local"]
fn rejects_a_test_certificate_in_live_mode() {
    let home = TempDir::new().unwrap();
    let output = cli(&home)
        .args(["--no-test", "--von", "2026-01-02"])
        .env(
            "KIECLI_ERIC_DIR",
            env::var_os("KIECLI_TEST_ERIC_HOME").unwrap(),
        )
        .env(
            "KIECLI_CERTIFICATE",
            env::var_os("KIECLI_TEST_CERTIFICATE").unwrap(),
        )
        .env("KIECLI_PIN", "123456")
        .env("KIECLI_HERSTELLER_ID", "12345")
        .output()
        .unwrap();
    let value = error(output, 3);
    assert_eq!(value["code"], "test_certificate_requires_testmerker");
    assert_eq!(
        value["suggestion"],
        "kiecli postfach status --test --von 2026-01-02"
    );
}
