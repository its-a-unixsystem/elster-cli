// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

const PACKAGE: &str = "KIECLI_TEST_ERIC_HOME";

fn cli(home: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_kiecli"));
    command
        .env_remove("KIECLI_CONFIG")
        .env_remove("KIECLI_ERIC_DIR")
        .env_remove("KIECLI_PROFILE")
        .env_remove("KIECLI_TEST")
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_DATA_HOME", home.join("data"))
        .current_dir(home);
    command
}

fn error(output: &Output, exit: i32, code: &str) -> Value {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["error"]["code"], code, "{error}");
    error
}

#[test]
fn rejects_an_unknown_flag_as_a_usage_error() {
    let home = TempDir::new().unwrap();
    let result = cli(home.path())
        .args(["eric", "status", "--not-a-flag"])
        .output()
        .unwrap();
    error(&result, 2, "usage_error");
    assert!(result.stdout.is_empty(), "{result:?}");
}

#[test]
fn install_rejects_missing_consent_and_untrusted_versions_before_downloading() {
    let home = TempDir::new().unwrap();
    let wrong_digest = "0".repeat(64);
    let attempts: &[(&[&str], i32, &str)] = &[
        (&[], 4, "eric_terms_not_accepted"),
        (
            &["--accept-eric-terms", "--version", "45.2.0.0"],
            4,
            "eric_version_unsupported",
        ),
        (
            &["--accept-eric-terms", "--version", "44.3.99.0"],
            4,
            "eric_sha256_required",
        ),
        (
            &["--accept-eric-terms", "--sha256", &wrong_digest],
            4,
            "eric_sha256_conflict",
        ),
    ];
    for (args, exit, code) in attempts {
        let result = cli(home.path())
            .args(["eric", "install"])
            .args(*args)
            .output()
            .unwrap();
        error(&result, *exit, code);
        assert!(result.stdout.is_empty(), "{result:?}");
    }
    assert!(!home.path().join("data").exists());
}

#[test]
fn status_checks_the_active_managed_version_not_the_installation_root() {
    let home = TempDir::new().unwrap();
    let root = home.path().join("data/kiecli/eric");
    let active = root.join("versions/44.3.6.0");
    let lib = active.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("libericapi.so"), []).unwrap();
    std::fs::write(active.join(".kiecli-eric-terms-accepted"), []).unwrap();
    std::os::unix::fs::symlink("versions/44.3.6.0", root.join(".kiecli-eric-current")).unwrap();

    let result = cli(home.path()).args(["eric", "status"]).output().unwrap();
    error(&result, 4, "eric_load_failed");
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["eric_dir"], active.display().to_string());
    assert_eq!(status["terms_accepted"], true);
}

#[test]
fn missing_installation_points_to_the_installer() {
    let home = TempDir::new().unwrap();
    let result = cli(home.path()).args(["eric", "status"]).output().unwrap();
    let error = error(&result, 4, "eric_missing");
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["test_load"], false);
    assert_eq!(status["version"], Value::Null);
    assert_eq!(status["supported"], Value::Null);
    assert!(
        error["error"]["help"]
            .as_str()
            .unwrap()
            .contains("eric install")
    );
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("data/kiecli/eric")
    );
}

#[test]
fn refuses_to_load_without_terms_acceptance() {
    let home = TempDir::new().unwrap();
    let lib = home.path().join("lib");
    std::fs::create_dir(&lib).unwrap();
    // Not a shared library: a load attempt would report eric_load_failed instead.
    std::fs::write(lib.join("libericapi.so"), []).unwrap();
    std::fs::write(home.path().join("current"), "unrelated file").unwrap();
    let result = cli(home.path())
        .args(["eric", "status", "--eric-dir", "."])
        .output()
        .unwrap();
    error(&result, 4, "eric_terms_not_accepted");
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["terms_accepted"], false);
}

