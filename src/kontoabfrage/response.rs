// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::io::Write;

use quick_xml::{Reader, events::Event};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::{Date, Month, PrimitiveDateTime, format_description::well_known::Iso8601};

use crate::{eric, error::Failure};

use super::Input;

#[cfg(test)]
mod tests;

#[derive(Serialize, JsonSchema)]
pub(super) struct Reply {
    teile: Vec<Teil>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    pub(super) xml: Option<String>,
    #[serde(skip)]
    pub(super) outcome: Result<(), Failure>,
}

#[derive(Serialize, JsonSchema)]
#[schemars(extend("allOf" = [
    {
        "if": {"required": ["antwort_format"], "properties": {"antwort_format": {"const": "alt"}}},
        "then": {"required": ["steuerarten"]}
    },
    {
        "if": {"required": ["antwort_format"], "properties": {"antwort_format": {"const": "biene"}}},
        "then": {"required": ["zeilen", "summen"]}
    },
    {"dependentRequired": {
        "steuerarten": ["antwort_format"],
        "zeilen": ["antwort_format"],
        "summen": ["antwort_format"]
    }}
]))]
struct Teil {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    steuerart: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(with = "(i32, u16)", description = "Date as [year, ordinal day].")]
    ab: Option<Date>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    ein_jahr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    jahr: Option<u16>,
    bearbeitungshinweis: Hinweis,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    antwort_format: Option<AntwortFormat>,
    #[serde(flatten)]
    alt: Option<Alt>,
    #[serde(flatten)]
    biene: Option<Biene>,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
enum AntwortFormat {
    Alt,
    Biene,
}

