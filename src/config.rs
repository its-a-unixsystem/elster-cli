// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{self, ErrorKind, Write};
use std::path::{Path, PathBuf, absolute};

use directories::BaseDirs;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use toml::{Table, Value};

use crate::{Cli, Format, error::Failure, write_json};

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
    hersteller_id: Option<Value>,
    default_profile: Option<String>,
    #[serde(default)]
    profile: std::collections::BTreeMap<String, Profile>,
}

#[derive(Deserialize, Default)]
struct Profile {
    test: Option<bool>,
    #[serde(flatten)]
    details: Table,
}

#[derive(Deserialize)]
struct ProfileDetails {
    #[serde(flatten)]
    credentials: CredentialConfig,
    steuernummer: Option<String>,
    bundesland: Option<String>,
}

pub(crate) struct TaxpayerNumber {
    pub(crate) steuernummer: String,
    pub(crate) bundesland: Option<String>,
    pub(crate) warnings: Vec<Warning>,
}



#[derive(Deserialize)]
struct CredentialConfig {
    certificate: Option<PathBuf>,
    pin_file: Option<PathBuf>,
    pin_command: Option<String>,
}

pub(crate) struct Settings {
    pub(crate) eric_dir: PathBuf,
    pub(crate) profile: Option<String>,
    pub(crate) test: bool,
    pub(crate) warnings: Vec<Warning>,
}

#[derive(Clone, Serialize, JsonSchema)]
pub(crate) struct Warning {
    pub(crate) code: &'static str,
    pub(crate) message: &'static str,
}

impl Warning {
    pub(crate) fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }

    pub(crate) fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        writeln!(out, "warning[{}]: {}", self.code, self.message)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Source {
    Flag,
    Env,
    File,
    Default,
}

struct Resolved<T> {
    value: T,
    source: Source,
}

fn select<T>(
    flag: Option<T>,
    env: impl FnOnce() -> Result<Option<T>, Failure>,
    file: Option<T>,
) -> Result<Resolved<Option<T>>, Failure> {
    let (value, source) = if flag.is_some() {
        (flag, Source::Flag)
    } else if let Some(value) = env()? {
        (Some(value), Source::Env)
    } else if file.is_some() {
        (file, Source::File)
    } else {
        (None, Source::Default)
    };
    Ok(Resolved { value, source })
}

struct Layers {
    base: Option<BaseDirs>,
    config: Resolved<Option<PathBuf>>,
    profile: Resolved<Option<String>>,
    selected: Profile,
    eric_dir: Resolved<PathBuf>,
    test: Resolved<bool>,
    hersteller_id: Option<Value>,
    warnings: Vec<Warning>,
}

impl Layers {
    fn load(cli: &Cli) -> Result<Self, Failure> {
        let base = BaseDirs::new();
        let home = base.as_ref().map(BaseDirs::home_dir);
        let mut config = select(cli.config.clone(), || Ok(env_path("KIECLI_CONFIG")), None)?;
        let required = config.source != Source::Default;
        if let Some(path) = config.value {
            config.value = Some(resolve_path(&path, home, None)?);
        } else {
            config.value = base
                .as_ref()
                .map(|base| base.config_dir().join("kiecli/config.toml"));
        }
        let mut file = match config.value.as_deref() {
            Some(path) => read_config(path, required)?,
            None => Config::default(),
        };
        let dir = select(
            cli.eric_dir.clone(),
            || Ok(env_path("KIECLI_ERIC_DIR")),
            file.eric_dir,
        )?;
        let eric_dir = Resolved {
            value: match dir.value {
                Some(path) => resolve_path(&path, home, relative_to(dir.source, &config))?,
                None => base
                    .as_ref()
                    .map(|base| base.data_dir().join(crate::eric::DEFAULT_DATA_DIR))
                    .ok_or_else(|| {
                        Failure::new(
                            3,
                            "eric_dir_unresolved",
                            "Could not determine the default ERiC directory",
                        )
                        .help("Set --eric-dir, KIECLI_ERIC_DIR or eric_dir in the config file.")
                    })?,
            },
            source: dir.source,
        };

        let mut profile = select(
            cli.profile.clone(),
            || read_env("KIECLI_PROFILE"),
            file.default_profile,
        )?;
        let selected = match profile.value.as_deref() {
            Some(name) => file.profile.remove(name).ok_or_else(|| {
                Failure::new(3, "profile_unknown", format!("Unknown profile '{name}'"))
            })?,
            None if file.profile.len() > 1 => {
                let names = file
                    .profile
                    .keys()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(", ");
                return Err(Failure::new(
                    3,
                    "profile_required",
                    format!("Select a profile: {names}"),
                )
                .help("Use --profile, KIECLI_PROFILE or default_profile in the config file."));
            }
            None => match file.profile.pop_first() {
                Some((name, selected)) => {
                    profile = Resolved {
                        value: Some(name),
                        source: Source::File,
                    };
                    selected
                }
                None => Profile::default(),
            },
        };
        let mut warnings = Vec::new();
        let test = profile_setting(
            (cli.test || cli.no_test).then_some(cli.test),
            read_test,
            selected.test,
            "KIECLI_TEST overrides the profile's test setting",
            &mut warnings,
        )?;
        Ok(Self {
            base,
            config,
            profile,
            selected,
            eric_dir,
            test: Resolved {
                value: test.value.unwrap_or_default(),
                source: test.source,
            },
            hersteller_id: file.hersteller_id,
            warnings,
        })
    }