#[test]
fn config_paths_are_relative_to_the_file_and_flags_win() {
    let home = TempDir::new().unwrap();
    let config_dir = home.path().join("settings");
    std::fs::create_dir(&config_dir).unwrap();
    std::fs::write(config_dir.join("custom.toml"), "eric_dir = 'from-file'\n").unwrap();

    let selected_path = |flag: Option<&str>, env: Option<&str>| {
        let mut command = cli(home.path());
        command.args(["--config", "settings/custom.toml", "eric", "status"]);
        if let Some(path) = flag {
            command.args(["--eric-dir", path]);
        }
        if let Some(path) = env {
            command.env("KIECLI_ERIC_DIR", path);
        }
        let result = command.output().unwrap();
        error(&result, 4, "eric_missing")["error"]["message"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert!(selected_path(None, None).contains("settings/from-file"));
    assert!(selected_path(None, Some("from-env")).contains("/from-env"));
    assert!(selected_path(Some("from-flag"), Some("from-env")).contains("/from-flag"));
}

#[test]
fn reports_loader_failures_for_non_utf8_paths_as_valid_json() {
    let home = TempDir::new().unwrap();
    let package = home
        .path()
        .join(std::ffi::OsStr::from_bytes(b"package-\xff"));
    let lib = package.join("lib");
    std::fs::create_dir_all(&lib).unwrap();
    std::fs::write(lib.join("libericapi.so"), []).unwrap();
    std::fs::write(package.join(".kiecli-eric-terms-accepted"), []).unwrap();
    let result = cli(home.path())
        .args(["eric", "status", "--eric-dir"])
        .arg(&package)
        .output()
        .unwrap();
    assert!(
        error(&result, 4, "eric_load_failed")["error"]["message"]
            .as_str()
            .unwrap()
            .contains("libericapi.so")
    );
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["eric_dir"], package.display().to_string());
    assert_eq!(status["terms_accepted"], true);
    assert_eq!(status["test_load"], false);
}

// The CI eric-local job supplies an unpacked official ERiC 44.3.6.0 package.
#[test]
#[ignore = "requires an unpacked official ERiC 44.3.6.0 package"]
fn reports_the_loaded_library_version_offline() {
    let package = std::fs::canonicalize(std::env::var_os(PACKAGE).expect(PACKAGE)).unwrap();
    let home = TempDir::new().unwrap();
    std::os::unix::fs::symlink(package.join("lib"), home.path().join("lib")).unwrap();
    std::fs::write(home.path().join(".kiecli-eric-terms-accepted"), []).unwrap();
    std::fs::write(
        home.path().join("settings.toml"),
        "eric_dir = '.'\n[profile.offline]\ntest = true\n",
    )
    .unwrap();
    let tmp = home.path().join("tmp");
    std::fs::create_dir(&tmp).unwrap();
    let result = cli(home.path())
        .args(["--config", "settings.toml", "-v", "eric", "status"])
        .env("TMPDIR", &tmp)
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(result.stderr.is_empty(), "{result:?}");
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["version"], "44.3.4.0");
    assert_eq!(status["supported"], true);
    assert_eq!(status["supported_majors"], serde_json::json!([44]));
    assert_eq!(status["terms_accepted"], true);
    assert_eq!(status["test_load"], true);
    let home_path = home.path().to_str().unwrap();
    assert_eq!(status["eric_dir"], home_path);
    assert_eq!(
        status["library_paths"]["eric"],
        format!("{home_path}/lib/libericapi.so")
    );
    assert_eq!(
        status["library_paths"]["otto"],
        format!("{home_path}/lib/libotto.so")
    );
    assert_eq!(status["profile"], "offline");
    assert_eq!(status["test"], true);
    assert_eq!(status["warnings"], serde_json::json!([]));
    assert!(
        status["trace"]
            .as_array()
            .unwrap()
            .iter()
            .any(|item| item["call"] == "EricVersion")
    );
    assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);

    let result = cli(home.path())
        .args([
            "eric",
            "status",
            "--eric-dir",
            ".",
            "--format",
            "text",
            "--eric-log",
            "logs",
        ])
        .output()
        .unwrap();
    assert!(result.status.success(), "{result:?}");
    assert!(
        String::from_utf8(result.stdout)
            .unwrap()
            .contains("ERiC 44.3.4.0")
    );
    assert!(home.path().join("logs/eric.log").is_file());

    std::fs::remove_file(home.path().join("lib")).unwrap();
    std::fs::create_dir(home.path().join("lib")).unwrap();
    for name in [
        "libericapi.so",
        "libericxerces.so",
        "libeSigner.so",
        "plugins",
    ] {
        std::os::unix::fs::symlink(
            package.join("lib").join(name),
            home.path().join("lib").join(name),
        )
        .unwrap();
    }
    let result = cli(home.path())
        .args(["eric", "status", "--eric-dir", "."])
        .output()
        .unwrap();
    assert!(
        error(&result, 4, "eric_missing")["error"]["message"]
            .as_str()
            .unwrap()
            .contains("libotto.so")
    );
    let status: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(status["version"], "44.3.4.0");
    assert_eq!(status["test_load"], false);

    std::os::unix::fs::symlink("libericapi.so", home.path().join("lib/libotto.so")).unwrap();
    let result = cli(home.path())
        .args(["eric", "status", "--eric-dir", "."])
        .output()
        .unwrap();
    error(&result, 4, "eric_symbol_missing");
}
