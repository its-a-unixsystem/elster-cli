// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

mod anfrage;

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
    /// List Postfach metadata without collecting or confirming Nachrichten.
    Anfrage(Options),
}

#[derive(Args)]
pub(crate) struct Options {
    /// Include only this Bereitstellungsdatenart (repeatable, case-sensitive).
    #[arg(long, value_name = "NAME")]
    datenart: Vec<String>,
    /// Include only Nachrichten not yet confirmed (default).
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

#[derive(Clone, Copy)]
enum Operation {
    Status,
    Anfrage,
}

impl Operation {
    fn command(self) -> &'static str {
        match self {
            Self::Status => "status",
            Self::Anfrage => "anfrage",
        }
    }

    fn datenart(self) -> &'static str {
        match self {
            Self::Status => "PostfachStatus",
            Self::Anfrage => "PostfachAnfrage",
        }
    }
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
        operation: Operation,
        datenart: Option<(usize, &str)>,
        enable_test: bool,
    ) -> Option<String> {
        // PIN commands can contain secrets; do not reconstruct the invocation in that case.
        if cli.pin_command.is_some() {
            return None;
        }
        let mut command = format!("kiecli postfach {}", operation.command());
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
    fn new(cli: &Cli, options: &Options, operation: Operation) -> Result<Self, Failure> {
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
                    match options.correction(cli, operation, Some((index, suggestion)), false) {
                        Some(command) => failure.suggestion(command),
                        None => failure,
                    },
                );
            };
            if !datenarten.contains(&known) {
                datenarten.push(known);
            }
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

    fn preflight(
        eric: &impl AuthenticatedEric,
        test: bool,
        warnings: &mut Vec<config::Warning>,
        now: OffsetDateTime,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<(), Failure> {
        eric.inspect(trace, |xml| {
            if certificate::check_for_request(xml, now, warnings)? && !test {
                return Err(test_certificate_error());
            }
            Ok(())
        })
    }

    fn execute(
        &self,
        eric: &impl AuthenticatedEric,
        hersteller_id: &str,
        test: bool,
        trace: &mut Option<Vec<TraceEvent>>,
    ) -> Result<Vec<Count>, Failure> {
        let xml = self.request(Operation::Status, hersteller_id, test, None);
        eric.send(&xml, c"PostfachStatus_32", trace, |code, xml, text| {
            self.read(code, xml, text)
        })
    }

    fn read(&self, code: i32, xml: &str, text: Option<String>) -> Result<Vec<Count>, Failure> {
        let operation = Operation::Status;
        let reply = operation.read_reply::<Payload>(code, xml, text)?;
        let blocks = reply.data.ok_or_else(|| operation.invalid_reply())?.blocks;
        let mut counts = Vec::new();
        let mut answered = false;
        for block in blocks {
            for answer in block.payload.into_iter().flat_map(|p| p.collection.status) {
                let effective = answer.scope.unwrap_or(Einschraenkung::Alle);
                if effective != self.einschraenkung {
                    return Err(operation.invalid_reply());
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
            return Err(operation.invalid_reply());
        }
        Ok(counts)
    }

    fn anfrage_datenarten(&self) -> &[&'static str] {
        if self.datenarten.is_empty() {
            DATENARTEN
        } else {
            &self.datenarten
        }
    }

    fn request(
        &self,
        operation: Operation,
        hersteller_id: &str,
        test: bool,
        cursor: Option<&str>,
    ) -> CString {
        let datenart = operation.datenart();
        let mut xml = String::with_capacity(1024);
        write!(xml, r#"<?xml version="1.0" encoding="UTF-8"?>
<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">
<TransferHeader version="11">
<Verfahren>ElsterDatenabholung</Verfahren><DatenArt>{datenart}</DatenArt><Vorgang>send-Auth</Vorgang>
"#).unwrap();
        if test {
            xml.push_str("<Testmerker>700000001</Testmerker>\n");
        }
        write!(xml, r#"<HerstellerID>{hersteller_id}</HerstellerID><DatenLieferant>KIECLI</DatenLieferant>
<Datei><Verschluesselung>CMSEncryptedData</Verschluesselung><Kompression>GZIP</Kompression><TransportSchluessel/></Datei>
</TransferHeader>
<DatenTeil><Nutzdatenblock>
<NutzdatenHeader version="11"><NutzdatenTicket>1</NutzdatenTicket><Empfaenger id="L">CS</Empfaenger>
<Hersteller><ProduktName>KIECLI</ProduktName><ProduktVersion>{}</ProduktVersion></Hersteller></NutzdatenHeader>
<Nutzdaten><Datenabholung xmlns="http://finkonsens.de/elster/elsterdatenabholung/v3" version="32">
<{datenart} einschraenkung="{}" rolle="inhaber""#, env!("CARGO_PKG_VERSION"), self.einschraenkung.as_str()).unwrap();
        for (name, date) in [("von", self.von), ("bis", self.bis)] {
            if let Some(date) = date {
                let (year, month, day) = date.to_calendar_date();
                write!(xml, " {name}=\"{year:04}{:02}{day:02}\"", u8::from(month)).unwrap();
            }
        }
        match operation {
            // PostfachStatus has no Datenart filter on the wire.
            Operation::Status => xml.push_str("/>\n"),
            Operation::Anfrage => {
                xml.push_str(" max=\"10000\"");
                if let Some(cursor) = cursor {
                    write!(
                        xml,
                        " paginierungLetzteBestId=\"{}\"",
                        quick_xml::escape::escape(cursor)
                    )
                    .unwrap();
                }
                xml.push_str(">\n");
                for name in self.anfrage_datenarten() {
                    writeln!(xml, "<DatenartBereitstellung name=\"{name}\"/>").unwrap();
                }
                xml.push_str("</PostfachAnfrage>\n");
            }
        }
        xml.push_str("</Datenabholung></Nutzdaten>\n</Nutzdatenblock></DatenTeil>\n</Elster>\n");
        CString::new(xml).expect("NUL-free Postfach request")
    }
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
    fn rejection(
        &self,
        operation: Operation,
        code: i32,
        text: &mut Option<String>,
    ) -> Option<Failure> {
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
        let mut failure = operation.elster_failure(mapped);
        if code != 0 {
            failure = failure.eric_cause(code, text.take());
        }
        for rc in codes {
            failure = failure.cause("elster", Some(rc.code), rc.text.clone());
        }
        Some(failure.note(eric::LOG_NOTE))
    }
}

impl Operation {
    fn read_reply<T: for<'de> Deserialize<'de>>(
        self,
        code: i32,
        xml: &str,
        mut text: Option<String>,
    ) -> Result<Reply<T>, Failure> {
        if code != 0 {
            return Err(self
                .decode_rejection(xml, code, &mut text)
                .unwrap_or_else(|| self.eric_failure(code, text)));
        }
        let reply = quick_xml::de::from_str::<Reply<T>>(xml).map_err(|_| {
            self.decode_rejection(xml, code, &mut text)
                .unwrap_or_else(|| self.invalid_reply())
        })?;
        if let Some(failure) = reply.rejection(self, code, &mut text) {
            return Err(failure);
        }
        Ok(reply)
    }

    fn decode_rejection(self, xml: &str, code: i32, text: &mut Option<String>) -> Option<Failure> {
        quick_xml::de::from_str::<Reply<IgnoredAny>>(xml)
            .ok()?
            .rejection(self, code, text)
    }

    fn invalid_reply(self) -> Failure {
        let error = match self {
            Self::Status => "postfach_status_invalid",
            Self::Anfrage => "postfach_anfrage_invalid",
        };
        Failure::new(
            1,
            error,
            format!("ELSTER did not return a valid {} answer", self.datenart()),
        )
        .note(eric::LOG_NOTE)
    }

    fn elster_failure(self, rc: &ReturnCode) -> Failure {
        match rc.code {
            190017608 => Failure::new(5, "postfach_too_many_entries", "ELSTER found more than 500,000 matching entries")
                .help(match self {
                    Self::Status => "Use --von or --bis to narrow the date range. --datenart only filters returned counts for PostfachStatus.",
                    Self::Anfrage => "Use --von, --bis or --datenart to narrow the Anfrage.",
                }),
            190010802 => Failure::new(5, "datenart_not_released", "ELSTER has not released the Bereitstellungsdatenart"),
            601015001 if rejected_hersteller_id(rc) => {
                Failure::new(3, "hersteller_id_rejected", "ELSTER did not admit the HerstellerID")
            }
            130025001 => Failure::new(3, "certificate_not_admitted", "ELSTER did not admit the Certificate for this procedure"),
            130025002 => test_certificate_error(),
            _ => self.rejected(),
        }
    }

    fn rejected(self) -> Failure {
        Failure::new(
            5,
            "elster_rejected",
            format!("ELSTER rejected the {} request", self.datenart()),
        )
    }

    fn eric_failure(self, code: i32, text: Option<String>) -> Failure {
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
            610101210 | 610101292 => self.rejected(),
            _ => Failure::new(
                1,
                "eric_process_failed",
                format!("ERiC could not process {}", self.datenart()),
            ),
        };
        failure.eric_cause(code, text).note(eric::LOG_NOTE)
    }
}

fn rejected_hersteller_id(rc: &ReturnCode) -> bool {
    rc.code == 601015001
        && rc
            .text
            .as_deref()
            .is_some_and(|text| text.contains("HerstellerID ist nicht zugelassen"))
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
#[serde(untagged)]
enum Results {
    Status {
        bereitstellungsdatenarten: Vec<Count>,
        einschraenkung: Einschraenkung,
    },
    Anfrage {
        postfach_nachrichten: Vec<anfrage::Nachricht>,
    },
}

#[derive(Serialize)]
struct Output {
    #[serde(flatten)]
    results: Results,
    test: bool,
    profile: Option<String>,
    warnings: Vec<config::Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace: Option<Vec<eric::TraceEvent>>,
}

impl Output {
    fn write_text(&self, mut out: impl Write) -> std::io::Result<()> {
        match &self.results {
            Results::Status {
                bereitstellungsdatenarten,
                einschraenkung,
            } => {
                writeln!(
                    out,
                    "Postfach counts (ELSTER may omit some kinds): {}",
                    einschraenkung.as_str()
                )?;
                for count in bereitstellungsdatenarten {
                    writeln!(out, "{}: {}", count.name.escape_debug(), count.anzahl)?;
                }
            }
            Results::Anfrage {
                postfach_nachrichten,
            } => {
                writeln!(out, "Postfach-Nachrichten: {}", postfach_nachrichten.len())?;
                for nachricht in postfach_nachrichten {
                    nachricht.write_text(&mut out)?;
                }
            }
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
                if let Some(page) = event.page {
                    writeln!(
                        out,
                        "{} (page {page}): {} ms",
                        event.call, event.duration_ms
                    )?;
                    continue;
                }
                writeln!(out, "{}: {} ms", event.call, event.duration_ms)?;
            }
        }
        Ok(())
    }
}

pub(crate) fn run(cli: &Cli, command: &Command, mut out: impl Write) -> Result<(), Failure> {
    let (options, operation) = match command {
        Command::Status(options) => (options, Operation::Status),
        Command::Anfrage(options) => (options, Operation::Anfrage),
    };
    let scope = Scope::new(cli, options, operation)?;
    let (mut settings, path, pin, hersteller_id) = config::request_settings(cli)?;
    let installed = eric::installed_dir(&settings.eric_dir)?;
    let dir = installed.as_deref().unwrap_or(&settings.eric_dir);
    let mut probe = eric::Probe::new(dir, cli.verbose);
    let results = probe
        .with_session(dir, cli.eric_log.as_deref(), |session, trace| {
            session.with_certificate(&path, trace, |cert, trace| {
                let eric = (cert, &pin);
                Scope::preflight(
                    &eric,
                    settings.test,
                    &mut settings.warnings,
                    OffsetDateTime::now_utc(),
                    trace,
                )?;
                match operation {
                    Operation::Status => Ok(Results::Status {
                        bereitstellungsdatenarten: scope.execute(
                            &eric,
                            &hersteller_id,
                            settings.test,
                            trace,
                        )?,
                        einschraenkung: scope.einschraenkung,
                    }),
                    Operation::Anfrage => Ok(Results::Anfrage {
                        postfach_nachrichten: anfrage::execute(
                            &scope,
                            &eric,
                            &hersteller_id,
                            settings.test,
                            trace,
                        )?,
                    }),
                }
            })
        })
        .map_err(|failure| {
            if failure.code() != TEST_CERTIFICATE_REQUIRES_TESTMERKER {
                return failure;
            }
            match options.correction(cli, operation, None, true) {
                Some(command) => failure.suggestion(command),
                None => failure,
            }
        })?;
    let output = Output {
        results,
        test: settings.test,
        profile: settings.profile,
        warnings: settings.warnings,
        trace: probe.trace,
    };
    match cli.format {
        Format::Json => write_json(out, &output),
        Format::Text => output.write_text(&mut out),
    }
    .map_err(|err| Failure::new(1, "output_failed", err.to_string()))
}
