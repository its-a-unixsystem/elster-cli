// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use schemars::JsonSchema;
use serde::Serialize;

#[derive(JsonSchema, Serialize)]
pub(super) struct Setting {
    name: &'static str,
    sources: &'static [SettingSource],
    scope: Scope,
    default: Option<DefaultValue>,
    meaning: &'static str,
}

#[derive(JsonSchema, Serialize)]
struct SettingSource {
    flag: Option<&'static str>,
    env: Option<&'static str>,
    config_key: Option<&'static str>,
}

impl SettingSource {
    const fn new(flag: &'static str, env: &'static str, config_key: &'static str) -> Self {
        Self {
            flag: Some(flag),
            env: Some(env),
            config_key: Some(config_key),
        }
    }

    const fn flag(flag: &'static str) -> Self {
        Self {
            flag: Some(flag),
            env: None,
            config_key: None,
        }
    }
}

#[derive(JsonSchema, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Scope {
    Profile,
    TopLevel,
    Invocation,
}

#[derive(JsonSchema, Serialize)]
#[serde(untagged)]
enum DefaultValue {
    Bool(bool),
    Text(&'static str),
}

pub(super) const SETTINGS: &[Setting] = &[
    Setting {
        name: "certificate",
        sources: &[SettingSource::new(
            "--certificate",
            "KIECLI_CERTIFICATE",
            "certificate",
        )],
        scope: Scope::Profile,
        default: None,
        meaning: "Path to the Taxpayer's Certificate.",
    },
    Setting {
        name: "pin",
        sources: &[
            SettingSource {
                flag: None,
                env: Some("KIECLI_PIN"),
                config_key: None,
            },
            SettingSource::new("--pin-file", "KIECLI_PIN_FILE", "pin_file"),
            SettingSource::new("--pin-command", "KIECLI_PIN_COMMAND", "pin_command"),
        ],
        scope: Scope::Profile,
        default: None,
        meaning: "Certificate PIN. Precedence: flag > environment > profile. In a layer, the first available source listed here wins. A literal PIN is accepted only from KIECLI_PIN.",
    },
    Setting {
        name: "test",
        sources: &[
            SettingSource::new("--test", "KIECLI_TEST", "test"),
            SettingSource::flag("--no-test"),
        ],
        scope: Scope::Profile,
        default: Some(DefaultValue::Bool(false)),
        meaning: "Whether to send the Testmerker. KIECLI_TEST accepts 0 (live) or 1 (test).",
    },
    Setting {
        name: "steuernummer",
        sources: &[SettingSource::new(
            "--steuernummer",
            "KIECLI_STEUERNUMMER",
            "steuernummer",
        )],
        scope: Scope::Profile,
        default: None,
        meaning: "Taxpayer's Steuernummer in ELSTER or Bescheid format, for Kontoabfrage.",
    },
    Setting {
        name: "bundesland",
        sources: &[SettingSource::new(
            "--bundesland",
            "KIECLI_BUNDESLAND",
            "bundesland",
        )],
        scope: Scope::Profile,
        default: None,
        meaning: "Two-letter Bundesland code, required for a Bescheid-format Steuernummer.",
    },
    Setting {
        name: "hersteller_id",
        sources: &[SettingSource::new(
            "--hersteller-id",
            "KIECLI_HERSTELLER_ID",
            "hersteller_id",
        )],
        scope: Scope::TopLevel,
        default: None,
        meaning: "Five-digit HerstellerID.",
    },
    Setting {
        name: "eric_dir",
        sources: &[SettingSource::new(
            "--eric-dir",
            "KIECLI_ERIC_DIR",
            "eric_dir",
        )],
        scope: Scope::TopLevel,
        default: Some(DefaultValue::Text(
            "$XDG_DATA_HOME/kiecli/eric or ~/.local/share/kiecli/eric",
        )),
        meaning: "Directory where ERiC and Otto are installed.",
    },
    Setting {
        name: "profile",
        sources: &[SettingSource::new(
            "--profile",
            "KIECLI_PROFILE",
            "default_profile",
        )],
        scope: Scope::TopLevel,
        default: Some(DefaultValue::Text(
            "The only configured profile, if exactly one exists",
        )),
        meaning: "Selects the Taxpayer's profile. If several profiles exist, select one explicitly or set default_profile.",
    },
    Setting {
        name: "config",
        sources: &[SettingSource {
            flag: Some("--config"),
            env: Some("KIECLI_CONFIG"),
            config_key: None,
        }],
        scope: Scope::Invocation,
        default: Some(DefaultValue::Text(
            "$XDG_CONFIG_HOME/kiecli/config.toml or ~/.config/kiecli/config.toml",
        )),
        meaning: "Path to the TOML config file; an absent default file is allowed.",
    },
    Setting {
        name: "format",
        sources: &[SettingSource::flag("--format")],
        scope: Scope::Invocation,
        default: Some(DefaultValue::Text("json")),
        meaning: "Output format: json or text.",
    },
    Setting {
        name: "verbose",
        sources: &[SettingSource::flag("-v")],
        scope: Scope::Invocation,
        default: Some(DefaultValue::Bool(false)),
        meaning: "Include a trace of calls that support tracing.",
    },
    Setting {
        name: "eric_log",
        sources: &[SettingSource::flag("--eric-log")],
        scope: Scope::Invocation,
        default: Some(DefaultValue::Text("A temporary directory deleted at exit")),
        meaning: "Keep ERiC logs in the specified directory. They may contain tax data.",
    },
];

#[derive(JsonSchema, Serialize)]
pub(super) struct ExitCode {
    code: u8,
    meaning: &'static str,
}

impl ExitCode {
    const fn new(code: u8, meaning: &'static str) -> Self {
        Self { code, meaning }
    }
}

pub(super) const EXIT_CODES: &[ExitCode] = &[
    ExitCode::new(0, "Success."),
    ExitCode::new(1, "Internal error."),
    ExitCode::new(2, "Usage error."),
    ExitCode::new(3, "Configuration or credentials error."),
    ExitCode::new(
        4,
        "ERiC is missing, unsupported, not usable, or its terms have not been accepted.",
    ),
    ExitCode::new(5, "ELSTER rejected the request."),
    ExitCode::new(6, "Transient failure; a retry makes sense."),
    ExitCode::new(7, "Partial failure; see the results on stdout."),
];

#[derive(JsonSchema, Serialize)]
pub(super) struct Code {
    code: &'static str,
    meaning: &'static str,
}

impl Code {
    const fn new(code: &'static str, meaning: &'static str) -> Self {
        Self { code, meaning }
    }
}

pub(super) const ERROR_CODES: &[Code] = &[
    Code::new(
        "abholung_partial",
        "At least one Postfach-Nachricht was not collected completely or its requested Bestätigung failed.",
    ),
    Code::new(
        "bestaetigung_partial",
        "ELSTER did not confirm every requested Postfach-Nachricht.",
    ),
    Code::new(
        "bundesland_invalid",
        "The supplied Bundesland code is unknown.",
    ),
    Code::new(
        "bundesland_required",
        "A Bescheid-format Steuernummer requires a Bundesland.",
    ),
    Code::new(
        "certificate_expired",
        "The Certificate's validity period has ended.",
    ),
    Code::new(
        "certificate_inspection_failed",
        "ERiC failed to inspect the Certificate.",
    ),
    Code::new(
        "certificate_not_admitted",
        "ELSTER did not admit the Certificate for this procedure.",
    ),
    Code::new(
        "certificate_not_found",
        "The Certificate path is missing or is not a readable file.",
    ),
    Code::new(
        "certificate_not_yet_valid",
        "The Certificate's validity period has not started.",
    ),
    Code::new(
        "certificate_properties_invalid",
        "ERiC returned missing or invalid Certificate properties.",
    ),
    Code::new(
        "certificate_unreadable",
        "ERiC could not open the Certificate.",
    ),
    Code::new(
        "config_invalid",
        "The configuration file contains invalid TOML or an invalid setting value or type.",
    ),
    Code::new(
        "config_unknown_key",
        "The configuration file contains a key not supported in that scope.",
    ),
    Code::new(
        "config_unreadable",
        "The configuration file could not be read.",
    ),
    Code::new(
        "datenart_not_released",
        "ELSTER has not released the Bereitstellungsdatenart.",
    ),
    Code::new(
        "datenart_unknown",
        "The supplied Bereitstellungsdatenart name is unknown.",
    ),
    Code::new("elster_rejected", "ELSTER rejected the Postfach request."),
    Code::new(
        "environment_invalid",
        "An environment setting is not valid UTF-8.",
    ),
    Code::new(
        "eric_archive_invalid",
        "The ERiC archive could not be unpacked or its layout is invalid.",
    ),
    Code::new(
        "eric_buffer_failed",
        "ERiC could not create a return buffer.",
    ),
    Code::new(
        "eric_dir_unresolved",
        "The default ERiC directory could not be determined.",
    ),
    Code::new(
        "eric_download_failed",
        "Downloading the ERiC package failed.",
    ),
    Code::new(
        "eric_download_too_large",
        "The ERiC download exceeded the size limit.",
    ),
    Code::new("eric_end_failed", "ERiC could not be shut down cleanly."),
    Code::new("eric_init_failed", "ERiC initialization failed."),
    Code::new(
        "eric_install_failed",
        "The ERiC installation could not be prepared or activated.",
    ),
    Code::new(
        "eric_installation_invalid",
        "The installed ERiC directory lacks the expected layout.",
    ),
    Code::new(
        "eric_libgcc_missing",
        "The dynamic loader could not find libgcc_s.so.1.",
    ),
    Code::new(
        "eric_load_failed",
        "An ERiC or Otto library could not be located or loaded.",
    ),
    Code::new(
        "eric_log_unavailable",
        "The ERiC log directory could not be created.",
    ),
    Code::new(
        "eric_missing",
        "ERiC is not installed in the selected directory.",
    ),
    Code::new(
        "eric_network_failed",
        "ERiC could not communicate with ELSTER.",
    ),
    Code::new("eric_process_failed", "ERiC failed to process the request."),
    Code::new(
        "eric_redirect_refused",
        "The ERiC download redirect was unsafe, invalid, or exceeded the redirect limit.",
    ),
    Code::new(
        "eric_sha256_conflict",
        "The supplied hash conflicts with the pinned ERiC package hash.",
    ),
    Code::new(
        "eric_sha256_invalid",
        "The supplied SHA-256 hash is malformed.",
    ),
    Code::new(
        "eric_sha256_mismatch",
        "The downloaded ERiC package does not match the expected SHA-256 hash.",
    ),
    Code::new(
        "eric_sha256_required",
        "The unpinned ERiC package requires an expected SHA-256 hash.",
    ),
    Code::new(
        "eric_symbol_missing",
        "A required ERiC or Otto library symbol is missing.",
    ),
    Code::new(
        "eric_terms_not_accepted",
        "The ERiC terms have not been accepted for this installation.",
    ),
    Code::new("eric_version_failed", "The ERiC version call failed."),
    Code::new(
        "eric_version_invalid",
        "The supplied or reported ERiC version is malformed.",
    ),
    Code::new(
        "eric_version_unsupported",
        "The selected or installed ERiC major version is unsupported.",
    ),
    Code::new("hersteller_id_blocked", "ERiC blocked the HerstellerID."),
    Code::new(
        "hersteller_id_invalid",
        "The HerstellerID is invalid; it must contain exactly five digits.",
    ),
    Code::new("hersteller_id_missing", "No HerstellerID was supplied."),
    Code::new(
        "hersteller_id_rejected",
        "ELSTER did not admit the HerstellerID.",
    ),
    Code::new(
        "home_unresolved",
        "A home-relative path could not be expanded.",
    ),
    Code::new(
        "kontoabfrage_authorization_required",
        "ELSTER reports that the Kontoabfrage is not authorised for this account.",
    ),
    Code::new(
        "kontoabfrage_invalid",
        "The Kontoabfrage response is invalid or cannot be matched to the request.",
    ),
    Code::new(
        "kontoabfrage_partial",
        "ELSTER reported that some Kontoabfrage Teile failed.",
    ),
    Code::new("kontoabfrage_rejected", "ELSTER rejected the Kontoabfrage."),
    Code::new(
        "kontoabfrage_transient",
        "ELSTER's Kontoabfrage backend did not return a result; retrying may succeed.",
    ),
    Code::new(
        "otto_fetch_failed",
        "Otto could not fetch the object or return a valid buffer.",
    ),
    Code::new("otto_init_failed", "Otto could not create an instance."),
    Code::new(
        "otto_invalid_parameter",
        "A parameter supplied to Otto was invalid.",
    ),
    Code::new("otto_object_not_found", "OTTER did not find the object."),
    Code::new(
        "otto_transport_failed",
        "The OTTER transfer failed transiently.",
    ),
    Code::new("otto_unauthorized", "OTTER refused authorization."),
    Code::new(
        "output_failed",
        "The command could not serialize or write its output.",
    ),
    Code::new(
        "path_invalid",
        "A path contains an interior NUL and cannot be passed to ERiC or Otto.",
    ),
    Code::new(
        "path_unresolved",
        "A path could not be resolved to an absolute path.",
    ),
    Code::new(
        "pin_command_failed",
        "The PIN command did not run successfully or did not produce a PIN.",
    ),
    Code::new("pin_file_unreadable", "The PIN file could not be read."),
    Code::new(
        "pin_invalid",
        "The PIN is empty or contains an interior NUL.",
    ),
    Code::new("pin_locked", "The Certificate PIN is locked."),
    Code::new("pin_missing", "No Certificate PIN source was supplied."),
    Code::new("pin_wrong", "The Certificate PIN is incorrect."),
    Code::new(
        "postfach_anfrage_invalid",
        "ELSTER did not return a valid PostfachAnfrage response.",
    ),
    Code::new(
        "postfach_bestaetigung_invalid",
        "ELSTER did not return a valid PostfachBestaetigung response.",
    ),
    Code::new(
        "postfach_nachricht_no_objects",
        "The Postfach-Nachricht contains no objects to collect.",
    ),
    Code::new(
        "postfach_nachricht_not_found",
        "The PostfachAnfrage result does not contain a requested Nachricht ID.",
    ),
    Code::new(
        "postfach_status_invalid",
        "ELSTER did not return a valid PostfachStatus response.",
    ),
    Code::new(
        "postfach_too_many_entries",
        "ELSTER found more than 500,000 matching Postfach entries.",
    ),
    Code::new(
        "postfach_write_failed",
        "A collected Postfach file could not be written or stored safely.",
    ),
    Code::new(
        "profile_required",
        "Several profiles are configured, but none was selected.",
    ),
    Code::new("profile_unknown", "The selected profile does not exist."),
    Code::new("steuerart_unknown", "The supplied Steuerart is unknown."),
    Code::new(
        "steuernummer_conversion_failed",
        "ERiC did not return a usable ELSTER-format Steuernummer.",
    ),
    Code::new(
        "steuernummer_invalid",
        "The supplied Steuernummer or Bundesfinanzamtsnummer is invalid.",
    ),
    Code::new(
        "steuernummer_missing",
        "No Steuernummer was supplied for the Kontoabfrage.",
    ),
    Code::new(
        "test_certificate_requires_testmerker",
        "A test Certificate cannot be used for a live request; pass --test.",
    ),
    Code::new("test_invalid", "KIECLI_TEST must be 0 or 1."),
    Code::new(
        "usage_error",
        "The command, its arguments, or their combination is invalid.",
    ),
];

pub(super) const WARNING_CODES: &[Code] = &[
    Code::new(
        "bestaetigung_pending",
        "An Abholung left Nachrichten unconfirmed; confirm within 24 hours.",
    ),
    Code::new(
        "certificate_expired",
        "Certificate status reports an expired or not-yet-valid Certificate.",
    ),
    Code::new(
        "certificate_expires_soon",
        "The Certificate expires in less than 60 days.",
    ),
    Code::new(
        "env_overrides_profile",
        "An environment variable overrides a setting in the selected profile.",
    ),
    Code::new(
        "pin_file_permissions",
        "The PIN file is readable by group or others.",
    ),
];

#[derive(JsonSchema, Serialize)]
pub(super) struct KontoabfrageCode {
    code: i32,
    exit: u8,
    meaning: &'static str,
}

impl KontoabfrageCode {
    const fn new(code: i32, meaning: &'static str) -> Self {
        Self {
            code,
            exit: crate::kontoabfrage::result_exit(code),
            meaning,
        }
    }
}

// Meanings from Fehlerliste ElsterKontoabfrage 2.1.1.
pub(super) const KONTOABFRAGE_CODES: &[KontoabfrageCode] = &[
    KontoabfrageCode::new(0, "The Kontoabfrage succeeded (all Teile succeeded)."),
    KontoabfrageCode::new(
        230077012,
        "The required authorisation for the Kontoabfrage is missing.",
    ),
    KontoabfrageCode::new(950013001, "Some Teile of the Kontoabfrage failed."),
    KontoabfrageCode::new(950013002, "Every Teil of the Kontoabfrage failed."),
    KontoabfrageCode::new(950013003, "The tax account is inactive."),
    KontoabfrageCode::new(
        950017004,
        "Online access is unavailable during ongoing insolvency proceedings.",
    ),
    KontoabfrageCode::new(950017005, "A request parameter is not permitted."),
    KontoabfrageCode::new(950017006, "The requested Abfrageart is not permitted."),
    KontoabfrageCode::new(950017007, "A value date before 1962 is not permitted."),
    KontoabfrageCode::new(
        950017008,
        "The responsible Bundesland's query system returned no result.",
    ),
    KontoabfrageCode::new(
        950017009,
        "ELSTER could not prepare the request correctly; contact the Finanzamt.",
    ),
    KontoabfrageCode::new(950017010, "The Kontoabfrage request to BIENE failed."),
    KontoabfrageCode::new(
        950017011,
        "The account is not enabled for ELSTER Kontoabfrage.",
    ),
    KontoabfrageCode::new(950019011, "A target system could not be reached."),
    KontoabfrageCode::new(
        950019999,
        "An unexpected error occurred during the Kontoabfrage.",
    ),
];
