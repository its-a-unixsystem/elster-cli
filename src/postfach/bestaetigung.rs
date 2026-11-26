// SPDX-License-Identifier: AGPL-3.0-only
// See NOTICE for the ERiC and Otto additional permission.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{AuthenticatedEric, Failure, Operation, Reply, TraceEvent, envelope};

const BLOCK_SIZE: usize = 100;
const ERIC_NUTZDATENHEADER_ERROR: i32 = 610101292;

#[derive(Serialize, JsonSchema)]
pub(super) struct Confirmation {
    pub(super) id: String,
    pub(super) bestaetigt: bool,
    pub(super) rueckgabe: Option<ConfirmationCode>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_failure"
    )]
    #[schemars(with = "Option<Failure>", required)]
    pub(super) fehler: Option<Rc<Failure>>,
    #[serde(skip)]
    rejection: Option<Rc<Failure>>,
}

fn serialize_failure<S: serde::Serializer>(
    failure: &Option<Rc<Failure>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    failure.as_deref().serialize(serializer)
}

#[derive(Clone, Deserialize, Serialize, JsonSchema)]
pub(super) struct ConfirmationCode {
    #[serde(rename(deserialize = "Code"))]
    pub(super) code: i32,
    #[serde(rename(deserialize = "Text"))]
    pub(super) text: Option<String>,
}

impl Confirmation {
    pub(super) fn into_failure(self) -> Option<Failure> {
        if self.bestaetigt {
            return None;
        }
        if let Some(fehler) = self.fehler {
            return Some(Rc::unwrap_or_clone(fehler));
        }
        let result = self.rueckgabe?;
        let failure = self
            .rejection
            .map_or_else(|| Operation::Bestaetigung.rejected(), Rc::unwrap_or_clone);
        Some(failure.cause("elster", Some(result.code), result.text))
    }
}

pub(super) fn execute(
    ids: &[&str],
    eric: &impl AuthenticatedEric,
    hersteller_id: &str,
    test: bool,
    trace: &mut Option<Vec<TraceEvent>>,
) -> Vec<Confirmation> {
    let mut confirmations = Vec::<Confirmation>::with_capacity(ids.len());
    let operation = Operation::Bestaetigung;
    for ids in ids.chunks(BLOCK_SIZE) {
        let request = envelope(operation, hersteller_id, test, |xml| {
            xml.push_str("<PostfachBestaetigung>\n<Bereitstellungen>\n");
            for id in ids {
                let id = quick_xml::escape::escape(*id);
                writeln!(xml, "<Bereitstellung id=\"{id}\"/>").unwrap();
            }
            xml.push_str("</Bereitstellungen>\n</PostfachBestaetigung>\n");
        });
        let response = eric.send(&request, c"PostfachBestaetigung_32", trace, read_block);
        let (mut replies, rejection) = match response {
            Ok((replies, rejection)) => (replies, rejection.map(Rc::new)),
            Err(failure) => (HashMap::new(), Some(Rc::new(failure))),
        };
        let had_replies = !replies.is_empty();
        let start = confirmations.len();
        let mut missing = None;
        for &id in ids {
            let (id, rueckgabe) = replies.remove_entry(id).map_or_else(
                || {
                    // ELSTER may return a single result for a repeated ID.
                    let repeated = had_replies
                        .then(|| confirmations[start..].iter().find(|entry| entry.id == id))
                        .flatten()
                        .and_then(|entry| entry.rueckgabe.clone());
                    (id.to_owned(), repeated)
                },
                |(id, result)| (id, Some(result)),
            );
            let bestaetigt = rueckgabe.as_ref().is_some_and(|result| result.code == 0);
            let fehler = rueckgabe.is_none().then(|| {
                Rc::clone(missing.get_or_insert_with(|| {
                    rejection
                        .clone()
                        .unwrap_or_else(|| Rc::new(operation.invalid_reply()))
                }))
            });
            confirmations.push(Confirmation {
                id,
                bestaetigt,
                rejection: if bestaetigt || fehler.is_some() {
                    None
                } else {
                    rejection.clone()
                },
                rueckgabe,
                fehler,
            });
        }
    }
    confirmations
}

type BlockReplies = (HashMap<String, ConfirmationCode>, Option<Failure>);

fn read_block(code: i32, xml: &str, mut text: Option<String>) -> Result<BlockReplies, Failure> {
    let operation = Operation::Bestaetigung;
    // A NutzdatenHeader error does not roll back IDs already confirmed in this block.
    if !matches!(code, 0 | ERIC_NUTZDATENHEADER_ERROR) {
        return Err(operation
            .decode_rejection(xml, code, &mut text)
            .unwrap_or_else(|| operation.eric_failure(code, text)));
    }
    let reply: Reply<Payload> = quick_xml::de::from_str(xml).map_err(|_| {
        operation
            .decode_rejection(xml, code, &mut text)
            .unwrap_or_else(|| {
                if code == 0 {
                    operation.invalid_reply()
                } else {
                    operation.eric_failure(code, text.take())
                }
            })
    })?;
    let rejection = reply
        .rejection(operation, code, &mut text)
        .or_else(|| (code != 0).then(|| operation.eric_failure(code, text)));
    let replies = reply
        .data
        .into_iter()
        .flat_map(|data| data.blocks)
        .filter_map(|block| block.payload?.collection.answer)
        .flat_map(|answer| answer.entries.rows)
        .map(|row| (row.id, row.result))
        .collect();
    Ok((replies, rejection))
}

#[derive(Deserialize)]
struct Payload {
    #[serde(rename = "Datenabholung")]
    collection: Collection,
}

#[derive(Deserialize)]
struct Collection {
    #[serde(rename = "PostfachBestaetigungAntwort")]
    answer: Option<Answer>,
}

