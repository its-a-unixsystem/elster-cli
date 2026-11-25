// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::io::{self, Write};

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use time::{Date, macros::format_description};

use super::{AuthenticatedEric, Einschraenkung, Failure, Operation, Scope, TraceEvent};

pub(super) fn execute(
    scope: &Scope,
    eric: &impl AuthenticatedEric,
    hersteller_id: &str,
    test: bool,
    trace: &mut Option<Vec<TraceEvent>>,
) -> Result<Vec<Nachricht>, Failure> {
    let operation = Operation::Anfrage;
    let mut nachrichten = Vec::new();
    let mut cursor = None::<String>;
    for page in 1.. {
        let trace_start = trace.as_ref().map_or(0, Vec::len);
        let request = scope.request(operation, hersteller_id, test, cursor.as_deref());
        let next = eric.send(&request, c"PostfachAnfrage_32", trace, |code, xml, text| {
            read_page(scope, code, xml, text, &mut nachrichten)
        })?;
        if let Some(event) = trace
            .as_mut()
            .and_then(|events| events.get_mut(trace_start))
        {
            event.page = Some(page);
        }
        let Some(next) = next else { break };
        // The server defines the cursor as a lexicographically increasing ID.
        if next.is_empty() || cursor.as_ref().is_some_and(|previous| next <= *previous) {
            return Err(operation
                .invalid_reply()
                .note("The Postfach cursor did not advance"));
        }
        cursor = Some(next);
    }
    nachrichten.sort_unstable_by(|a, b| a.id.cmp(&b.id));
    Ok(nachrichten)
}

fn read_page(
    scope: &Scope,
    code: i32,
    xml: &str,
    text: Option<String>,
    nachrichten: &mut Vec<Nachricht>,
) -> Result<Option<String>, Failure> {
    let operation = Operation::Anfrage;
    let reply = operation.read_reply::<Payload>(code, xml, text)?;
    let mut pages = reply
        .data
        .into_iter()
        .flat_map(|data| data.blocks)
        .filter_map(|block| block.payload?.collection.page);
    let page = pages.next().ok_or_else(|| operation.invalid_reply())?;
    if pages.next().is_some() || page.scope.unwrap_or(Einschraenkung::Alle) != scope.einschraenkung
    {
        return Err(operation.invalid_reply());
    }
    for group in page.groups {
        if group.role.as_deref() == Some("vertreter") {
            continue;
        }
        let datenart = scope
            .anfrage_datenarten()
            .iter()
            .copied()
            .find(|name| *name == group.name)
            .ok_or_else(|| operation.invalid_reply())?;
        for message in group.messages.into_iter().flat_map(|list| list.messages) {
            nachrichten.push(message.into_nachricht(datenart)?);
        }
    }
    Ok(page.cursor)
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "Datenabholung")]
    collection: Collection,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(rename = "PostfachAnfrageAntwort")]
    page: Option<Page>,
}

#[derive(Deserialize)]
struct Page {
    #[serde(rename = "@einschraenkung")]
    scope: Option<Einschraenkung>,
    #[serde(rename = "@paginierungLetzteBestId")]
    cursor: Option<String>,
    #[serde(rename = "DatenartBereitstellung", default)]
    groups: Vec<Group>,
}

#[derive(Deserialize)]
struct Group {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@rolle")]
    role: Option<String>,
    #[serde(rename = "Bereitstellungen")]
    messages: Option<Messages>,
}

#[derive(Deserialize)]
struct Messages {
    #[serde(rename = "Bereitstellung", default)]
    messages: Vec<WireNachricht>,
}

#[derive(Deserialize)]
struct WireNachricht {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@bereitstellungsticket")]
    bereitstellungsticket: Option<String>,
    #[serde(rename = "@groesse")]
    groesse: u64,
    #[serde(rename = "MetaInformationen", default)]
    meta: Metadata,
    #[serde(rename = "Datenpaket")]
    datenpaket: Option<WireDatenpaket>,
    #[serde(rename = "Anhaenge")]
    anhaenge: Option<Attachments>,
}

