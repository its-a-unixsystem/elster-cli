// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

mod response;


use std::ffi::CString;
use std::fmt::Write as FmtWrite;
use std::io::Write;

use clap::{Args, Subcommand};
use schemars::JsonSchema;
use serde::Serialize;
use time::{Date, format_description::well_known::Iso8601};

use crate::{Cli, Format, certificate, config, eric, error::Failure, write_json};
use eric::{AuthenticatedEric, TraceEvent};
const TEST_CERTIFICATE_REQUIRES_TESTMERKER: &str = "test_certificate_requires_testmerker";
const LANDESNUMMERN: &[(&str, &str)] = &[
    ("BW", "28"), ("BY", "91"), ("BE", "11"), ("BB", "30"),
    ("HB", "24"), ("HH", "22"), ("HE", "26"), ("MV", "40"),
    ("NI", "23"), ("NW", "51"), ("RP", "27"), ("SL", "10"),
    ("SN", "32"), ("ST", "31"), ("SH", "21"), ("TH", "41"),
];

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Query open amounts on the tax account.
    OffeneBetraege,
    /// Query payments made to the tax account.
    Zahlungen(PaymentOptions),
    /// Query amounts assessed on the tax account.
    Sollstellungen(AssessmentOptions),
}

#[derive(Args)]
struct PaymentOptions {
    /// Include payments posted on or after this date (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    ab: Date,
    /// Limit the search to one year from --ab.
    #[arg(long)]
    ein_jahr: bool,
    /// Tax type (repeatable, case-insensitive; defaults to all).
    #[arg(long = "steuerart", value_name = "TYPE")]
    steuerarten: Vec<String>,
}

#[derive(Args)]
struct AssessmentOptions {
    /// Tax type (repeatable, case-insensitive).
    #[arg(long = "steuerart", required = true, value_name = "TYPE")]
    steuerarten: Vec<String>,
    /// Assessment year (repeatable).
    #[arg(long = "jahr", required = true, value_name = "YEAR")]
    jahre: Vec<u16>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Steuerart { Alle, Est, Kst, Ust, Lst, Gewst, Zast, Kapest }

pub(crate) const STEUERARTEN: &[Steuerart] = &[
    Steuerart::Alle,
    Steuerart::Est,
    Steuerart::Kst,
    Steuerart::Ust,
    Steuerart::Lst,
    Steuerart::Gewst,
    Steuerart::Zast,
    Steuerart::Kapest,
];

pub(crate) const DEFAULT_PAYMENT_STEUERART: Steuerart = Steuerart::Alle;

pub(crate) const fn result_exit(code: i32) -> u8 {
    match code {
        0 => 0,
        130025002 => 3,
        950017008 | 950019011 => 6,
        950013001 => 7,
        _ => 5,
    }
}

impl Steuerart {
    fn parse(raw: &str) -> Result<Self, Failure> {
        STEUERARTEN
            .iter()
            .copied()
            .find(|kind| raw.eq_ignore_ascii_case(kind.as_str()))
            .ok_or_else(|| {
                Failure::new(
                    2,
                    "steuerart_unknown",
                    format!("Unknown Steuerart `{}`", raw.escape_debug()),
                )
                .help("Use one of alle, ESt, KSt, USt, LSt, GewSt, ZaSt or KapESt.")
            })
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Alle => "alle", Self::Est => "ESt", Self::Kst => "KSt", Self::Ust => "USt",
            Self::Lst => "LSt", Self::Gewst => "GewSt", Self::Zast => "ZaSt", Self::Kapest => "KapESt",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Input {
    Open,
    Payment { steuerart: Steuerart, ab: Date, ein_jahr: bool },
    Assessment { steuerart: Steuerart, jahr: u16 },
}

impl Input {
    pub(super) fn art(self) -> &'static str {
        match self { Self::Open => "O", Self::Payment { .. } => "I", Self::Assessment { .. } => "ZS" }
    }
}

#[derive(Serialize, JsonSchema)]
pub(crate) struct Output {
    steuernummer: String,
    bundesfinanzamtsnummer: String,
    abfrageart: &'static str,
    #[serde(flatten)]
    teile: response::Reply,
    test: bool,
    profile: Option<String>,
    warnings: Vec<config::Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    trace: Option<Vec<TraceEvent>>,
}

impl Output {
    fn write_text(&self, mut out: impl Write) -> std::io::Result<()> {
        writeln!(out, "Kontoabfrage {} for {} (Finanzamt {})", self.abfrageart, self.steuernummer, self.bundesfinanzamtsnummer)?;
        self.teile.write_text(&mut out)?;
        writeln!(out, "Testmerker: {}; profile: {}", self.test, self.profile.as_deref().unwrap_or("none"))?;
        for warning in &self.warnings { warning.write_text(&mut out)?; }
        if let Some(trace) = &self.trace { for event in trace { writeln!(out, "{}: {} ms", event.call, event.duration_ms)?; } }
        Ok(())
    }
}

fn parse_date(raw: &str) -> Result<Date, &'static str> {
    let bytes = raw.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        Date::parse(raw, &Iso8601::DATE).map_err(|_| "expected a valid YYYY-MM-DD date")
    } else { Err("expected a valid YYYY-MM-DD date") }
}