#[derive(Deserialize)]
struct Answer {
    #[serde(rename = "Bereitstellungen")]
    entries: Entries,
}

#[derive(Deserialize)]
struct Entries {
    #[serde(rename = "Bereitstellung", default)]
    rows: Vec<ReplyRow>,
}

#[derive(Deserialize)]
struct ReplyRow {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "Rueckgabe")]
    result: ConfirmationCode,
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::ffi::CStr;

    use super::*;

    const FIRST: &str = "eh036000000000000000000000000001";
    const SECOND: &str = "eh036000000000000000000000000002";

    #[derive(Default)]
    struct FakeEric<'a> {
        responses: RefCell<VecDeque<(i32, &'a str)>>,
        requests: RefCell<Vec<String>>,
    }

    impl AuthenticatedEric for FakeEric<'_> {
        fn inspect<T>(
            &self,
            _: &mut Option<Vec<TraceEvent>>,
            _: impl FnOnce(&str) -> Result<T, Failure>,
        ) -> Result<T, Failure> {
            panic!("certificate was already inspected")
        }

        fn send<T>(
            &self,
            xml: &CStr,
            version: &CStr,
            _: &mut Option<Vec<TraceEvent>>,
            read: impl FnOnce(i32, &str, Option<String>) -> Result<T, Failure>,
        ) -> Result<T, Failure> {
            assert_eq!(version, c"PostfachBestaetigung_32");
            self.requests
                .borrow_mut()
                .push(xml.to_str().unwrap().to_owned());
            let (code, xml) = self
                .responses
                .borrow_mut()
                .pop_front()
                .expect("unexpected call");
            read(code, xml, None)
        }
    }

    #[test]
    fn reports_both_ids_in_requested_order_after_nutzdatenheader_error() {
        let reply = format!(
            r#"<Elster><TransferHeader/><DatenTeil><Nutzdatenblock>
<NutzdatenHeader><RC><Rueckgabe><Code>190017112</Code><Text>Der Empfang von mindestens einer der Daten wurde nicht erfolgreich bestätigt.</Text></Rueckgabe></RC></NutzdatenHeader>
<Nutzdaten><Datenabholung><PostfachBestaetigungAntwort><Bereitstellungen>
<Bereitstellung id="{SECOND}"><Rueckgabe><Code>190015302</Code><Text>Keine Daten vorhanden.</Text></Rueckgabe></Bereitstellung>
<Bereitstellung id="{FIRST}"><Rueckgabe><Code>0</Code><Text>Der Empfang der Daten wurde erfolgreich bestaetigt.</Text></Rueckgabe></Bereitstellung>
</Bereitstellungen></PostfachBestaetigungAntwort></Datenabholung></Nutzdaten>
</Nutzdatenblock></DatenTeil></Elster>"#,
        );
        let eric = FakeEric::default();
        eric.responses.borrow_mut().push_back((610101292, &reply));
        let result = execute(&[FIRST, SECOND], &eric, "12345", true, &mut None);
        let golden = include_str!("../../tests/fixtures/requests/postfach-bestaetigung.xml")
            .replace(
                "<ProduktVersion>0.1.0",
                concat!("<ProduktVersion>", env!("CARGO_PKG_VERSION")),
            );
        assert_eq!(&*eric.requests.borrow(), &[golden]);
        assert_eq!(
            serde_json::to_value(&result).unwrap(),
            serde_json::json!([
                {"id": FIRST, "bestaetigt": true, "rueckgabe": {"code": 0, "text": "Der Empfang der Daten wurde erfolgreich bestaetigt."}},
                {"id": SECOND, "bestaetigt": false, "rueckgabe": {"code": 190015302, "text": "Keine Daten vorhanden."}}
            ]),
        );
        let failure = result.into_iter().nth(1).unwrap().into_failure().unwrap();
        assert_eq!(failure.exit, 5);
        let error = serde_json::to_value(failure).unwrap();
        assert_eq!(error["cause"][0]["code"], 610101292);
        assert_eq!(error["cause"][1]["code"], 190017112);
        assert_eq!(error["cause"][2]["code"], 190015302);
    }

    #[test]
    fn continues_after_a_block_failure_without_inventing_per_id_return_codes() {
        let input: Vec<_> = (0..=100).map(|i| format!("eh036{i:027}")).collect();
        let ids: Vec<_> = input.iter().map(String::as_str).collect();
        let last = ids[100];
        let reply = format!(
            r#"<Elster><TransferHeader/><DatenTeil><Nutzdatenblock><Nutzdaten><Datenabholung>
<PostfachBestaetigungAntwort><Bereitstellungen>
<Bereitstellung id="{last}"><Rueckgabe><Code>0</Code><Text>OK</Text></Rueckgabe></Bereitstellung>
</Bereitstellungen></PostfachBestaetigungAntwort>
</Datenabholung></Nutzdaten></Nutzdatenblock></DatenTeil></Elster>"#,
        );
        let eric = FakeEric::default();
        eric.responses
            .borrow_mut()
            .extend([(610101200, ""), (0, &reply)]);
        let result = execute(&ids, &eric, "12345", false, &mut None);
        let requests = eric.requests.borrow();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].matches("<Bereitstellung id=").count(), 100);
        assert_eq!(requests[1].matches("<Bereitstellung id=").count(), 1);
        assert!(requests[1].contains(&format!("<Bereitstellung id=\"{last}\"/>")));
        assert_eq!(
            result.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ids
        );
        for row in &result[..100] {
            assert!(!row.bestaetigt);
            assert!(row.rueckgabe.is_none());
            assert_eq!(row.fehler.as_ref().unwrap().exit, 6);
        }
        assert!(result[100].bestaetigt);
        assert!(result[100].fehler.is_none());
    }
}
