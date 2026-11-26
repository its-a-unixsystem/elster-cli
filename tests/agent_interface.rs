// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::process::{Command, Output};

use serde_json::{Value, json};

fn cli(args: &[&str]) -> Output {
    let home = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_kiecli"))
        .env_clear()
        .current_dir(home.path())
        .env("KIECLI_CONFIG", "missing-config.toml")
        .env("KIECLI_PIN_COMMAND", "exit 99")
        .args(args)
        .output()
        .unwrap()
}

fn describe(args: &[&str]) -> Value {
    let output = cli(&[&["describe"], args].concat());
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn describe_exposes_abholung_confirmation_choices_and_json_contract() {
    let value = describe(&["postfach", "abholung"]);
    let command = &value["commands"][0];
    assert_eq!(command["path"], "postfach abholung");
    assert_eq!(value["commands"].as_array().unwrap().len(), 1);
    let group = command["exclusive_groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|group| group["name"] == "confirmation")
        .unwrap();
    assert_eq!(group["required"], true);
    assert_eq!(
        group["arguments"],
        json!(["bestaetigen", "ohne_bestaetigung"])
    );
    let destination = command["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|argument| argument["long"] == "out")
        .unwrap();
    assert_eq!(destination["required"], true);
    assert_eq!(destination["value_type"], "path");
    assert!(command["stdout_schema"]["properties"]["postfach_nachrichten"].is_object());
    assert!(value["stderr_schema"]["properties"]["error"].is_object());
}

#[test]
fn describe_lists_offline_and_remote_commands_and_accepts_a_quoted_path() {
    let value = describe(&[]);
    let commands = value["commands"].as_array().unwrap();
    for (path, format) in [("skill", "markdown"), ("kontoabfrage zahlungen", "json")] {
        assert!(
            commands
                .iter()
                .any(|command| command["path"] == path && command["stdout_format"] == format)
        );
    }
    for command in commands.iter().filter(|command| command["stdout_format"] == "json") {
        assert!(command["stdout_schema"].is_object(), "{}", command["path"]);
    }
    let value = describe(&["postfach status"]);
    let command = &value["commands"][0];
    assert_eq!(command["path"], "postfach status");
    assert!(command["about"].as_str().unwrap().contains("hint"));
    assert!(
        value["bereitstellungsdatenarten"]
            .as_array()
            .unwrap()
            .contains(&json!("EPBescheid"))
    );
}

#[test]
fn describe_reports_effective_defaults_and_retry_codes() {
    let value = describe(&["-v"]);
    assert_eq!(value["trace"], json!([]));
    let commands = value["commands"].as_array().unwrap();
    for (path, flag, default) in [
        ("postfach status", "neue", "true"),
        ("postfach anfrage", "neue", "true"),
        ("kontoabfrage zahlungen", "steuerart", "alle"),
    ] {
        let command = commands.iter().find(|command| command["path"] == path).unwrap();
        let arg = command["arguments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|arg| arg["long"] == flag)
            .unwrap();
        assert_eq!(arg["default_values"], json!([default]));
    }
    for code in [950017008, 950019011] {
        let row = value["kontoabfrage_result_codes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["code"] == code)
            .unwrap();
        assert_eq!(row["exit"], 6);
    }
}

#[test]
fn describe_rejects_unknown_paths_as_usage_errors() {
    let output = cli(&["describe", "postfach", "unknown"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["error"]["code"], "usage_error");
}

#[test]
fn skill_prints_markdown_without_config_or_eric() {
    let output = cli(&["skill"]);
    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    assert_eq!(output.stdout, include_bytes!("../SKILL.md"));
    let markdown = String::from_utf8(output.stdout).unwrap();
    assert!(markdown.starts_with("---\nname: kiecli\n"));
    assert!(markdown.contains("zertifikat status"));
    assert!(markdown.contains("24 hours"));
}
