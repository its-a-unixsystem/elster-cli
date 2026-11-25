// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf, absolute};

use directories::BaseDirs;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::{Cli, error::Failure};

const ROOT_KEYS: &[&str] = &["eric_dir", "hersteller_id", "default_profile", "profile"];
const PROFILE_KEYS: &[&str] = &[
    "certificate",
    "pin_file",
    "pin_command",
    "test",
    "steuernummer",
    "bundesland",
];

#[derive(Deserialize, Default)]
struct Config {
    eric_dir: Option<PathBuf>,
    default_profile: Option<String>,
    #[serde(default)]
    profile: std::collections::BTreeMap<String, Profile>,
}

#[derive(Deserialize)]
struct Profile {
    test: Option<bool>,
}

pub(crate) struct Settings {
    pub(crate) eric_dir: PathBuf,
    pub(crate) profile: Option<String>,
    pub(crate) test: bool,
    pub(crate) warnings: Vec<Warning>,
}

#[derive(Serialize)]
pub(crate) struct Warning {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

pub(crate) fn resolve(cli: &Cli) -> Result<Settings, Failure> {
    let base = BaseDirs::new();
    let home = base.as_ref().map(BaseDirs::home_dir);
    let explicit_config = cli
        .config
        .clone()
        .or_else(|| env::var_os("KIECLI_CONFIG").map(PathBuf::from));
    let file = if let Some(path) = explicit_config.as_ref() {
        Some(resolve_path(path, home, None)?)
    } else {
        base.as_ref()
            .map(|base| base.config_dir().join("kiecli/config.toml"))
    };
    let config = match file.as_deref() {
        Some(path) => read_config(path, explicit_config.is_some())?,
        None => Config::default(),
    };

    let eric_dir = if let Some(path) = cli
        .eric_dir
        .clone()
        .or_else(|| env::var_os("KIECLI_ERIC_DIR").map(PathBuf::from))
    {
        resolve_path(&path, home, None)?
    } else if let Some(path) = config.eric_dir.as_deref() {
        resolve_path(path, home, file.as_deref().and_then(Path::parent))?
    } else {
        base.as_ref()
            .map(|base| base.data_dir().join(crate::eric::DEFAULT_DATA_DIR))
            .ok_or_else(|| {
                Failure::new(
                    3,
                    "eric_dir_unresolved",
                    "Could not determine the default ERiC directory",
                )
                .help("Set --eric-dir, KIECLI_ERIC_DIR or eric_dir in the config file.")
            })?
    };

    let requested_profile = match &cli.profile {
        Some(name) => Some(name.clone()),
        None => read_env("KIECLI_PROFILE")?.or(config.default_profile),
    };
    let profile = match requested_profile {
        Some(name) if !config.profile.contains_key(&name) => {
            return Err(Failure::new(
                3,
                "profile_unknown",
                format!("Unknown profile '{name}'"),
            ));
        }
        selected @ Some(_) => selected,
        None if config.profile.len() > 1 => {
            let names = config
                .profile
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ");
            return Err(
                Failure::new(3, "profile_required", format!("Select a profile: {names}"))
                    .help("Use --profile, KIECLI_PROFILE or default_profile in the config file."),
            );
        }
        None => config
            .profile
            .first_key_value()
            .map(|(name, _)| name.clone()),
    };
    let profile_test = profile.as_ref().and_then(|name| config.profile[name].test);
    let mut warnings = Vec::new();
    let test = if cli.test || cli.no_test {
        cli.test
    } else {
        match read_env("KIECLI_TEST")?.as_deref() {
            Some(value @ ("0" | "1")) => {
                if profile_test.is_some() {
                    warnings.push(Warning {
                        code: "env_overrides_profile",
                        message: "KIECLI_TEST overrides the profile's test setting",
                    });
                }
                value == "1"
            }
            Some(_) => {
                return Err(Failure::new(
                    3,
                    "test_invalid",
                    "KIECLI_TEST must be 0 or 1",
                ));
            }
            None => profile_test.unwrap_or_default(),
        }
    };
    Ok(Settings {
        eric_dir,
        profile,
        test,
        warnings,
    })
}

fn read_env(key: &'static str) -> Result<Option<String>, Failure> {
    env::var_os(key)
        .map(|value| {
            value.into_string().map_err(|_| {
                Failure::new(
                    3,
                    "environment_invalid",
                    format!("{key} must be valid UTF-8"),
                )
            })
        })
        .transpose()
}

fn resolve_path(
    path: &Path,
    home: Option<&Path>,
    relative_to: Option<&Path>,
) -> Result<PathBuf, Failure> {
    let path = if let Ok(rest) = path.strip_prefix("~") {
        home.ok_or_else(|| {
            Failure::new(
                3,
                "home_unresolved",
                "Cannot expand '~' without a home directory",
            )
        })?
        .join(rest)
    } else if let Some(dir) = relative_to.filter(|_| path.is_relative()) {
        dir.join(path)
    } else {
        return absolute(path).map_err(|err| {
            Failure::new(
                3,
                "path_unresolved",
                format!("Cannot resolve '{}': {err}", path.display()),
            )
        });
    };
    absolute(&path).map_err(|err| {
        Failure::new(
            3,
            "path_unresolved",
            format!("Cannot resolve '{}': {err}", path.display()),
        )
    })
}

fn read_config(path: &Path, required: bool) -> Result<Config, Failure> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if !required && err.kind() == ErrorKind::NotFound => return Ok(Config::default()),
        Err(err) => {
            return Err(Failure::new(
                3,
                "config_unreadable",
                format!("Cannot read config '{}': {err}", path.display()),
            ));
        }
    };
    // TOML diagnostics can quote the input; never print it (it can hold credentials).
    let invalid = || {
        Failure::new(
            3,
            "config_invalid",
            format!("Invalid TOML or setting type in '{}'", path.display()),
        )
    };
    let table: Table = text.parse().map_err(|_| invalid())?;
    check_keys(&table, ROOT_KEYS, path)?;
    if let Some(profiles) = table.get("profile").and_then(Value::as_table) {
        for profile in profiles.values().filter_map(Value::as_table) {
            check_keys(profile, PROFILE_KEYS, path)?;
        }
    }
    table.try_into().map_err(|_| invalid())
}

fn check_keys(table: &Table, known: &[&str], path: &Path) -> Result<(), Failure> {
    if let Some(key) = table.keys().find(|key| !known.contains(&key.as_str())) {
        let mut failure = Failure::new(
            3,
            "config_unknown_key",
            format!("Unknown key '{key}' in '{}'", path.display()),
        );
        if let Some(closest) = known
            .iter()
            .min_by_key(|candidate| strsim::levenshtein(key, candidate))
        {
            failure = failure.help(format!("Did you mean '{closest}'?"));
        }
        return Err(failure);
    }
    Ok(())
}
