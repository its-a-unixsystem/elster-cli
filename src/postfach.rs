// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

#[cfg(test)]
mod tests;

use std::ffi::CString;
use std::fmt::Write as _;
use std::io::Write;

use clap::Args;
use serde::{Deserialize, Serialize, de::IgnoredAny};
use time::{Date, OffsetDateTime, format_description::well_known::Iso8601};

use crate::{Cli, Format, certificate, config, eric, error::Failure, write_json};
use eric::{AuthenticatedEric, TraceEvent};

const TEST_CERTIFICATE_REQUIRES_TESTMERKER: &str = "test_certificate_requires_testmerker";

#[derive(clap::Subcommand)]
pub(crate) enum Command {
    /// Show ELSTER's Postfach counts as a hint; ELSTER may omit some kinds.
    Status(Options),
}

#[derive(Args)]
pub(crate) struct Options {
    /// Show only this Bereitstellungsdatenart (repeatable, case-sensitive).
    #[arg(long, value_name = "NAME")]
    datenart: Vec<String>,
    /// Count only Nachrichten not yet confirmed (default).
    #[arg(long, conflicts_with = "alle")]
    neue: bool,
    /// Include already confirmed Nachrichten.
    #[arg(long)]
    alle: bool,
    /// Include Nachrichten from this date (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    von: Option<Date>,
    /// Include Nachrichten through this date (YYYY-MM-DD).
    #[arg(long, value_name = "DATE", value_parser = parse_date)]
    bis: Option<Date>,
}

// Accepted through ERiC for Datenabholung version 32; see the test-system spike.
const DATENARTEN: &[&str] = &[
    "ANAuskunft",
    "AenderungslisteDUe",
    "AnhangRueckmeldung",
    "DIVABescheid",
    "DIVABuchmachersteuer",
    "DIVALotteriesteuerEinmal",
    "DIVALotteriesteuerMehr",
    "DIVAOnlinePokerStB",
    "DIVASportwettensteuer",
    "DIVATotalisator",
    "DIVAVirtuelleAutomatenStB",
    "DivaAdvMitteilung",
    "DivaAdvVA",
    "DivaAdvZinsVA",
    "DivaBescheidESt",
    "DivaBescheidErbSt",
    "DivaBescheidFEIN",
    "DivaBescheidGewSt",
    "DivaBescheidKSt",
    "DivaBescheidSchenkSt",
    "DivaBescheidUSt",
    "DivaEinwilligungRM",
    "DivaErlassMitteilung",
    "DivaErlassVA",
    "DivaSonstigeMitteilung",
    "DivaSonstigerVA",
    "DivaStundungVA",
    "EBilanzRueckmeldung",
    "ELOProtokoll",
    "EPAntwort",
    "EPAntwortKopie",
    "EPBescheid",
    "EPKurzmitteilung",
    "EPMitteilung",
    "ESB",
    "GEWAbmeldung",
    "GEWMB",
    "GSB",
    "Gewerbesteuerbescheid",
    "KDBVollmachtRM",
    "KMVProtokoll",
    "LStHVVMRM",
    "OVMVRM",
    "OZGBescheid",
    "OZGMitteilung",
    "USB",
    "VDBKDBRueckmeldungLief",
    "VerbindlicheAuskunft",
    "VerbindlicheZusage",
];

fn parse_date(raw: &str) -> Result<Date, &'static str> {
    let bytes = raw.as_bytes();
    if bytes.len() == 10 && bytes[4] == b'-' && bytes[7] == b'-' {
        Date::parse(raw, &Iso8601::DATE).map_err(|_| "expected a valid YYYY-MM-DD date")
    } else {
        Err("expected a valid YYYY-MM-DD date")
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum Einschraenkung {
    Neue,
    Alle,
}

impl Einschraenkung {
    fn as_str(self) -> &'static str {
        match self {
            Self::Neue => "neue",
            Self::Alle => "alle",
        }
    }
}

struct Scope {
    einschraenkung: Einschraenkung,
    von: Option<Date>,
    bis: Option<Date>,
    datenarten: Vec<&'static str>,
}