#[derive(Default, Deserialize)]
struct Metadata {
    #[serde(rename = "Meta", default)]
    entries: Vec<Meta>,
    #[serde(rename = "MetaInfoMaps")]
    maps: Option<Maps>,
}

#[derive(Deserialize)]
struct Maps {
    #[serde(rename = "MetaInfoMap")]
    maps: Vec<MetaMap>,
}

#[derive(Deserialize, Serialize)]
struct MetaMap {
    #[serde(rename(deserialize = "@typ"))]
    typ: String,
    #[serde(rename(deserialize = "Meta"), default)]
    meta: Vec<Meta>,
    #[serde(rename(deserialize = "MetaList"), default)]
    meta_lists: Vec<MetaList>,
}

#[derive(Deserialize, Serialize)]
struct Meta {
    #[serde(rename(deserialize = "@name"))]
    name: String,
    #[serde(
        rename(deserialize = "@coding"),
        skip_serializing_if = "Option::is_none"
    )]
    coding: Option<String>,
    #[serde(rename(deserialize = "$text"), default)]
    value: String,
}

#[derive(Deserialize, Serialize)]
struct MetaList {
    #[serde(rename(deserialize = "@typ"))]
    typ: String,
    #[serde(rename(deserialize = "MetaListElement"))]
    elements: Vec<MetaListElement>,
}

#[derive(Deserialize, Serialize)]
struct MetaListElement {
    #[serde(rename(deserialize = "@name"))]
    name: Option<String>,
    #[serde(rename(deserialize = "$text"), default)]
    value: String,
}

#[derive(Deserialize)]
struct WireDatenpaket {
    #[serde(rename = "ReferenzId")]
    objektid: Option<String>,
    #[serde(rename = "Encoding")]
    encoding: Option<String>,
    #[serde(rename = "Groesse")]
    groesse: Option<u64>,
}

#[derive(Deserialize)]
struct Attachments {
    #[serde(rename = "Anhang")]
    anhaenge: Vec<WireAnhang>,
}

#[derive(Deserialize)]
struct WireAnhang {
    #[serde(rename = "DateinameMitErweiterung")]
    dateiname: Option<String>,
    #[serde(rename = "Dateibezeichnung")]
    dateibezeichnung: String,
    #[serde(rename = "Dateityp")]
    dateityp: String,
    #[serde(rename = "DateiReferenzId")]
    objektid: String,
    #[serde(rename = "DateiGroesse")]
    groesse: u64,
    #[serde(rename = "MetadatenAnhang")]
    meta: Option<AttachmentMetadata>,
}

#[derive(Deserialize)]
struct AttachmentMetadata {
    #[serde(rename = "MetadatumAnhang", default)]
    entries: Vec<AttachmentMeta>,
}

#[derive(Deserialize, Serialize)]
struct AttachmentMeta {
    #[serde(rename(deserialize = "SchluesselAnhang"))]
    name: Option<String>,
    #[serde(rename(deserialize = "WertAnhang"))]
    value: Option<String>,
}

#[derive(Serialize)]
pub(super) struct Nachricht {
    id: String,
    bereitstellungsdatenart: &'static str,
    bereitstellungsticket: Option<String>,
    verfuegbar_bis: Option<String>,
    groesse: u64,
    meta: BTreeMap<String, String>,
    anschreiben: Option<Anschreiben>,
    meta_maps: Vec<MetaMap>,
    datenpaket: Option<Datenpaket>,
    anhaenge: Vec<Anhang>,
}

#[derive(Default, Serialize)]
struct Anschreiben {
    sendername: Option<String>,
    bezeichnung: Option<String>,
    text: Option<String>,
}

#[derive(Serialize)]
struct Datenpaket {
    objektid: String,
    encoding: String,
    groesse: u64,
}

#[derive(Serialize)]
struct Anhang {
    position: usize,
    dateiname: Option<String>,
    dateibezeichnung: String,
    dateityp: String,
    objektid: String,
    groesse: u64,
    meta: Vec<AttachmentMeta>,
}

