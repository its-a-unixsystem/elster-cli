// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::{Value, json};
use tempfile::TempDir;

fn cli(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kiecli"));
    command
        .env_clear()
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .current_dir(home)
        .args(["config", "show"]);
    command
}

fn result(output: Output) -> Value {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn shows_resolved_values_and_their_sources_without_using_credentials() {
    let home = TempDir::new().unwrap();
    let home = home.path();
    let file = home.join("settings/kiecli.toml");
    fs::create_dir(file.parent().unwrap()).unwrap();
    fs::write(
        &file,
        "hersteller_id = '00123'\ndefault_profile = 'firma'\n\
        [profile.firma]\ncertificate = 'old.pfx'\npin_command = 'touch pin-was-run'\n\
        test = true\nsteuernummer = '001/002/00003'\nbundesland = 'BY'\n",
    )
    .unwrap();
    let response = result(
        cli(home)
            .env("KIECLI_CONFIG", "settings/kiecli.toml")
            .env("KIECLI_CERTIFICATE", "~/missing.pfx")
            .env("KIECLI_STEUERNUMMER", "ignored")
            .args(["--steuernummer", "001/002/00004", "--no-test", "-v"])
            .output()
            .unwrap(),
    );
    let source = format!("file:{}", file.display());
    assert_eq!(response["config"], json!({"value": file, "source": "env"}));
    assert_eq!(response["trace"], json!([]));
    assert_eq!(response["profile"], "firma");
    assert_eq!(response["profile_source"], source);
    assert_eq!(response["test"], false);
    assert_eq!(response["test_source"], "flag");
    assert_eq!(
        response["certificate"],
        json!({"value": home.join("missing.pfx"), "source": "env"})
    );
    assert_eq!(
        response["eric_dir"],
        json!({"value": home.join("data/kiecli/eric"), "source": "default"})
    );
    assert_eq!(
        response["pin"],
        json!({"value": "command", "source": source})
    );
    assert_eq!(
        response["hersteller_id"],
        json!({"value": "00123", "source": source})
    );
    assert_eq!(
        response["steuernummer"],
        json!({"value": "001/002/00004", "source": "flag"})
    );
    assert_eq!(
        response["bundesland"],
        json!({"value": "BY", "source": source})
    );
    let warnings = response["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{response}");
    assert_eq!(warnings[0]["code"], "env_overrides_profile");
    assert!(
        warnings[0]["message"]
            .as_str()
            .unwrap()
            .contains("certificate")
    );
    assert!(!home.join("pin-was-run").exists());
    assert!(!response.to_string().contains("pin-was-run"));

    let response = result(
        cli(home)
            .args(["--config", "settings/kiecli.toml"])
            .env("KIECLI_TEST", "0")
            .output()
            .unwrap(),
    );
    assert_eq!(
        response["certificate"],
        json!({"value": home.join("settings/old.pfx"), "source": source})
    );
    assert_eq!(response["test"], false);
    assert_eq!(response["test_source"], "env");
    assert_eq!(response["warnings"][0]["code"], "env_overrides_profile");
    assert!(
        response["warnings"][0]["message"]
            .as_str()
            .unwrap()
            .contains("KIECLI_TEST")
    );
    assert!(!home.join("pin-was-run").exists());
}

#[test]
fn rejects_conflicting_pin_flags_across_subcommands_without_echoing_them() {
    let output = Command::new(env!("CARGO_BIN_EXE_kiecli"))
        .args([
            "--pin-command",
            "command-secret",
            "config",
            "show",
            "--pin-file",
            "file-secret",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(!error.contains("command-secret"));
    let report: Value = serde_json::from_str(&error).unwrap();
    assert_eq!(
        report["error"]["message"],
        "Use either --pin-file or --pin-command, not both"
    );
    assert!(!error.contains("file-secret"));
}

#[test]
fn pin_override_discloses_only_the_selected_channel_in_both_formats() {
    let home = TempDir::new().unwrap();
    let home = home.path();
    let direct_pin = "synthetic-secret";
    let pin_file = "nonexistent-secret-path";
    let output = cli(home)
        .env("KIECLI_PIN", direct_pin)
        .args(["--pin-file", pin_file])
        .output()
        .unwrap();
    let response = result(output);
    assert_eq!(response["pin"], json!({"value": "file", "source": "flag"}));
    assert!(!response.to_string().contains(direct_pin));
    assert!(!response.to_string().contains(pin_file));

    let output = cli(home)
        .env("KIECLI_PIN", direct_pin)
        .args(["--format", "text"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("pin: env (env)"), "{text}");
    assert!(!text.contains(direct_pin));
}

#[test]
fn text_escapes_control_characters_in_settings() {
    let home = TempDir::new().unwrap();
    let output = cli(home.path())
        .env("KIECLI_BUNDESLAND", "BY\nwarning:\x1b[2J")
        .args(["--format", "text"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains(r"bundesland: BY\nwarning:\u{1b}[2J (env)"));
    assert!(!text.contains("\nwarning:"));
    assert!(!text.contains('\x1b'));
}

#[test]
fn invalid_effective_hersteller_id_is_rejected_without_echoing_it() {
    let home = TempDir::new().unwrap();
    let output = cli(home.path())
        .env("KIECLI_HERSTELLER_ID", "not-an-id")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{output:?}");
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(!error.contains("not-an-id"));
    let error: Value = serde_json::from_str(&error).unwrap();
    assert_eq!(error["error"]["code"], "hersteller_id_invalid");
}