impl Options {
    fn correction(
        &self,
        cli: &Cli,
        datenart: Option<(usize, &str)>,
        enable_test: bool,
    ) -> Option<String> {
        // PIN commands can contain secrets; do not reconstruct the invocation in that case.
        if cli.pin_command.is_some() {
            return None;
        }
        let mut command = String::from("kiecli postfach status");
        if self.alle {
            command.push_str(" --alle");
        }
        if self.neue {
            command.push_str(" --neue");
        }
        if enable_test || cli.test {
            command.push_str(" --test");
        } else if cli.no_test {
            command.push_str(" --no-test");
        }
        if cli.verbose {
            command.push_str(" -v");
        }
        if cli.format == Format::Text {
            command.push_str(" --format text");
        }
        for (flag, date) in [("--von", self.von), ("--bis", self.bis)] {
            if let Some(date) = date {
                write!(command, " {flag} {date}").unwrap();
            }
        }
        for (index, name) in self.datenart.iter().enumerate() {
            let value = datenart
                .filter(|(target, _)| *target == index)
                .map_or(name.as_str(), |(_, replacement)| replacement);
            append_arg(&mut command, "--datenart", value)?;
        }
        for (flag, value) in [
            ("--config", cli.config.as_deref()),
            ("--eric-dir", cli.eric_dir.as_deref()),
            ("--eric-log", cli.eric_log.as_deref()),
            ("--certificate", cli.certificate.as_deref()),
            ("--pin-file", cli.pin_file.as_deref()),
        ] {
            if let Some(value) = value {
                append_arg(&mut command, flag, value.to_str()?)?;
            }
        }
        for (flag, value) in [
            ("--profile", cli.profile.as_deref()),
            ("--hersteller-id", cli.hersteller_id.as_deref()),
        ] {
            if let Some(value) = value {
                append_arg(&mut command, flag, value)?;
            }
        }
        Some(command)
    }
}

fn append_arg(command: &mut String, flag: &str, value: &str) -> Option<()> {
    if value.chars().any(char::is_control) {
        return None;
    }
    write!(command, " {flag}=").unwrap();
    if !value.is_empty()
        && value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || b"_./-".contains(&ch))
    {
        command.push_str(value);
    } else {
        write!(command, "'{}'", value.replace('\'', "'\\''")).unwrap();
    }
    Some(())
}

impl Scope {
    fn new(cli: &Cli, options: &Options) -> Result<Self, Failure> {
        if matches!((options.von, options.bis), (Some(von), Some(bis)) if von > bis) {
            return Err(Failure::new(
                2,
                "usage_error",
                "--von must not be later than --bis",
            ));
        }
        let mut datenarten = Vec::with_capacity(options.datenart.len());
        for (index, name) in options.datenart.iter().enumerate() {
            let Some(known) = DATENARTEN.iter().copied().find(|known| known == name) else {
                let suggestion = DATENARTEN
                    .iter()
                    .copied()
                    .min_by_key(|known| {
                        if known.eq_ignore_ascii_case(name) {
                            0
                        } else {
                            strsim::levenshtein(known, name) + 1
                        }
                    })
                    .expect("nonempty Datenart list");
                let failure = Failure::new(
                    2,
                    "datenart_unknown",
                    format!("Unknown Bereitstellungsdatenart `{}`", name.escape_debug()),
                )
                .note("Bereitstellungsdatenart names are case-sensitive")
                .help(format!("Did you mean `{suggestion}`? Replace that --datenart value in your original command."));
                return Err(
                    match options.correction(cli, Some((index, suggestion)), false) {
                        Some(command) => failure.suggestion(command),
                        None => failure,
                    },
                );
            };
            datenarten.push(known);
        }
        Ok(Self {
            einschraenkung: if options.neue || !options.alle {
                Einschraenkung::Neue
            } else {
                Einschraenkung::Alle
            },
            von: options.von,
            bis: options.bis,
            datenarten,
        })
    }