    fn certificate(
        &mut self,
        cli: &Cli,
        profile: Option<PathBuf>,
    ) -> Result<Resolved<Option<PathBuf>>, Failure> {
        let mut certificate = profile_setting(
            cli.certificate.clone(),
            || Ok(env_path("KIECLI_CERTIFICATE")),
            profile,
            "KIECLI_CERTIFICATE overrides the profile's certificate setting",
            &mut self.warnings,
        )?;
        if let Some(path) = &certificate.value {
            certificate.value = Some(resolve_path(
                path,
                self.base.as_ref().map(BaseDirs::home_dir),
                relative_to(certificate.source, &self.config),
            )?);
        }
        Ok(certificate)
    }

    fn pin(
        &mut self,
        cli: &Cli,
        profile: CredentialConfig,
    ) -> Result<Resolved<Option<crate::pin::Source>>, Failure> {
        let pin_source = |file: Option<PathBuf>, command: Option<String>| {
            file.map(crate::pin::Source::File)
                .or_else(|| command.map(|value| crate::pin::Source::Command(value.into())))
        };
        profile_setting(
            pin_source(cli.pin_file.clone(), cli.pin_command.clone()),
            || Ok(crate::pin::Source::environment()),
            pin_source(profile.pin_file, profile.pin_command),
            "The PIN environment setting overrides the profile's pin_file or pin_command setting",
            &mut self.warnings,
        )
    }

    fn into_settings(self) -> Settings {
        Settings {
            eric_dir: self.eric_dir.value,
            profile: self.profile.value,
            test: self.test.value,
            warnings: self.warnings,
        }
    }

    fn hersteller_id(&mut self, cli: &Cli) -> Result<Resolved<Option<String>>, Failure> {
        let setting: Resolved<Option<String>> = select(
            cli.hersteller_id.clone(),
            || read_env("KIECLI_HERSTELLER_ID"),
            self.hersteller_id
                .take()
                .map(Value::try_into)
                .transpose()
                .map_err(|_| {
                    Failure::new(
                        3,
                        "config_invalid",
                        "Invalid HerstellerID setting type in the config file",
                    )
                })?,
        )?;
        if setting
            .value
            .as_ref()
            .is_some_and(|id| id.len() != 5 || !id.bytes().all(|byte| byte.is_ascii_digit()))
        {
            return Err(Failure::new(
                3,
                "hersteller_id_invalid",
                "HerstellerID must be five digits",
            )
            .help(HERSTELLER_ID_HELP));
        }
        Ok(setting)
    }
}

const HERSTELLER_ID_HELP: &str =
    "Set --hersteller-id, KIECLI_HERSTELLER_ID or hersteller_id in the config file.";

fn env_path(key: &str) -> Option<PathBuf> {
    env::var_os(key).map(PathBuf::from)
}

fn relative_to(source: Source, config: &Resolved<Option<PathBuf>>) -> Option<&Path> {
    if source == Source::File {
        config.value.as_deref().and_then(Path::parent)
    } else {
        None
    }
}

fn read_test() -> Result<Option<bool>, Failure> {
    match read_env("KIECLI_TEST")?.as_deref() {
        Some("0") => Ok(Some(false)),
        Some("1") => Ok(Some(true)),
        None => Ok(None),
        Some(_) => Err(Failure::new(
            3,
            "test_invalid",
            "KIECLI_TEST must be 0 or 1",
        )),
    }
}

fn warn_override(warnings: &mut Vec<Warning>, message: &'static str) {
    warnings.push(Warning {
        code: "env_overrides_profile",
        message,
    });
}

