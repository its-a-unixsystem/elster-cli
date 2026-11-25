// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::{
    env, fs,
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;
use tempfile::TempDir;

fn cli(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kiecli"));
    command
        .env_clear()
        .env("HOME", home)
        .current_dir(home)
        .args(["zertifikat", "status"]);
    command
}

fn credentials_error(output: Output, code: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(3),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stdout.is_empty());
    let report: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(report["error"]["code"], code);
    assert_eq!(report["error"]["retry"], false);
    report
}

#[test]
fn refuses_missing_certificate_before_running_pin_command_or_loading_eric() {
    let home = TempDir::new().unwrap();
    credentials_error(
        cli(home.path())
            .args([
                "--certificate",
                "missing.pfx",
                "--pin-command",
                "touch pin-was-run",
            ])
            .output()
            .unwrap(),
        "certificate_not_found",
    );
    assert!(!home.path().join("pin-was-run").exists());
}

#[test]
fn reports_pin_command_failure_without_revealing_its_stdout() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("available.pfx"), []).unwrap();
    let report = credentials_error(
        cli(home.path())
            .args([
                "--certificate",
                "available.pfx",
                "--pin-command",
                "printf 'fake-pin\\n'; printf 'vault denied fake-pin\\n' >&2; exit 13",
            ])
            .output()
            .unwrap(),
        "pin_command_failed",
    );
    assert_eq!(report["error"]["cause"][0]["code"], 13);
    assert_eq!(
        report["error"]["cause"][0]["text"],
        "vault denied [redacted]\n"
    );
    assert!(!report.to_string().contains("fake-pin"));
}

#[test]
fn withholds_pin_command_stderr_when_there_is_no_pin_to_redact() {
    let home = TempDir::new().unwrap();
    fs::write(home.path().join("available.pfx"), []).unwrap();
    let report = credentials_error(
        cli(home.path())
            .args([
                "--certificate",
                "available.pfx",
                "--pin-command",
                "printf 'secret-on-the-wrong-stream\\n' >&2; exit 13",
            ])
            .output()
            .unwrap(),
        "pin_command_failed",
    );
    assert_eq!(report["error"]["cause"][0]["code"], 13);
    assert!(report["error"]["cause"][0].get("text").is_none());
}

#[test]
#[ignore = "requires official ERiC 44 and the official test Organisationszertifikat; no network"]
fn inspects_an_official_test_certificate_offline() {
    let home = TempDir::new().unwrap();
    let eric = env::var_os("KIECLI_TEST_ERIC_HOME").expect("KIECLI_TEST_ERIC_HOME");
    let certificate = env::var_os("KIECLI_TEST_CERTIFICATE").expect("KIECLI_TEST_CERTIFICATE");
    let mut command = cli(home.path());
    command
        .env("KIECLI_ERIC_DIR", eric)
        .env("KIECLI_CERTIFICATE", certificate);
    command.args(["--config", "config.toml"]);
    fs::write(
        home.path().join("config.toml"),
        "[profile.firma]\npin_file = 'pin'\n",
    )
    .unwrap();
    let pin = home.path().join("pin");
    fs::write(&pin, "123456\n").unwrap();
    fs::set_permissions(&pin, fs::Permissions::from_mode(0o644)).unwrap();
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["profile"], "firma");
    assert_eq!(report["test"], false, "inspection must not switch modes");
    assert_eq!(report["testzertifikat"], true);
    assert_eq!(report["organisationszertifikat"], true);
    assert!(
        report["gueltig_bis"]
            .as_str()
            .unwrap()
            .starts_with("2028-07-10T")
    );
    assert_eq!(report["signaturzertifikat"]["schluessellaenge"], 3072);
    assert_eq!(
        report["verschluesselungszertifikat"]["subjekt"][0]["name"],
        "CN"
    );
    assert!(
        report["konto_id"]
            .as_str()
            .is_some_and(|id| id.len() == 10 && id.bytes().all(|byte| byte.is_ascii_digit()))
    );
    assert!(report["signaturzertifikat"].get("public_key_md5").is_none());
    assert!(
        report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning["code"] == "pin_file_permissions")
    );

    let wrong_pin = credentials_error(
        command.env("KIECLI_PIN", "654321").output().unwrap(),
        "pin_wrong",
    );
    assert_eq!(wrong_pin["error"]["cause"][0]["code"], 610201106);
    assert!(!wrong_pin.to_string().contains("654321"));

    let output = command
        .args(["--format", "text", "--pin-command", "printf '123456\\n'"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Registrant type: Organisation"));
}