    fn execute(
        &self,
        eric: &impl AuthenticatedEric,
        hersteller_id: &str,
        test: bool,
        warnings: &mut Vec<config::Warning>,
        now: OffsetDateTime,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<Vec<Count>, Failure> {
        eric.inspect(trace, |xml| {
            if certificate::check_for_request(xml, now, warnings)? && !test {
                return Err(test_certificate_error());
            }
            Ok(())
        })?;
        let xml = CString::new(status_request(
            hersteller_id,
            test.then_some("700000001"),
            self.einschraenkung,
            self.von,
            self.bis,
        ))
        .expect("NUL-free PostfachStatus XML");
        eric.send(&xml, c"PostfachStatus_32", trace, |code, xml, text| {
            self.read(code, xml, text)
        })
    }

    fn read(&self, code: i32, xml: &str, mut text: Option<String>) -> Result<Vec<Count>, Failure> {
        if code != 0 {
            if let Some(failure) = decode_rejection(xml, code, &mut text) {
                return Err(failure);
            }
            return Err(eric_failure(code, text));
        }
        let reply = quick_xml::de::from_str::<Reply<Payload>>(xml)
            .map_err(|_| decode_rejection(xml, code, &mut text).unwrap_or_else(invalid_reply))?;
        if let Some(failure) = reply.rejection(code, &mut text) {
            return Err(failure);
        }
        let blocks = reply.data.ok_or_else(invalid_reply)?.blocks;
        let mut counts = Vec::new();
        let mut answered = false;
        for block in blocks {
            for answer in block.payload.into_iter().flat_map(|p| p.collection.status) {
                let effective = answer.scope.unwrap_or(Einschraenkung::Alle);
                if effective != self.einschraenkung {
                    return Err(invalid_reply());
                }
                answered = true;
                if answer.role.as_deref() == Some("vertreter") {
                    continue;
                }
                for count in answer.counts {
                    if count.role.as_deref() == Some("vertreter")
                        || (!self.datenarten.is_empty()
                            && !self.datenarten.contains(&count.name.as_str()))
                        || counts
                            .iter()
                            .any(|existing: &Count| existing.name == count.name)
                    {
                        continue;
                    }
                    counts.push(Count {
                        name: count.name,
                        anzahl: count.anzahl,
                    });
                }
            }
        }
        if !answered {
            return Err(invalid_reply());
        }
        Ok(counts)
    }
}

// PostfachStatus has no Datenart filter on the wire; filter its counts locally.
fn status_request(
    hersteller_id: &str,
    testmerker: Option<&str>,
    einschraenkung: Einschraenkung,
    von: Option<Date>,
    bis: Option<Date>,
) -> String {
    let mut xml = String::with_capacity(1024);
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>
<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">
<TransferHeader version="11">
<Verfahren>ElsterDatenabholung</Verfahren><DatenArt>PostfachStatus</DatenArt><Vorgang>send-Auth</Vorgang>
"#);
    if let Some(marker) = testmerker {
        writeln!(xml, "<Testmerker>{marker}</Testmerker>").unwrap();
    }
    write!(xml, r#"<HerstellerID>{hersteller_id}</HerstellerID><DatenLieferant>KIECLI</DatenLieferant>
<Datei><Verschluesselung>CMSEncryptedData</Verschluesselung><Kompression>GZIP</Kompression><TransportSchluessel/></Datei>
</TransferHeader>
<DatenTeil><Nutzdatenblock>
<NutzdatenHeader version="11"><NutzdatenTicket>1</NutzdatenTicket><Empfaenger id="L">CS</Empfaenger>
<Hersteller><ProduktName>KIECLI</ProduktName><ProduktVersion>{}</ProduktVersion></Hersteller></NutzdatenHeader>
<Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32">
<PostfachStatus einschraenkung="{}" rolle="inhaber""#, env!("CARGO_PKG_VERSION"), einschraenkung.as_str()).unwrap();
    for (name, date) in [("von", von), ("bis", bis)] {
        if let Some(date) = date {
            let (year, month, day) = date.to_calendar_date();
            write!(xml, " {name}=\"{year:04}{:02}{day:02}\"", u8::from(month)).unwrap();
        }
    }
    xml.push_str("/>\n</Datenabholung></Nutzdaten>\n</Nutzdatenblock></DatenTeil>\n</Elster>\n");
    xml
}

#[derive(Deserialize)]
struct Reply<T> {
    #[serde(rename = "TransferHeader")]
    header: Header,
    #[serde(rename = "DatenTeil")]
    data: Option<Data<T>>,
}

#[derive(Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct Data<T> {
    #[serde(rename = "Nutzdatenblock", default)]
    blocks: Vec<Block<T>>,
}

#[derive(Deserialize)]
struct Block<T> {
    #[serde(rename = "NutzdatenHeader")]
    header: Option<Header>,
    #[serde(rename = "Nutzdaten")]
    payload: Option<T>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "Datenabholung")]
    collection: Collection,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(rename = "PostfachStatusAntwort", default)]
    status: Vec<Answer>,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(rename = "@einschraenkung")]
    scope: Option<Einschraenkung>,
    #[serde(rename = "@rolle")]
    role: Option<String>,
    #[serde(rename = "DatenartBereitstellung", default)]
    counts: Vec<WireCount>,
}