fn profile_setting<T>(
    flag: Option<T>,
    env: impl FnOnce() -> Result<Option<T>, Failure>,
    file: Option<T>,
    warning: &'static str,
    warnings: &mut Vec<Warning>,
) -> Result<Resolved<Option<T>>, Failure> {
    let in_profile = file.is_some();
    let setting = select(flag, env, file)?;
    if setting.source == Source::Env && in_profile {
        warn_override(warnings, warning);
    }
    Ok(setting)
}

pub(crate) fn resolve(cli: &Cli) -> Result<Settings, Failure> {
    Ok(Layers::load(cli)?.into_settings())
}

pub(crate) fn certificate_settings(
    cli: &Cli,
) -> Result<(Settings, PathBuf, crate::pin::Pin), Failure> {
    let mut layers = Layers::load(cli)?;
    let (path, pin) = credentials(cli, &mut layers)?;
    Ok((layers.into_settings(), path, pin))
}

pub(crate) fn request_settings(
    cli: &Cli,
) -> Result<(Settings, PathBuf, crate::pin::Pin, String), Failure> {
    let mut layers = Layers::load(cli)?;
    let id = layers.hersteller_id(cli)?.value.ok_or_else(|| {
        Failure::new(3, "hersteller_id_missing", "No HerstellerID was supplied")
            .note(HERSTELLER_ID_HELP)
    })?;
    let (path, pin) = credentials(cli, &mut layers)?;
    Ok((layers.into_settings(), path, pin, id))
}

pub(crate) fn taxpayer_number_settings(cli: &Cli) -> Result<TaxpayerNumber, Failure> {
    let mut layers = Layers::load(cli)?;
    let mut profile: ProfileDetails = std::mem::take(&mut layers.selected.details)
        .try_into()
        .map_err(|_| Failure::new(3, "config_invalid", "Invalid selected profile setting type"))?;
    let steuernummer = profile_setting(
        cli.steuernummer.clone(),
        || read_env("KIECLI_STEUERNUMMER"),
        profile.steuernummer.take(),
        "KIECLI_STEUERNUMMER overrides the profile's steuernummer setting",
        &mut layers.warnings,
    )?.value.ok_or_else(|| Failure::new(2, "steuernummer_missing", "No Steuernummer was supplied")
        .help("Set --steuernummer, KIECLI_STEUERNUMMER or steuernummer in the selected profile."))?;
    let bundesland = profile_setting(
        cli.bundesland.clone(),
        || read_env("KIECLI_BUNDESLAND"),
        profile.bundesland.take(),
        "KIECLI_BUNDESLAND overrides the profile's bundesland setting",
        &mut layers.warnings,
    )?.value;
    Ok(TaxpayerNumber { steuernummer, bundesland, warnings: layers.warnings })
}

fn credentials(cli: &Cli, layers: &mut Layers) -> Result<(PathBuf, crate::pin::Pin), Failure> {
    let mut credentials: CredentialConfig = std::mem::take(&mut layers.selected.details)
        .try_into()
        .map_err(|_| {
            Failure::new(
                3,
                "config_invalid",
                "Invalid Certificate or PIN setting type",
            )
        })?;
    let certificate = layers
        .certificate(cli, credentials.certificate.take())?
        .value
        .ok_or_else(|| {
            Failure::new(
                3,
                "certificate_not_found",
                "No Certificate path was supplied",
            )
            .note("Set --certificate, KIECLI_CERTIFICATE or certificate in the profile.")
        })?;
    if !certificate.is_file() || fs::File::open(&certificate).is_err() {
        return Err(Failure::new(
            3,
            "certificate_not_found",
            format!(
                "Certificate '{}' is not a readable file",
                certificate.display()
            ),
        ));
    }
    let pin = layers.pin(cli, credentials)?;
    let pin = match pin.value {
        Some(crate::pin::Source::File(path)) => crate::pin::Source::File(resolve_path(
            &path,
            layers.base.as_ref().map(BaseDirs::home_dir),
            relative_to(pin.source, &layers.config),
        )?),
        Some(source) => source,
        None => return Err(Failure::new(3, "pin_missing", "No Certificate PIN was supplied")
            .note("Set KIECLI_PIN, or use a PIN file (--pin-file, KIECLI_PIN_FILE, pin_file) or a PIN command (--pin-command, KIECLI_PIN_COMMAND, pin_command).")),
    };
    let pin = pin.read(&mut layers.warnings)?;
    Ok((certificate, pin))
}