fn validate_steuernummer_shape(raw: &str, bundesland: Option<&str>) -> Result<String, Failure> {
    let normalized: String = raw.chars().filter(|ch| !matches!(ch, ' ' | '/' | '-')).collect();
    if normalized.len() == 13 && normalized.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(normalized);
    }
    let Some(code) = bundesland else {
        return Err(Failure::new(2, "bundesland_required", "A Bundesland is required for a Bescheid-format Steuernummer")
            .help("Set --bundesland, KIECLI_BUNDESLAND or bundesland in the selected profile."));
    };
    if !LANDESNUMMERN.iter().any(|(state, _)| *state == code.to_ascii_uppercase()) {
        return Err(Failure::new(2, "bundesland_invalid", format!("Unknown Bundesland `{}`", code.escape_debug()))
            .help("Use a two-letter Bundesland code, for example BY or NW."));
    }
    Ok(normalized)
}

fn resolve_steuernummer(
    eric: &eric::Session,
    raw: &str,
    bundesland: Option<&str>,
    trace: &mut Option<Vec<TraceEvent>>,
) -> Result<(String, String), Failure> {
    let normalized: String = raw.chars().filter(|ch| !matches!(ch, ' ' | '/' | '-')).collect();
    if normalized.len() == 13 && normalized.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok((normalized.clone(), normalized[..4].to_owned()));
    }
    let code = bundesland.expect("validated Bescheid-format Bundesland").to_ascii_uppercase();
    let land = LANDESNUMMERN.iter().find_map(|(state, number)| (*state == code).then_some(*number)).expect("state code validated before credentials");
    let bufa = if matches!(code.as_str(), "BY" | "BE") && normalized.len() == 11 && raw.contains('/') { Some(&raw[..3]) } else { None };
    eric.convert_steuernummer(raw, land, bufa, trace)
}