#[derive(Deserialize)]
struct WireCount {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@anzahltreffer")]
    anzahl: u64,
    #[serde(rename = "@rolle")]
    role: Option<String>,
}

#[derive(Deserialize)]
struct Header {
    #[serde(rename = "RC")]
    rc: Option<ReturnCodes>,
}

#[derive(Deserialize)]
struct ReturnCodes {
    #[serde(rename = "Rueckgabe")]
    result: ReturnCode,
    #[serde(rename = "Stack", default)]
    stack: Vec<ReturnCode>,
}

#[derive(Deserialize)]
struct ReturnCode {
    #[serde(rename = "Code")]
    code: i32,
    #[serde(rename = "Text")]
    text: Option<String>,
}

impl<T> Reply<T> {
    fn rejection(&self, code: i32, text: &mut Option<String>) -> Option<Failure> {
        let codes = self
            .header
            .rc
            .iter()
            .chain(
                self.data
                    .iter()
                    .flat_map(|data| &data.blocks)
                    .filter_map(|block| block.header.as_ref()?.rc.as_ref()),
            )
            .flat_map(|rc| std::iter::once(&rc.result).chain(&rc.stack))
            .filter(|rc| rc.code != 0);
        let first = codes.clone().next()?;
        let mapped = codes
            .clone()
            .find(|rc| {
                matches!(rc.code, 190017608 | 190010802 | 130025001 | 130025002)
                    || rejected_hersteller_id(rc)
            })
            .unwrap_or(first);
        let mut failure = elster_failure(mapped);
        if code != 0 {
            failure = failure.eric_cause(code, text.take());
        }
        for rc in codes {
            failure = failure.cause("elster", Some(rc.code), rc.text.clone());
        }
        Some(failure.note(eric::LOG_NOTE))
    }
}

fn decode_rejection(xml: &str, code: i32, text: &mut Option<String>) -> Option<Failure> {
    quick_xml::de::from_str::<Reply<IgnoredAny>>(xml)
        .ok()?
        .rejection(code, text)
}

fn rejected_hersteller_id(rc: &ReturnCode) -> bool {
    rc.code == 601015001
        && rc
            .text
            .as_deref()
            .is_some_and(|text| text.contains("HerstellerID ist nicht zugelassen"))
}

fn invalid_reply() -> Failure {
    Failure::new(
        1,
        "postfach_status_invalid",
        "ELSTER did not return a valid PostfachStatus answer",
    )
    .note(eric::LOG_NOTE)
}

fn elster_failure(rc: &ReturnCode) -> Failure {
    match rc.code {
        190017608 => Failure::new(5, "postfach_too_many_entries", "ELSTER found more than 500,000 matching entries")
            .help("Use --von or --bis to narrow the date range. --datenart only filters returned counts for PostfachStatus."),
        190010802 => Failure::new(5, "datenart_not_released", "ELSTER has not released the Bereitstellungsdatenart"),
        601015001 if rejected_hersteller_id(rc) => {
            Failure::new(3, "hersteller_id_rejected", "ELSTER did not admit the HerstellerID")
        }
        130025001 => Failure::new(3, "certificate_not_admitted", "ELSTER did not admit the Certificate for this procedure"),
        130025002 => test_certificate_error(),
        _ => Failure::new(5, "elster_rejected", "ELSTER rejected the PostfachStatus request"),
    }
}