pub(crate) fn show(cli: &Cli, mut out: impl Write) -> Result<(), Failure> {
    let mut layers = Layers::load(cli)?;
    let invalid = || {
        Failure::new(
            3,
            "config_invalid",
            "Invalid HerstellerID or selected profile setting type in the config file",
        )
    };
    let mut profile: ProfileDetails = std::mem::take(&mut layers.selected.details)
        .try_into()
        .map_err(|_| invalid())?;
    let certificate = layers.certificate(cli, profile.credentials.certificate.take())?;
    let pin = layers.pin(cli, profile.credentials)?;
    let steuernummer = profile_setting(
        cli.steuernummer.clone(),
        || read_env("KIECLI_STEUERNUMMER"),
        profile.steuernummer,
        "KIECLI_STEUERNUMMER overrides the profile's steuernummer setting",
        &mut layers.warnings,
    )?;
    let bundesland = profile_setting(
        cli.bundesland.clone(),
        || read_env("KIECLI_BUNDESLAND"),
        profile.bundesland,
        "KIECLI_BUNDESLAND overrides the profile's bundesland setting",
        &mut layers.warnings,
    )?;
    let hersteller_id = layers.hersteller_id(cli)?;
    let pin_channel = pin.value.as_ref().map(crate::pin::Source::channel);

    let file_source = layers
        .config
        .value
        .as_ref()
        .map(|path| format!("file:{}", path.display()));
    let source = |s| match s {
        Source::Flag => "flag",
        Source::Env => "env",
        Source::File => file_source
            .as_deref()
            .expect("file source requires a config path"),
        Source::Default => "default",
    };
    let report = Show {
        config: Shown::new(
            layers.config.value.as_deref().map(Path::to_string_lossy),
            source(layers.config.source),
        ),
        eric_dir: Shown::new(
            Some(layers.eric_dir.value.to_string_lossy()),
            source(layers.eric_dir.source),
        ),
        certificate: Shown::new(
            certificate.value.as_deref().map(Path::to_string_lossy),
            source(certificate.source),
        ),
        pin: Shown::new(pin_channel, source(pin.source)),
        hersteller_id: Shown::new(hersteller_id.value.as_deref(), source(hersteller_id.source)),
        steuernummer: Shown::new(steuernummer.value.as_deref(), source(steuernummer.source)),
        bundesland: Shown::new(bundesland.value.as_deref(), source(bundesland.source)),
        profile: layers.profile.value.as_deref(),
        profile_source: source(layers.profile.source),
        test: layers.test.value,
        test_source: source(layers.test.source),
        warnings: &layers.warnings,
        trace: cli.verbose.then_some(&[]),
    };
    match cli.format {
        Format::Json => write_json(out, &report),
        Format::Text => report.write_text(&mut out),
    }
    .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
}

#[derive(Serialize, JsonSchema)]
struct Shown<'a, T> {
    value: Option<T>,
    source: &'a str,
}

impl<'a, T> Shown<'a, T> {
    fn new(value: Option<T>, source: &'a str) -> Self {
        Self { value, source }
    }
}

impl<T: AsRef<str>> Shown<'_, T> {
    fn write_text(&self, mut out: impl Write, name: &str) -> io::Result<()> {
        let value = self.value.as_ref().map_or("none", AsRef::as_ref);
        writeln!(
            out,
            "{name}: {} ({})",
            value.escape_debug(),
            self.source.escape_debug()
        )
    }
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Show<'a> {
    config: Shown<'a, Cow<'a, str>>,
    eric_dir: Shown<'a, Cow<'a, str>>,
    certificate: Shown<'a, Cow<'a, str>>,
    pin: Shown<'a, &'a str>,
    hersteller_id: Shown<'a, &'a str>,
    steuernummer: Shown<'a, &'a str>,
    bundesland: Shown<'a, &'a str>,
    profile: Option<&'a str>,
    profile_source: &'a str,
    test: bool,
    test_source: &'a str,
    warnings: &'a [Warning],
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    trace: Option<&'static [crate::eric::TraceEvent]>,
}

impl Show<'_> {
    fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        Shown::new(self.profile, self.profile_source).write_text(&mut out, "profile")?;
        Shown::new(
            Some(if self.test { "true" } else { "false" }),
            self.test_source,
        )
        .write_text(&mut out, "test")?;
        self.config.write_text(&mut out, "config")?;
        self.eric_dir.write_text(&mut out, "eric_dir")?;
        self.certificate.write_text(&mut out, "certificate")?;
        self.pin.write_text(&mut out, "pin")?;
        self.hersteller_id.write_text(&mut out, "hersteller_id")?;
        self.steuernummer.write_text(&mut out, "steuernummer")?;
        self.bundesland.write_text(&mut out, "bundesland")?;
        for warning in self.warnings {
            warning.write_text(&mut out)?;
        }
        Ok(())
    }
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
