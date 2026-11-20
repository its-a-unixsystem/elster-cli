# Redraw the destination after the format finding

Type: grilling
Status: resolved
Blocked by: 05

## Question

The destination promised key figures for every Bereitstellungsdatenart carrying a Bescheid. [Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md) showed that only `ESB`/`USB`/`GSB` (probably `GEWMB`) carry amounts. They do so only for returns filed with `bescheid="ja"` by the same account, and they are not legally binding. Every DIVA kind, including the KSt Bescheid an organisation receives, is PDF plus metadata. `Gewerbesteuerbescheid` embeds XUnternehmen XML in its PDF/A-3.

Does the effort still pay off, and with what scope?

- **Keep, narrowed:** BDS-2 mapping for ESB/USB/GSB, plus typed DIVA2 metadata (`vorlaeufigkeitsmerker`, `aenderungsdokument`, `bescheiddatum`, `dokumenttyp`) for the PDF-only kinds.
- **Gewerbesteuer XML:** decide whether reading the XML embedded in the PDF/A-3 counts as "machine-readable data ELSTER delivers" or as the PDF extraction the map rules out. [XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid](05-xunternehmen-gewerbesteuer.md) supplies what it contains.
- **Metadata only:** drop BDS-2 and only type the Anfrage metadata into v1's `anfrage` output.
- **Stop:** fold nothing into elster-cli; v1's raw Datenpaket and the PDF suffice.

Input from [XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid](05-xunternehmen-gewerbesteuer.md): the XML carries every key figure the destination wants, plus the only defined Vorläufigkeit/Vorbehalt codes. But it "hat keinerlei Bescheidcharakter" (the PDF governs), may be missing, and the spec asks recipients to compare it with the PDF themselves, which elster-cli cannot do without PDF text extraction. If it counts: is "XML as delivered, marked as not the Bescheid, comparison not done" acceptable, and is "PDF without XML" an ordinary result rather than an error?

Input from [Download developer-area Bescheid material](02-developer-area-material.md): BDS-2 does carry Vorbehalt/Vorläufigkeit (`0680000853`), and Bescheiddaten can also arrive as an XML **Anhang** of a DIVA Bescheid (the § 51 InvStG Feststellung does so as standard). Decide whether XML Anhänge in general are in scope (detected by `dateityp`/namespace, mapped only for known schemas, passed through otherwise), or only the named formats.

Also settle whether DIVA2 Verwaltungsakte (Erlass, Stundung, Zinsen, AdV) count as Bescheide for the metadata typing. Update Destination, Notes and Out of scope accordingly.

## Answer

**Keep the effort, narrowed to three sources of Bescheiddaten: the BDS-2 Datenpaket of `ESB`/`USB`/`GSB`, the XUnternehmen.Gewerbesteuer XML embedded in a `Gewerbesteuerbescheid`, and the DIVA2 metadata of every Verwaltungsakt.** Each is a faithful copy, plus named key figures where the format defines them. Destination, Notes and Out of scope of the [map](../map.md) are redrawn to match.

- **BDS-2 kept** for `ESB`/`USB`/`GSB` (and `GEWMB` if confirmed), even though v1.x may rarely see them: they exist only for returns filed with `bescheid="ja"` by the same account, which the spec states plainly. The record-by-record copy is unconditional; named key figures depend on [Ship the Bescheidwertnummer table?](06-ship-number-table.md). BDS-2 becomes the main path once v2 files with `bescheid="ja"`.
- **Gewerbesteuer XML counts as machine-readable data.** A file embedded in the PDF/A-3 is delivered data; reading the printed page (text extraction, OCR) is not. The JSON carries it as delivered, marked as not the Bescheid (the PDF governs) and not compared with the PDF. A Gewerbesteuerbescheid without embedded XML is an ordinary result, not an error. Implemented from the public spec only: no vendored XSDs, spec text or Musterbescheide while the licence is unconfirmed; fixtures are invented.
- **XML Anhänge:** only named schemas are mapped. Any other XML Anhang is detected and reported (`dateityp`, root element, namespace, pointer to the raw file v1 writes), never converted. No generic XML→JSON: it is lossy and its shape is the sender's, which breaks semver'd output.
- **§ 51 InvStG Feststellung XML:** recognised by namespace, not mapped. Out of scope (niche recipients, addable later as a minor version).
- **DIVA2 metadata decoded** next to v1's raw `meta`, never replacing it: Bescheidkennzeichnung from `vorlaeufigkeitsmerker`, `aenderungsdokument`, `bescheiddatum`, `dokumenttyp`. Decoding is keyed on which fields a Postfach-Nachricht carries, not on its Bereitstellungsdatenart, so it covers every DIVA2 Verwaltungsakt (Erlass, Stundung, Zinsen, AdV included).
- **Glossary:** added **Verwaltungsakt** (Bescheid is one kind) and **Bescheidkennzeichnung** (erstmalig endgültig / Vorbehalt der Nachprüfung / vorläufig / Berichtigung, one concept across BDS-2 `0680000853`, DIVA2 `vorlaeufigkeitsmerker` and XUnternehmen `bescheidKennzeichnung`); **Bescheiddaten** now names its three ways of arriving.
- Out of scope: § 51 InvStG mapping, generic XML→JSON, checking the Gewerbesteuer XML against its PDF.
- Graduated from fog: [Command surface for Bescheiddaten](07-command-surface.md) and [JSON shape of Bescheiddaten](08-json-shape.md).