impl WireNachricht {
    fn into_nachricht(self, datenart: &'static str) -> Result<Nachricht, Failure> {
        let invalid = || Operation::Anfrage.invalid_reply();
        let meta: BTreeMap<_, _> = self
            .meta
            .entries
            .into_iter()
            .map(|m| (m.name, m.value))
            .collect();
        let verfuegbar_bis = meta
            .get("verfuegbarbis")
            .map(|raw| {
                Date::parse(raw, format_description!("[year][month][day]"))
                    .map(|date| date.to_string())
                    .map_err(|_| invalid())
            })
            .transpose()?;
        let mut meta_maps = self.meta.maps.map_or_else(Vec::new, |maps| maps.maps);
        let anschreiben = meta_maps
            .iter()
            .position(|map| map.typ == "ANSCHREIBEN")
            .map(|index| Anschreiben::decode(meta_maps.remove(index)))
            .transpose()?;
        let datenpaket = match self.datenpaket {
            None
            | Some(WireDatenpaket {
                objektid: None,
                encoding: None,
                groesse: None,
            }) => None,
            Some(WireDatenpaket {
                objektid: Some(objektid),
                encoding: Some(encoding),
                groesse: Some(groesse),
            }) => Some(Datenpaket {
                objektid,
                encoding,
                groesse,
            }),
            _ => return Err(invalid()),
        };
        let anhaenge = self.anhaenge.map_or_else(Vec::new, |attachments| {
            attachments
                .anhaenge
                .into_iter()
                .enumerate()
                .map(|(index, a)| Anhang {
                    position: index + 1,
                    dateiname: a.dateiname,
                    dateibezeichnung: a.dateibezeichnung,
                    dateityp: a.dateityp,
                    objektid: a.objektid,
                    groesse: a.groesse,
                    meta: a.meta.map_or_else(Vec::new, |meta| meta.entries),
                })
                .collect()
        });
        Ok(Nachricht {
            id: self.id,
            bereitstellungsdatenart: datenart,
            bereitstellungsticket: self.bereitstellungsticket,
            verfuegbar_bis,
            groesse: self.groesse,
            meta,
            anschreiben,
            meta_maps,
            datenpaket,
            anhaenge,
        })
    }
}

impl Anschreiben {
    fn decode(map: MetaMap) -> Result<Self, Failure> {
        let mut anschreiben = Self::default();
        for entry in map.meta {
            match entry.name.as_str() {
                "sendername" => anschreiben.sendername = Some(entry.value),
                "bezeichnungnachricht" => anschreiben.bezeichnung = Some(entry.value),
                "textnachricht" => {
                    let encoded = if entry.value.bytes().any(|b| b.is_ascii_whitespace()) {
                        Cow::Owned(
                            entry
                                .value
                                .bytes()
                                .filter(|b| !b.is_ascii_whitespace())
                                .collect::<Vec<u8>>(),
                        )
                    } else {
                        Cow::Borrowed(entry.value.as_bytes())
                    };
                    let invalid = || {
                        Operation::Anfrage
                            .invalid_reply()
                            .note("Anschreiben text is not base64-encoded UTF-8")
                    };
                    let bytes = STANDARD.decode(encoded).map_err(|_| invalid())?;
                    anschreiben.text = Some(String::from_utf8(bytes).map_err(|_| invalid())?);
                }
                _ => {}
            }
        }
        Ok(anschreiben)
    }
}

impl Nachricht {
    pub(super) fn write_text(&self, mut out: impl Write) -> io::Result<()> {
        write!(
            out,
            "{} {}",
            self.id.escape_debug(),
            self.bereitstellungsdatenart.escape_debug()
        )?;
        if let Some(date) = &self.verfuegbar_bis {
            write!(out, " (available until {date})")?;
        }
        if let Some(subject) = self
            .anschreiben
            .as_ref()
            .and_then(|a| a.bezeichnung.as_deref())
        {
            write!(out, ": {}", subject.escape_debug())?;
        }
        writeln!(out)
    }
}
