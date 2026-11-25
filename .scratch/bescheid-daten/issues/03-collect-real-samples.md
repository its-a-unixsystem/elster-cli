# Collect real Bescheid samples

Type: task
Status: open
Blocked by: ../../kiecli/issues/11-test-system-spike.md

## Question

No download shows a real Datenpaket of a Bescheid, and the test system may never deliver one. HITL: with the real Certificate, run an Anfrage over the Bescheid-carrying Bereitstellungsdatenarten and an Abholung with explicit Bestätigung of the Bescheide found, using the spike's ERiC call from [Test-system spike: ERiC end to end](../../kiecli/issues/11-test-system-spike.md) (or the CLI once it exists).

Keep every fetched file outside this repository. Record here only anonymised structure notes per Bereitstellungsdatenart: element names or Bescheidwertnummern present, encoding, inner layering (gzip, base64), schema or version markers, which Anhänge came alongside, and whether a kind delivered only a PDF. Never values, Steuernummern, names or amounts.

Compare against [Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md) and note where reality differs from the documentation. In particular check:

- which BDS-2 version (`0003` or `0004`) arrives, whether Otto's output is plain text or still needs decoding, and how `Sub-Nummer` is used in GSB (the handbook examples on p. 508 don't match its table layout);
- whether ESB/USB/GSB appear at all for this Certificate, given they exist only for returns filed with `bescheid="ja"` by the same account;
- whether any of the undocumented table-9.4 names (`ESSGS`, `ESTGS`, `EURS`, `FEIGS`, `GEWGS`, `GMB*`, `GRSTE`, `VERSVA`, `WPMBR`) show up in `PostfachStatus`;
- for a `Gewerbesteuerbescheid`, whether the PDF/A-3 actually embeds XML, its root element and XUnternehmen namespace version, and whether it sits alone in the `/EmbeddedFiles` name tree as in the Musterbescheide ([XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid](05-xunternehmen-gewerbesteuer.md));
- whether any DIVA Bescheid carries an XML Anhang besides the PDF (as the § 51 InvStG Feststellung does), with its `dateityp`, file name pattern and root namespace;
- which DIVA2 metadata (`vorlaeufigkeitsmerker`, `aenderungsdokument`, `dokumenttyp`) are actually sent.
