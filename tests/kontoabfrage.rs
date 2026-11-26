// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::{
    env, fs,
    path::Path,
    process::{Command, Output},
};

use serde_json::Value;
use tempfile::TempDir;

const ELSTER_STEUERNUMMER: &str = "9199026211407";
const BESCHEID_STEUERNUMMER: &str = "919/902/62114";

fn root(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kiecli"));
    command
        .env_clear()
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .env("KIECLI_HERSTELLER_ID", "12345")
        .current_dir(home)
        .args(["kontoabfrage"]);
    command
}

fn cli(home: &Path) -> Command {
    let mut command = root(home);
    command.arg("offene-betraege");
    command
}

fn failure(output: Output, exit: i32, code: &str) -> Value {
    assert_eq!(
        output.status.code(),
        Some(exit),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    let report: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(report["error"]["code"], code, "{report}");
    report["error"].clone()
}

#[test]
fn requires_a_steuernummer() {
    let home = TempDir::new().unwrap();
    failure(
        cli(home.path()).output().unwrap(),
        2,
        "steuernummer_missing",
    );
}

#[test]
fn requires_bundesland_for_a_bescheid_format_number() {
    let home = TempDir::new().unwrap();
    let output = root(home.path())
        .args(["offene-betraege", "--steuernummer", BESCHEID_STEUERNUMMER])
        .output()
        .unwrap();
    failure(output, 2, "bundesland_required");
}

#[test]
fn rejects_an_unknown_bundesland_without_loading_credentials() {
    let home = TempDir::new().unwrap();
    let output = root(home.path())
        .args([
            "offene-betraege",
            "--steuernummer",
            BESCHEID_STEUERNUMMER,
            "--bundesland",
            "XX",
        ])
        .output()
        .unwrap();
    failure(output, 2, "bundesland_invalid");
}
#[test]
fn accepts_case_insensitive_steuerart_values() {
    let home = TempDir::new().unwrap();
    let output = root(home.path())
        .args([
            "zahlungen",
            "--steuernummer",
            ELSTER_STEUERNUMMER,
            "--ab",
            "2026-01-02",
            "--steuerart",
            "kApEsT",
        ])
        .output()
        .unwrap();
    let error = failure(output, 3, "certificate_not_found");
    assert_ne!(error["code"], "usage_error");
}

#[test]
fn rejects_invalid_steuerart_values() {
    let home = TempDir::new().unwrap();
    let output = root(home.path())
        .args([
            "zahlungen",
            "--steuernummer",
            ELSTER_STEUERNUMMER,
            "--ab",
            "2026-01-02",
            "--steuerart",
            "not-a-tax",
        ])
        .output()
        .unwrap();
    failure(output, 2, "steuerart_unknown");
}

#[test]
fn rejects_more_than_75_sollstellungen_parts() {
    let home = TempDir::new().unwrap();
    let args = vec![
        "sollstellungen",
        "--steuernummer",
        ELSTER_STEUERNUMMER,
        "--steuerart",
        "ESt",
        "--steuerart",
        "KSt",
        "--steuerart",
        "USt",
        "--steuerart",
        "LSt",
        "--steuerart",
        "GewSt",
        "--steuerart",
        "ZaSt",
        "--steuerart",
        "KapESt",
    ];
    let years = (2017..=2027)
        .map(|year| year.to_string())
        .collect::<Vec<_>>();
    let mut owned_args = args.into_iter().map(str::to_owned).collect::<Vec<_>>();
    for year in &years {
        owned_args.push("--jahr".to_owned());
        owned_args.push(year.clone());
    }
    let output = root(home.path()).args(owned_args).output().unwrap();
    failure(output, 2, "usage_error");
}

#[test]
fn validates_required_query_options() {
    let home = TempDir::new().unwrap();
    let payments = root(home.path())
        .args(["zahlungen", "--steuernummer", ELSTER_STEUERNUMMER])
        .output()
        .unwrap();
    failure(payments, 2, "usage_error");

    let assessments = root(home.path())
        .args([
            "sollstellungen",
            "--steuernummer",
            ELSTER_STEUERNUMMER,
            "--steuerart",
            "ESt",
        ])
        .output()
        .unwrap();
    failure(assessments, 2, "usage_error");
}

#[test]
fn command_flags_override_profile_taxpayer_settings() {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("settings");
    fs::create_dir_all(&config_dir).unwrap();
    let config = config_dir.join("kiecli.toml");
    fs::write(
        &config,
        "default_profile = 'firma'\n[profile.firma]\nsteuernummer = 'bad-profile-value'\nbundesland = 'XX'\n",
    )
    .unwrap();

    let output = cli(home.path())
        .env("KIECLI_CONFIG", &config)
        .args(["--steuernummer", ELSTER_STEUERNUMMER])
        .output()
        .unwrap();
    let error = failure(output, 3, "certificate_not_found");
    assert_ne!(error["code"], "bundesland_invalid");
}

#[test]
#[ignore = "requires official ERiC 44 and a test Certificate; no network is intended"]
fn refuses_the_official_test_certificate_in_live_mode() {
    let home = TempDir::new().unwrap();
    let output = cli(home.path())
        .args(["--no-test", "--steuernummer", ELSTER_STEUERNUMMER])
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
    failure(output, 3, "test_certificate_requires_testmerker");
}