fn eric_failure(code: i32, text: Option<String>) -> Failure {
    let failure = match code {
        610301202 => Failure::new(3, "hersteller_id_blocked", "ERiC blocked the HerstellerID"),
        610201106 => Failure::new(3, "pin_wrong", "The Certificate PIN is incorrect"),
        610201107 => Failure::new(3, "pin_locked", "The Certificate PIN is locked"),
        610101200 | 610101271 | 610101276 | 610101278 | 610101279 | 610101283 => {
            Failure::transient(
                "eric_network_failed",
                "ERiC could not communicate with ELSTER",
            )
        }
        610101210 | 610101292 => Failure::new(
            5,
            "elster_rejected",
            "ELSTER rejected the PostfachStatus request",
        ),
        _ => Failure::new(
            1,
            "eric_process_failed",
            "ERiC could not process PostfachStatus",
        ),
    };
    failure.eric_cause(code, text).note(eric::LOG_NOTE)
}

fn test_certificate_error() -> Failure {
    Failure::new(
        3,
        TEST_CERTIFICATE_REQUIRES_TESTMERKER,
        "A test Certificate requires --test",
    )
    .help("Add --test to your original command (replace --no-test if present).")
}

#[derive(Serialize)]
struct Count {
    name: String,
    anzahl: u64,
}

#[derive(Serialize)]
struct Status {
    bereitstellungsdatenarten: Vec<Count>,
    einschraenkung: Einschraenkung,
    test: bool,
    profile: Option<String>,
    warnings: Vec<config::Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<Vec<eric::TraceEvent>>,
}

impl Status {
    fn write_text(&self, mut out: impl Write) -> std::io::Result<()> {
        writeln!(
            out,
            "Postfach counts (ELSTER may omit some kinds): {}",
            self.einschraenkung.as_str()
        )?;
        for count in &self.bereitstellungsdatenarten {
            writeln!(out, "{}: {}", count.name.escape_debug(), count.anzahl)?;
        }
        writeln!(
            out,
            "Testmerker: {}; profile: {}",
            self.test,
            self.profile.as_deref().unwrap_or("none").escape_debug()
        )?;
        for warning in &self.warnings {
            warning.write_text(&mut out)?;
        }
        if let Some(trace) = &self.trace {
            for event in trace {
                writeln!(out, "{}: {} ms", event.call, event.duration_ms)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn status(cli: &Cli, options: &Options, mut out: impl Write) -> Result<(), Failure> {
    let scope = Scope::new(cli, options)?;
    let (mut settings, path, pin, hersteller_id) = config::request_settings(cli)?;
    let installed = eric::installed_dir(&settings.eric_dir)?;
    let dir = installed.as_deref().unwrap_or(&settings.eric_dir);
    let mut probe = eric::Probe::new(dir, cli.verbose);
    let counts = probe
        .with_session(dir, cli.eric_log.as_deref(), |session, trace| {
            session.with_certificate(&path, trace, |cert, trace| {
                scope.execute(
                    &(cert, &pin),
                    &hersteller_id,
                    settings.test,
                    &mut settings.warnings,
                    OffsetDateTime::now_utc(),
                    trace,
                )
            })
        })
        .map_err(|failure| {
            if failure.code() != TEST_CERTIFICATE_REQUIRES_TESTMERKER {
                return failure;
            }
            match options.correction(cli, None, true) {
                Some(command) => failure.suggestion(command),
                None => failure,
            }
        })?;
    let status = Status {
        bereitstellungsdatenarten: counts,
        einschraenkung: scope.einschraenkung,
        test: settings.test,
        profile: settings.profile,
        warnings: settings.warnings,
        trace: probe.trace,
    };
    match cli.format {
        Format::Json => write_json(out, &status),
        Format::Text => status.write_text(&mut out),
    }
    .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
}