fn request(inputs: &[Input], steuernummer: &str, bufa: &str, hersteller_id: &str, test: bool) -> CString {
    let mut xml = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">
<TransferHeader version="11"><Verfahren>ElsterKontoabfrage</Verfahren><DatenArt>Kontoabfrage</DatenArt><Vorgang>send-Auth</Vorgang>
"#);
    if test { xml.push_str("<Testmerker>230000001</Testmerker>\n"); }
    xml.push_str(&format!(r#"<HerstellerID>{hersteller_id}</HerstellerID><DatenLieferant>KIECLI</DatenLieferant>
<Datei><Verschluesselung>CMSEncryptedData</Verschluesselung><Kompression>GZIP</Kompression><TransportSchluessel/></Datei>
</TransferHeader><DatenTeil><Nutzdatenblock><NutzdatenHeader version="11"><NutzdatenTicket>1</NutzdatenTicket><Empfaenger id="F">{bufa}</Empfaenger><Hersteller><ProduktName>KIECLI</ProduktName><ProduktVersion>{}</ProduktVersion></Hersteller></NutzdatenHeader>
<Nutzdaten><kontoabfrage xmlns="http://finkonsens.de/elster/elsterkontoabfrage/kontoabfrage/v6" xmlns:o="http://www.elster.de/KontoAbfrage/O-Abfrage/06" xmlns:i="http://www.elster.de/KontoAbfrage/I-Abfrage/06" xmlns:zs="http://www.elster.de/KontoAbfrage/ZS-Abfrage/06" version="6">
"#, env!("CARGO_PKG_VERSION")));
    for input in inputs {
        let (prefix, art) = match input { Input::Open => ("o", "O"), Input::Payment { .. } => ("i", "I"), Input::Assessment { .. } => ("zs", "ZS") };
        write!(xml, "<kontoabfrage-teil><{prefix}:kontoabfrage-input><{prefix}:kontoabfrage-art>{art}</{prefix}:kontoabfrage-art><{prefix}:steuernummer>{steuernummer}</{prefix}:steuernummer>").unwrap();
        match input {
            Input::Open => {}
            Input::Payment { steuerart, ab, ein_jahr } => {
                let (year, month, day) = ab.to_calendar_date();
                let option = if *ein_jahr { "J" } else { "V" };
                write!(xml, "<i:steuerart>{}</i:steuerart><i:wertstellungsdatum option=\"{option}\">{day:02}{:02}{year:04}</i:wertstellungsdatum>", steuerart.as_str(), u8::from(month)).unwrap();
            }
            Input::Assessment { steuerart, jahr } => write!(xml, "<zs:steuerart>{}</zs:steuerart><zs:zeitraum>{jahr}</zs:zeitraum>", steuerart.as_str()).unwrap(),
        }
        writeln!(xml, "</{prefix}:kontoabfrage-input></kontoabfrage-teil>").unwrap();
    }
    xml.push_str("</kontoabfrage></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>\n");
    CString::new(xml).expect("NUL-free Kontoabfrage request")
}

fn inputs(command: &Command) -> Result<Vec<Input>, Failure> {
    let mut inputs = Vec::new();
    match command {
        Command::OffeneBetraege => inputs.push(Input::Open),
        Command::Zahlungen(options) => {
            let kinds = if options.steuerarten.is_empty() {
                vec![DEFAULT_PAYMENT_STEUERART]
            } else {
                parse_steuerarten(&options.steuerarten, true)?
            };
            inputs.reserve(kinds.len());
            inputs.extend(kinds.into_iter().map(|steuerart| Input::Payment { steuerart, ab: options.ab, ein_jahr: options.ein_jahr }));
        }
        Command::Sollstellungen(options) => {
            let kinds = parse_steuerarten(&options.steuerarten, false)?;
            let total = kinds.len().checked_mul(options.jahre.len()).filter(|count| *count <= 75).ok_or_else(|| Failure::new(2, "usage_error", "A Kontoabfrage may contain at most 75 Teile"))?;
            inputs.reserve(total);
            for steuerart in kinds { for jahr in &options.jahre { inputs.push(Input::Assessment { steuerart, jahr: *jahr }); } }
        }
    }
    Ok(inputs)
}

fn parse_steuerarten(values: &[String], allow_alle: bool) -> Result<Vec<Steuerart>, Failure> {
    let mut kinds = Vec::with_capacity(values.len());
    for value in values {
        let kind = Steuerart::parse(value)?;
        if kind == Steuerart::Alle && !allow_alle { return Err(Failure::new(2, "usage_error", "`alle` is not valid for sollstellungen")); }
        if !kinds.contains(&kind) { kinds.push(kind); }
    }
    Ok(kinds)
}

pub(crate) fn run(cli: &Cli, command: &Command, mut out: impl Write) -> Result<(), Failure> {
    let requested = inputs(command)?;
    let taxpayer = config::taxpayer_number_settings(cli)?;
    let _ = validate_steuernummer_shape(&taxpayer.steuernummer, taxpayer.bundesland.as_deref())?;
    let (mut settings, path, pin, hersteller_id) = config::request_settings(cli)?;
    settings.warnings.extend(taxpayer.warnings.clone());
    let installed = eric::installed_dir(&settings.eric_dir)?;
    let dir = installed.as_deref().unwrap_or(&settings.eric_dir);
    let mut probe = eric::Probe::new(dir, cli.verbose);
    let (normalized, bufa, mut result) = probe.with_session(dir, cli.eric_log.as_deref(), |session, trace| {
        let (normalized, bufa) = resolve_steuernummer(session, &taxpayer.steuernummer, taxpayer.bundesland.as_deref(), trace)?;
        session.with_certificate(&path, trace, |cert, trace| {
            let authenticated = (cert, &pin);
            let properties = authenticated.inspect(trace, |xml| Ok(xml.to_owned()))?;
            if certificate::check_for_request(&properties, time::OffsetDateTime::now_utc(), &mut settings.warnings)? && !settings.test { return Err(test_certificate_error()); }
            let xml = request(&requested, &normalized, &bufa, &hersteller_id, settings.test);
            authenticated.send(&xml, c"ElsterKontoabfrage_6", trace, |code, reply, text| response::read(code, &reply, text, &normalized, &requested, cli.mit_xml))
                .map(|payload| (normalized, bufa, payload))
        })
    })?;
    drop(pin);
    let outcome = std::mem::replace(&mut result.outcome, Ok(()));
    let report = Output { steuernummer: normalized, bundesfinanzamtsnummer: bufa, abfrageart: requested[0].art(), teile: result, test: settings.test, profile: settings.profile, warnings: settings.warnings, trace: probe.trace };
    match cli.format { Format::Json => write_json(&mut out, &report), Format::Text => report.write_text(&mut out) }
        .map_err(|err| Failure::new(1, "output_failed", err.to_string()))?;
    outcome
}

pub(super) fn test_certificate_error() -> Failure {
    Failure::new(3, TEST_CERTIFICATE_REQUIRES_TESTMERKER, "A test Certificate requires --test")
        .help("Add --test to your original command (replace --no-test if present).")
}