#[derive(Serialize, JsonSchema)]
struct Hinweis {
    code: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    text: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Alt {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    abfrage_tagesdatum: Option<String>,
    steuerarten: Vec<AltSteuerart>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    gesamtsumme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    erlaeuterung: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct AltSteuerart {
    klartext: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    erlaeuterung: Option<String>,
    teilbetraege: Vec<AltTeilbetrag>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    gesamtbetrag: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct AltTeilbetrag {
    zeitraum: Option<String>,
    faelligkeit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    wert: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    erlaeuterung: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Biene {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    stpfl: Option<Stpfl>,
    zeilen: Vec<Zeile>,
    summen: Vec<Summe>,
}

#[derive(Serialize, JsonSchema)]
struct Stpfl {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    finanzamt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    steuernummer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    abfragedatum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    kein_leistungsgebot: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Zeile {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    faelligkeit_wertstellung: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    betrag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    zufluss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    abfluss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    offen_forderung: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    offen_verbindlichkeit: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    erlaeuterungen: Option<String>,
}

#[derive(Serialize, JsonSchema)]
struct Summe {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    betrag: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    zufluss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    abfluss: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    offen_forderung: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schemars(required)]
    offen_verbindlichkeit: Option<String>,
}

#[derive(Deserialize)]
struct Envelope {
    #[serde(rename = "TransferHeader")]
    transfer_header: Option<Header>,
    #[serde(rename = "DatenTeil")]
    data: Option<Data>,
}

#[derive(Deserialize)]
struct Data {
    #[serde(rename = "Nutzdatenblock", default)]
    blocks: Vec<Block>,
}

#[derive(Deserialize)]
struct Block {
    #[serde(rename = "NutzdatenHeader")]
    header: Option<Header>,
    #[serde(rename = "Nutzdaten")]
    payload: Option<Payload>,
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "kontoabfrage")]
    kontoabfrage: Option<Kontoabfrage>,
}

#[derive(Deserialize)]
struct Kontoabfrage {
    #[serde(rename = "kontoabfrage-teil", default)]
    teile: Vec<WireTeil>,
}

#[derive(Deserialize)]
struct WireTeil {
    #[serde(rename = "kontoabfrage-input")]
    input: Option<WireInput>,
    #[serde(rename = "kontoabfrage-output")]
    alt: Option<WireAlt>,
    #[serde(rename = "kontoabfrage_antwort")]
    biene: Option<WireBiene>,
    #[serde(rename = "bearbeitungshinweis")]
    hinweis: Option<WireHinweis>,
}

#[derive(Deserialize)]
struct WireInput {
    #[serde(rename = "kontoabfrage-art")]
    art: Option<String>,
    steuernummer: Option<String>,
    steuerart: Option<String>,
    #[serde(rename = "wertstellungsdatum")]
    wertstellung: Option<WireDate>,
    zeitraum: Option<String>,
}

#[derive(Deserialize)]
struct WireDate {
    #[serde(rename = "$text", default)]
    value: String,
    #[serde(rename = "@option")]
    option: Option<String>,
}

#[derive(Deserialize)]
struct WireHinweis {
    #[serde(rename = "return-code")]
    code: Option<String>,
    #[serde(rename = "return-text")]
    text: Option<String>,
    #[serde(rename = "stack-code")]
    stack_code: Option<String>,
    #[serde(rename = "stack-text")]
    stack_text: Option<String>,
}

#[derive(Deserialize)]
struct WireAlt {
    #[serde(rename = "abfrage-tagesdatum")]
    date: Option<String>,
    #[serde(rename = "abfrage-steuerart", default)]
    steuerarten: Vec<WireAltSteuerart>,
    #[serde(rename = "abfrage-gesamtsumme")]
    total: Option<String>,
    #[serde(rename = "abfrage-erlaeuterung")]
    explanation: Option<String>,
}

#[derive(Deserialize)]
struct WireAltSteuerart {
    #[serde(rename = "steuerart-klartext")]
    klartext: Option<String>,
    #[serde(rename = "steuerart-erlaeuterung")]
    explanation: Option<String>,
    #[serde(rename = "steuerart-teilbetrag", default)]
    parts: Vec<WireAltTeilbetrag>,
    #[serde(rename = "steuerart-gesamtbetrag")]
    total: Option<String>,
}

#[derive(Deserialize)]
struct WireAltTeilbetrag {
    #[serde(rename = "teilbetrag-zeitraum")]
    period: Option<String>,
    #[serde(rename = "teilbetrag-faelligkeit")]
    due: Option<String>,
    #[serde(rename = "teilbetrag-wert")]
    value: Option<String>,
    #[serde(rename = "teilbetrag-erlaeuterung")]
    explanation: Option<String>,
}

#[derive(Deserialize)]
struct WireBiene {
    stpfl: Option<WireStpfl>,
    #[serde(rename = "steuerkontoTilgungen")]
    payments: Option<WireRows>,
    #[serde(rename = "steuerkontoOffeneBetraege")]
    open: Option<WireRows>,
    steuerkontoauszug: Option<WireRows>,
}

#[derive(Deserialize)]
struct WireStpfl {
    finanzamt: Option<String>,
    name: Option<String>,
    steuernummer: Option<String>,
    abfragedatum: Option<String>,
    #[serde(rename = "keinLeistungsgebot")]
    no_performance: Option<String>,
}

#[derive(Deserialize)]
struct WireRows {
    #[serde(rename = "zeile", default)]
    rows: Vec<WireRow>,
    #[serde(rename = "summen", default)]
    sums: Vec<WireSum>,
}

#[derive(Deserialize)]
struct WireRow {
    text: Option<String>,
    #[serde(rename = "faelligkeitWertstellung")]
    due: Option<String>,
    betrag: Option<String>,
    zufluss: Option<String>,
    abfluss: Option<String>,
    #[serde(rename = "offenForderung")]
    open_claim: Option<String>,
    #[serde(rename = "offenVerbindlichkeit")]
    open_liability: Option<String>,
    erlaeuterungen: Option<String>,
}

#[derive(Deserialize)]
struct WireSum {
    text: Option<String>,
    betrag: Option<String>,
    zufluss: Option<String>,
    abfluss: Option<String>,
    #[serde(rename = "offenForderung")]
    open_claim: Option<String>,
    #[serde(rename = "offenVerbindlichkeit")]
    open_liability: Option<String>,
}

#[derive(Deserialize)]
struct Header {
    #[serde(rename = "RC")]
    rc: Option<ReturnCodes>,
}

#[derive(Deserialize)]
struct ReturnCodes {
    #[serde(rename = "Rueckgabe")]
    result: Option<ReturnCode>,
    #[serde(rename = "Stack", default)]
    stack: Vec<ReturnCode>,
}

#[derive(Deserialize, Clone)]
struct ReturnCode {
    #[serde(rename = "Code")]
    code: i32,
    #[serde(rename = "Text")]
    text: Option<String>,
}

pub(super) fn read(
    code: i32,
    xml: &str,
    text: Option<String>,
    steuernummer: &str,
    expected: &[Input],
    include_xml: bool,
) -> Result<Reply, Failure> {
    let parsed = quick_xml::de::from_str::<Envelope>(xml).map_err(|_| invalid_reply())?;
    let mut all_codes = Vec::new();
    if let Some(header) = &parsed.transfer_header { collect_codes(header, &mut all_codes); }
    let blocks = parsed.data.as_ref().map(|data| &data.blocks);
    if let Some(blocks) = blocks {
        for block in blocks { if let Some(header) = &block.header { collect_codes(header, &mut all_codes); } }
    }
    let mut wire_parts = Vec::new();
    if let Some(blocks) = blocks {
        for block in blocks {
            if let Some(konto) = block.payload.as_ref().and_then(|payload| payload.kontoabfrage.as_ref()) {
                wire_parts.extend(konto.teile.iter());
            }
        }
    }
    if wire_parts.is_empty() {
        if let Some((server_code, server_text)) = all_codes.iter().find(|(server_code, _)| *server_code != 0) {
            let mut failure = mapped_failure(*server_code, server_text.clone());
            if code != 0 { failure = failure.eric_cause(code, text).note(eric::LOG_NOTE); }
            return Err(failure);
        }
        return Err(if code != 0 { transport_failure(code, text) } else { invalid_reply() });
    }
    let mut used = vec![false; expected.len()];
    let mut matched = Vec::with_capacity(wire_parts.len());
    let mut teile = Vec::with_capacity(wire_parts.len());
    for part in &wire_parts {
        let input = part.input.as_ref().ok_or_else(invalid_reply)?;
        let index = expected.iter().enumerate().find(|(index, wanted)| !used[*index] && input_matches(input, wanted, steuernummer)).map(|(index, _)| index).ok_or_else(invalid_reply)?;
        used[index] = true;
        matched.push(index);
        teile.push(convert_part(part, &expected[index]));
    }
    if used.iter().any(|used| !used) { return Err(invalid_reply()); }
    let mut outcome = Ok(());
    for (part, index) in wire_parts.iter().zip(matched) {
        let hint = part.hinweis.as_ref().map(hint_code).unwrap_or((0, None));
        if hint.0 != 0 { outcome = Err(mapped_failure(hint.0, hint.1)); break; }
        if teile[index].bearbeitungshinweis.code != 0 {
            let note = &teile[index].bearbeitungshinweis;
            outcome = Err(mapped_failure(note.code, note.text.clone()));
            break;
        }
    }
    if outcome.is_ok()
        && let Some((server_code, server_text)) =
            all_codes.iter().find(|(server_code, _)| *server_code != 0)
    {
        outcome = Err(mapped_failure(*server_code, server_text.clone()));
    }
    if let Err(mut failure) = outcome {
        for (server_code, server_text) in &all_codes { if *server_code != 0 { failure = failure.cause("elster", Some(*server_code), server_text.clone()); } }
        if code != 0 { failure = failure.eric_cause(code, text).note(eric::LOG_NOTE); }
        outcome = Err(failure);
    } else if code != 0 { return Err(transport_failure(code, text)); }
    Ok(Reply { teile, xml: include_xml.then(|| extract_kontoabfrage(xml)).flatten(), outcome })
}

impl Reply {
    pub(super) fn write_text(&self, mut out: impl Write) -> std::io::Result<()> {
        writeln!(out, "Kontoabfrage results: {}", self.teile.len())?;
        for (index, teil) in self.teile.iter().enumerate() {
            write!(out, "Teil {}", index + 1)?;
            if let Some(kind) = &teil.steuerart { write!(out, " {kind}")?; }
            if let Some(date) = teil.ab { write!(out, " from {date}")?; }
            if let Some(year) = teil.jahr { write!(out, " {year}")?; }
            writeln!(out, ": ELSTER {}", teil.bearbeitungshinweis.code)?;
            if let Some(text) = &teil.bearbeitungshinweis.text { writeln!(out, "  {}", text.escape_debug())?; }
            if let Some(alt) = &teil.alt { writeln!(out, "  Legacy response: {} tax types, total {}", alt.steuerarten.len(), alt.gesamtsumme.as_deref().unwrap_or("unknown"))?; }
            if let Some(biene) = &teil.biene { writeln!(out, "  BIENE response: {} rows, {} sums", biene.zeilen.len(), biene.summen.len())?; }
        }
        if let Some(xml) = &self.xml { writeln!(out, "Raw XML: {} bytes", xml.len())?; }
        Ok(())
    }
}

fn convert_part(part: &WireTeil, expected: &Input) -> Teil {
    let hinweis = part.hinweis.as_ref().map(convert_hint).unwrap_or(Hinweis { code: 0, text: None });
    let (steuerart, ab, ein_jahr, jahr) = match expected {
        Input::Open => (None, None, None, None),
        Input::Payment { steuerart, ab, ein_jahr } => (Some(steuerart.as_str().to_owned()), Some(*ab), Some(*ein_jahr), None),
        Input::Assessment { steuerart, jahr } => (Some(steuerart.as_str().to_owned()), None, None, Some(*jahr)),
    };
    let alt = part.alt.as_ref().map(convert_alt);
    let biene = part.biene.as_ref().map(convert_biene);
    let antwort_format = if alt.is_some() {
        Some(AntwortFormat::Alt)
    } else if biene.is_some() {
        Some(AntwortFormat::Biene)
    } else {
        None
    };
    Teil { steuerart, ab, ein_jahr, jahr, bearbeitungshinweis: hinweis, antwort_format, alt, biene }
}

fn convert_hint(hint: &WireHinweis) -> Hinweis {
    let code = hint.code.as_deref().or(hint.stack_code.as_deref()).and_then(|value| value.parse().ok()).unwrap_or(0);
    Hinweis { code, text: hint.text.clone().or_else(|| hint.stack_text.clone()) }
}

fn hint_code(hint: &WireHinweis) -> (i32, Option<String>) {
    let code = hint.code.as_deref().or(hint.stack_code.as_deref()).and_then(|value| value.parse().ok()).unwrap_or(0);
    (code, hint.text.clone().or_else(|| hint.stack_text.clone()))
}

fn convert_alt(alt: &WireAlt) -> Alt {
    Alt {
        abfrage_tagesdatum: alt.date.as_deref().map(normalize_datetime),
        steuerarten: alt.steuerarten.iter().map(|kind| AltSteuerart {
            klartext: kind.klartext.clone(), erlaeuterung: kind.explanation.clone(),
            teilbetraege: kind.parts.iter().map(|part| AltTeilbetrag { zeitraum: part.period.clone(), faelligkeit: part.due.as_deref().map(normalize_date), wert: part.value.as_deref().map(normalize_amount), erlaeuterung: part.explanation.clone() }).collect(),
            gesamtbetrag: kind.total.as_deref().map(normalize_amount),
        }).collect(),
        gesamtsumme: alt.total.as_deref().map(normalize_amount), erlaeuterung: alt.explanation.clone(),
    }
}

fn convert_biene(biene: &WireBiene) -> Biene {
    let mut zeilen = Vec::new();
    let mut summen = Vec::new();
    for rows in [&biene.payments, &biene.open, &biene.steuerkontoauszug].into_iter().flatten() {
        zeilen.extend(rows.rows.iter().map(convert_row));
        summen.extend(rows.sums.iter().map(convert_sum));
    }
    Biene {
        stpfl: biene.stpfl.as_ref().map(|person| Stpfl {
            finanzamt: person.finanzamt.clone(), name: person.name.clone(), steuernummer: person.steuernummer.clone(),
            abfragedatum: person.abfragedatum.as_deref().map(normalize_datetime), kein_leistungsgebot: person.no_performance.clone(),
        }), zeilen, summen,
    }
}

fn convert_row(row: &WireRow) -> Zeile {
    Zeile {
        text: row.text.clone(), faelligkeit_wertstellung: row.due.as_deref().map(normalize_date),
        betrag: row.betrag.as_deref().map(normalize_amount), zufluss: row.zufluss.as_deref().map(normalize_amount),
        abfluss: row.abfluss.as_deref().map(normalize_amount), offen_forderung: row.open_claim.as_deref().map(normalize_amount),
        offen_verbindlichkeit: row.open_liability.as_deref().map(normalize_amount), erlaeuterungen: row.erlaeuterungen.clone(),
    }
}

fn convert_sum(sum: &WireSum) -> Summe {
    Summe {
        text: sum.text.clone(), betrag: sum.betrag.as_deref().map(normalize_amount), zufluss: sum.zufluss.as_deref().map(normalize_amount),
        abfluss: sum.abfluss.as_deref().map(normalize_amount), offen_forderung: sum.open_claim.as_deref().map(normalize_amount),
        offen_verbindlichkeit: sum.open_liability.as_deref().map(normalize_amount),
    }
}

fn input_matches(input: &WireInput, expected: &Input, steuernummer: &str) -> bool {
    if input.steuernummer.as_deref().is_some_and(|number| number.trim() != steuernummer) { return false; }
    match (input.art.as_deref(), expected) {
        (Some("O"), Input::Open) => true,
        (Some("I"), Input::Payment { steuerart, ab, ein_jahr }) => input.steuerart.as_deref().is_some_and(|kind| kind.eq_ignore_ascii_case(steuerart.as_str())) && input.wertstellung.as_ref().is_some_and(|date| date.option.as_deref() == Some(if *ein_jahr { "J" } else { "V" }) && parse_wire_date(&date.value) == Some(*ab)),
        (Some("ZS"), Input::Assessment { steuerart, jahr }) => input.steuerart.as_deref().is_some_and(|kind| kind.eq_ignore_ascii_case(steuerart.as_str())) && input.zeitraum.as_deref().and_then(|year| year.parse().ok()) == Some(*jahr),
        _ => false,
    }
}

fn parse_wire_date(raw: &str) -> Option<Date> {
    let raw = raw.trim();
    if raw.len() != 8 { return None; }
    let day: u8 = raw[0..2].parse().ok()?;
    let month: u8 = raw[2..4].parse().ok()?;
    let year: i32 = raw[4..8].parse().ok()?;
    Date::from_calendar_date(year, Month::try_from(month).ok()?, day).ok()
}

fn normalize_amount(raw: &str) -> String {
    let mut value = raw.trim().replace(' ', "");
    let negative = value.starts_with('-');
    if negative { value.remove(0); }
    if value.contains(',') { value.retain(|ch| ch != '.'); value = value.replace(',', "."); }
    let (whole, fraction) = value.split_once('.').unwrap_or((&value, ""));
    let whole = if whole.is_empty() { "0" } else { whole };
    let fraction = match fraction { "" => "00".to_owned(), part if part.len() == 1 => format!("{part}0"), part => part.to_owned() };
    let mut result = format!("{whole}.{fraction}");
    if negative { result.insert(0, '-'); }
    result
}

fn normalize_date(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() == 10 && raw.as_bytes()[2] == b'.' && raw.as_bytes()[5] == b'.' { format!("{}-{}-{}", &raw[6..10], &raw[3..5], &raw[0..2]) } else { raw.to_owned() }
}

fn normalize_datetime(raw: &str) -> String {
    let raw = raw.trim();
    if raw.len() == 19 && raw.as_bytes()[2] == b'.' && raw.as_bytes()[5] == b'.' { return format!("{}-{}-{}T{}", &raw[6..10], &raw[3..5], &raw[0..2], &raw[11..]); }
    PrimitiveDateTime::parse(raw, &Iso8601::DATE_TIME).ok().and_then(|value| value.format(&Iso8601::DATE_TIME).ok()).unwrap_or_else(|| raw.to_owned())
}

fn collect_codes(header: &Header, codes: &mut Vec<(i32, Option<String>)>) {
    if let Some(rc) = &header.rc {
        if let Some(result) = &rc.result { codes.push((result.code, result.text.clone())); }
        codes.extend(rc.stack.iter().map(|result| (result.code, result.text.clone())));
    }
}

fn mapped_failure(code: i32, text: Option<String>) -> Failure {
    let exit = super::result_exit(code);
    let failure = match code {
        950013001 => Failure::new(exit, "kontoabfrage_partial", "ELSTER returned a partial Kontoabfrage result"),
        230077012 | 950017011 => Failure::new(exit, "kontoabfrage_authorization_required", "ELSTER did not authorize the Kontoabfrage").note("The Steuerkontoabfrage requires an authorisation, such as a Vollmacht or activation in Mein ELSTER.").help("Activate the Steuerkontoabfrage for this Steuernummer in Mein ELSTER, or ask the Finanzamt how to activate it."),
        130025002 => super::test_certificate_error(),
        950017009 | 950017010 | 950019999 => {
            Failure::new(exit, "kontoabfrage_rejected", "ELSTER rejected the Kontoabfrage")
                .help("Contact the Finanzamt about the Kontoabfrage failure.")
        }
        _ if exit == 6 => Failure::transient(
            "kontoabfrage_transient",
            "ELSTER's Kontoabfrage backend did not return a result",
        ),
        _ => Failure::new(exit, "kontoabfrage_rejected", "ELSTER rejected the Kontoabfrage"),
    };
    failure.cause("elster", Some(code), text)
}

fn transport_failure(code: i32, text: Option<String>) -> Failure {
    let failure = match code {
        610101200 | 610101271 | 610101276 | 610101278 | 610101279 | 610101283 => Failure::transient("eric_network_failed", "ERiC could not communicate with ELSTER"),
        610101210 | 610101292 => Failure::new(5, "kontoabfrage_rejected", "ELSTER rejected the Kontoabfrage"),
        _ => Failure::new(1, "eric_process_failed", "ERiC could not process the Kontoabfrage"),
    };
    failure.eric_cause(code, text).note(eric::LOG_NOTE)
}

fn invalid_reply() -> Failure {
    Failure::new(1, "kontoabfrage_invalid", "ELSTER did not return a valid Kontoabfrage answer").note(eric::LOG_NOTE)
}

fn extract_kontoabfrage(xml: &str) -> Option<String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut start = None;
    let mut depth = 0usize;
    loop {
        let offset = reader.buffer_position() as usize;
        match reader.read_event_into(&mut buf).ok()? {
            Event::Start(tag) if tag.local_name().as_ref() == b"kontoabfrage" => { if depth == 0 { start = Some(offset); } depth += 1; }
            Event::End(tag) if tag.local_name().as_ref() == b"kontoabfrage" => { depth = depth.checked_sub(1)?; if depth == 0 { return Some(xml.get(start?..reader.buffer_position() as usize)?.to_owned()); } }
            Event::Empty(tag) if tag.local_name().as_ref() == b"kontoabfrage" => return Some(xml.get(offset..reader.buffer_position() as usize)?.to_owned()),
            Event::Eof => return None,
            _ => {}
        }
        buf.clear();
    }
}
