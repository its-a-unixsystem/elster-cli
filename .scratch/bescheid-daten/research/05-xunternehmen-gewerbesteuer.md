# 05 - XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid

Ticket: `issues/05-xunternehmen-gewerbesteuer.md`. Terms follow `GLOSSARY.md`. Builds on `01-bescheiddaten-formats.md` (the `Gewerbesteuerbescheid` row and section 7); what is settled there is not repeated.

Documents were downloaded into scratch directories outside the repo and treated as untrusted data (PDF text via `pdftotext`, XML/PDF structure via `qpdf` and `python3 -I`). Nothing from them is copied here beyond short facts; no personal data is involved (the Musterbescheide are invented). Page numbers are the PDF's own.

## Sources

- **XR**: XRepository entry `urn:xoev-de:xunternehmen:standard:gewerbesteuer` and its versions 1.4, 1.5, 1.6 (`https://www.xrepository.de/api/xrepository/urn:xoev-de:xunternehmen:standard:gewerbesteuer[_1.x]`, the XML metadata behind `https://www.xrepository.de/details/...`). Files per version: Spezifikationsdokument, XSD zip, Praxishandbuch, Musterbescheide zip (`...:dokument:<Name>:datei:<file>/daten` and `...:xmlschema:datei:<file>/daten`).
- **SPEC 1.6**: `xunternehmen-gewerbesteuer-1.6.pdf`, "XUnternehmen-Fachmodul Gewerbesteuer, Version 1.6 (Endfassung), Fassung vom 29.10.2026" (also read the 1.4 text for chapters later shortened). **XSD 1.4/1.5/1.6** and the 1.3 XSD from DGB-Web (`xsd_v1.3.zip`).
- **MUSTER**: `xunternehmen-gewerbesteuer-1.6-musterbescheide.zip`: six invented Bescheide/Berechnungen as PDF/A-3 plus the embedded XML as separate files, and a `readme.md`.
- **PH**: Praxishandbuch V1.4 to V1.6 (same zip family).
- **REL**: `Release-Konzept XUnternehmen.Gewerbesteuer v1.0` (30.10.2025, XR documentation of the standard); **BK**: `Betriebskonzept XUnternehmen v1.0` (XR).
- **LH**: `Lastenheft Digitaler Gewerbesteuerbescheid v2.1` (]init[ AG, 29.07.2026, DGB-Web `/hkr-hersteller`).
- **FAQ-GewSt**: `20240726_FAQ_Digitaler_Gewerbesteuerbescheid.pdf` (v2.1, 26.07.2024, `download.elster.de`).
- **SSb-BEM**: `ElsterBereitstellung-SSB-V5.0.8.pdf` (public ERiC 44.3.6.0 documentation zip); **SSb-DA**: `SSb ElsterDatenabholung - Extern V32.0.1.pdf` (`docs/API/`).
- **Crates**: crates.io API metadata and the published `.crate` sources of `lopdf` 0.47.0, `pdf` 0.10.0, `pdf_oxide` 0.3.78, `oxidize-pdf` 5.6.1, `pdfium-render` 0.9.5, `mupdf` 0.8.0, `pdf-extract` 0.12.1.

Claims marked **[inference]** are my reading. **[not found]** means the sources are silent.

## 1. Result in one paragraph

A Kommune's Gewerbesteuerbescheid reaches the Taxpayer through ELSTER as a PDF Anhang (no Datenpaket, see 01). The PDF is a PDF/A-3 whose catalog carries one attached XML file named `GEWST-BS-<AGS>-<Steuernummer>-<Bescheiddatum>_<Nachrichten-ID>.xml`. The XML is defined by the XÖV Fachmodul XUnternehmen.Gewerbesteuer (public on xrepository.de; XSD, specification, Praxishandbuch, Musterbescheide; versions 1.4 to 1.6 there, 1.2.x/1.3 earlier on DGB-Web). It carries the key figures the map wants: Messbetrag, Hebesatz, festgesetzte Gewerbesteuer, Vorauszahlungen, Zinsen, Zuschläge, Zahlungsaufforderung with Fälligkeit, and Bescheidkennzeichnung codes for Vorläufigkeit/Vorbehalt der Nachprüfung. The XML is explicitly an auxiliary dataset without Bescheidcharakter; the PDF governs. Embedding is required of HKR software by the spec and the Lastenheft, but I found no ELSTER-side check. The XML is not backward compatible between versions, and the version is in the namespace. A Rust reader can extract it with `lopdf` in about 40 lines without rendering (verified on all six samples). The files carry no licence statement; the published position is "no licence fees".

## 2. The schema, its versions, where it lives

- **Standard**: "XUnternehmen-Fachmodul Gewerbesteuer", short name `XUnternehmen.Gewerbesteuer`, technical name `gewerbesteuer`, `kennung` `urn:xoev-de:xunternehmen:standard:gewerbesteuer`, description "Datenformat für die digitale Übermittlung von Gewerbesteuerbescheiden und -berechnungen" (XR). Herausgeber: Bundesministerium für Wirtschaft und Energie; Betreiber: KoSIT and d-NRW AöR (SPEC cover). Entry created 2025-02-13; DGB-Web says that since version 1.4 spec, data model, Musterbescheide and Praxishandbuch are provided there (`/hkr-hersteller`).
- **Versions on XR** (XR metadata, SPEC change history, REL):

| Version | Date | Namespace | Productive from (XR `datumGueltigkeitAb`) | Status on XR |
|---|---|---|---|---|
| 1.2.1 to 1.3 (pre-XR, DGB-Web archive) | 13.01.2023 to 31.07.2024 | `http://www.esteuer.de/gewerbesteuer-bescheide/<v>/` (from 1.2.3) | n/a | not on XR |
| 1.4 | 30.10.2025 | `urn:xoev-de:xunternehmen:standard:gewerbesteuer_1.4` | 2026-09-01 | Endfassung, certified |
| 1.5 | published 2026-04-22 (spec history says 01.05.2026) | `..._1.5` | 2027-03-01 | Endfassung, certification requested |
| 1.6 | 30.10.2026 (spec dated 29.10.2026) | `..._1.6` | 2027-09-01 | Endfassung, certification requested |

  Release rhythm (REL 2.1, 2.2): publication 1 May and 1 November; productive 10 months later (1 March / 1 September of the next year). So today (2026-11) 1.4 is the first XR-era version in force; 1.3 and earlier are what Kommunen on the older HKR releases sent. **[inference]** which version a given real Bescheid uses cannot be known in advance; the root-element namespace tells.
- **Not backward compatible** (REL 2.3, p. 7): "Die XML-Nachrichten des Fachmoduls sind nicht abwärtskompatibel"; the operators recommend consumers support several versions during a transition. A transition period for old versions is explicitly undecided (REL 1.3, 2.2). Consequence: a reader needs per-version handling, keyed on namespace and on `nachrichtenkopf/versionStandard` (fixed value per schema version, SPEC II.3.2.1).
- **What changed** (SPEC 1.6 change history, pp. 3-9, and my own comparison of the XSD element names):
  - 1.3 to 1.4: single XSD file `xunternehmen-gewerbesteuer.xsd` (was four), new namespace, DIN 91379 string type, `kommune/ags`, `artVertretung`, address choices (`strukturiert`/`freitext`), file-name convention (GEWST-11), Kurzbescheid, partially masked IBAN allowed. My name-set diff 1.3 vs 1.4: removed `angabenAdressfeld`, `nameUndAnschrift`; added `art`, `artVertretung`, `freitext`, `strukturiert`, `gwstKurzbescheid`; renamed `VorauszahlungBerechnung` to `vorauszahlungBerechnung`.
  - 1.4 to 1.5: `messbetragAlt`/`hebesatzAlt` in Vorauszahlungen, PDF/A-3a recommended, new Musterbescheide.
  - 1.5 to 1.6: `kommune/nameMitgliedsgemeinde`, `summeJeZinsart`, spec text shortened (background chapters moved to a separate "Hintergrundwissen" document, not updated).
- **Messages** (root elements, SPEC I.2): `bescheide.gewerbesteuer.0001` (Veranlagung), `bescheide.zinsen.0002`, `bescheide.vorauszahlung.0003`, `bescheide.gewerbesteuer.generisch.0010` (any mix of Veranlagung, Zinsen, Vorauszahlungen, Zuschläge, Kurzbescheid), `berechnung.gewerbesteuer.0021` (Gewerbesteuerberechnung for insolvent Unternehmen; not a Verwaltungsakt).
- **Validation**: all six 1.6 Musterbescheide XML files validate against the shipped 1.6 XSD with `xmllint` (24 XSD files including the imported XUnternehmen Basismodul 1.2 and XÖV library schemas; the main file is 172 KB).

## 3. Key figures (SPEC 1.6, XSD)

Element names are as in the XSD; sections and pages are SPEC 1.6.

| Wanted figure | Where in the XML | Notes |
|---|---|---|
| Messbetrag | `gwstVeranlagung/messbetrag` (1; `AngabeBetrag`), `messbetragAlt` (0..1); in `gwstVorauszahlungen` optional `messbetrag`/`messbetragAlt` (II.3.2.28, II.3.2.30, pp. 64-67) | Absent in 0002-only Bescheide (Zinsen) |
| Hebesatz | `hebesatz` (1 in Veranlagung, 0..1 in Vorauszahlung), `hebesatzAlt`; `AngabeHebesatz`: integer percent 200 to 10000 (II.2.3) | |
| festgesetzte Gewerbesteuer | `festsetzungsAngaben/festsetzungAktuell` ("entspricht der Steuerfestsetzung des Gewerbesteuerbescheides"), `festsetzungBisher` (0..1), `erhebungszeitraum` (`bezugsjahr`, optional `beginn`, `ende`, `quartal`), `abgabeart` code, `festsetzungsart` code (II.3.2.18, pp. 55-56) | One `Festsetzung` per Erhebungszeitraum; same type base for Zinsen, Vorauszahlungen, Zuschläge |
| Abrechnung / Zahlung | `festsetzungsAngaben/zahlungsaufforderungsAngaben`: `SOLLoderIST`, `geleisteteZahlung[]` (`betragBereitsGeleisteteZahlungen`, signed, negative in favour of the Unternehmen), `zahlungsaufforderung[]` (`betrag`, negative = Erstattung, already net of Zahlungen) (II.3.2.19 to II.3.2.23, pp. 56-59) | The net amount to pay is the Zahlungsaufforderung, not festsetzungAktuell |
| Fälligkeit | `zahlungsaufforderung/faelligkeit`: choice `zahlungBis` (xs:date) or `regelmaessig` (`zahlungAb`, `wiederholung` code, e.g. quarterly) (II.3.2.21, II.3.2.22, p. 58 f.) | Optional for Erstattungen |
| Vorauszahlungen | `gwstVorauszahlungen[]` (0003, 0010): each element is one Festsetzung (usually one quarter), `abgabeart` `VZ`/`VN`/`JV`/`JB`; `JV` = Jahresfestsetzung der Vorauszahlungen as a separate block (SPEC history 1.2.3; II.3.2.30, p. 66) | Spec recommends separate Festsetzungen, not summed |
| Zinsen, Zuschläge | `gwstZinsen[]` (`zinsangaben[]`, `zinsFestsetzungUngerundet`, 1.6: `summeJeZinsart`), `gwstVerspaetungszuschlag[]`, `gwstSaeumniszuschlag[]` | Zinsangaben carry `festsetzungsstatus` OFFEN/GETILGT/INFO/UNBEKANNT, which decides whether the amount adds, subtracts or is information only (II.3.2.24, p. 60) |
| Vorläufigkeit, Vorbehalt der Nachprüfung | `bescheidKennzeichnung[]`: `kennzeichnungCode` (codelist `gewerbesteuer.bescheidkennzeichnung`, version 2) or `kennzeichnungText` (free text) (II.3.2.5, p. 45) | Codelist (III.D.3.3, pp. 94-97): 11/12/16/18 vorläufig (§ 165 AO), 13 Vorbehalt der Nachprüfung (§ 164 Abs. 1), 15 Aussetzung, 20 to 29 Änderung nach Vorbehalt/Vorläufigkeit, 31 to 46 other Änderungen, 50/71/77 GewStG-specific, 80 geschätzt, 00 none. Its description says it expresses "Hinweis auf Vorläufigkeit gem. Abgabenordnung". This is the only place in any Bescheiddaten format found so far where Vorbehalt/Vorläufigkeit is a defined value |
| Endgültigkeit, Aufhebung | `gwstKurzbescheid` (0..1, 0010 only): `art` code, date and Erhebungszeitraum of the Bescheid concerned (II.3.2.31) | |
| Header | `nachrichtenkopf` (`nachrichtenID`, `zeitstempel`, `nameStandard`="GewSt", `versionStandard`, `fachverfahren`); `briefkopf` (`bescheidDatum`, `stichtag`, `steuernummerBund` 13 digits, `steuernummerFA`, `buchungsnummer`, `behoerdenZeichen`); `kommune` (`nameBehoerde`, `ags`, 1.6 `nameMitgliedsgemeinde`) (II.3.2.1, .4, .6) | `transferticket` in `festsetzungsAngaben` is the ELSTER Transferticket of the Erklärung, "dient der Zuordnung ... zur entsprechenden Steuererklärung" (p. 56): a join key to a return filed through ERiC |
| Personal data | `inhaltsAdressat`, `bekanntgabeAdressat[]`, `bescheidEmpfaenger`, `bankverbindung` (IBAN, may be masked with `*`), `mandant`, free text `rechtsbehelfsBelehrung`, `erlaeuterungsText[]` | Real Bescheide contain all of it. Fixtures must be invented |

Typing facts for JSON (II.2): amounts are `xs:decimal` with at most two fraction digits and a dot separator (the samples write `4800`, `1200` without decimals); Hebesatz is an integer percent; dates are `xs:date`; Erstattungen are negative. Fits the v1 precedent of amounts as decimal strings.

## 4. Mandatory or optional embedding

- **Spec**: II.4 (p. 79): the specification "erfordert", that Bescheide and Berechnungen are encoded as hybrid PDF/A-3 (ISO 19005-3:2012) with the XML as file attachment; PDF/A-3a conformance level recommended (GEWST-27, since 1.5). I.1.3 (pp. 18-19) is softer: embedding is "empfohlen"; sending both formats at once or embedding one in the other are both possible. For the Berechnung (not a Verwaltungsakt) the hybrid format is only recommended.
- **Lastenheft** (LH 3.2.1): the HKR system "muss" generate the Bescheid per the Spezifikation as PDF/A-3 with an embedded XML that matches the PDF, and must validate the XML against the schema before Bekanntgabe. This binds HKR vendors as a requirement document; it is not an ELSTER check.
- **ELSTER side**: SSb-BEM section 8 and 4.4.3/6: `Gewerbesteuerbescheid` allows no `Nutzdaten`/Datenpaket and requires at least one Anhang with `Dateityp` (e.g. `application/pdf`), a Dateibezeichnung and Base64 `Dateiinhalt`; a virus check runs on the Anhang. I found no statement that ELSTER checks PDF/A conformance, the presence of the embedded XML, or the schema **[not found]**. A Kommune that uses its own system or an older HKR release may deliver a plain PDF **[inference]**.
- **Participation**: not every Kommune sends through ELSTER-Transfer; connection is voluntary and gradual (FAQ-GewSt Q6; 01 notes Q16). FAQ-GewSt Q6: HKR software "muss in der Lage sein", a PDF with XML attachment per the Standardspezifikation to create; software of Steuerbüros "muss in der Lage sein ... einzulesen".
- **Bottom line**: expect XML in most Bescheide from Kommunen on the standard, not in all. The CLI must handle "PDF without attachment" as a normal outcome and report it as such, not as an error **[inference]**.

## 5. How the XML sits inside the PDF

From the six Musterbescheide (qpdf structure dump, and the `lopdf` probe below):

- XMP metadata: `pdfaid:part` 3, `pdfaid:conformance` A (in the 2025-09 sample).
- The attachment is a normal `/Filespec` with `/F` and `/UF` = the XML file name (the naming scheme of SPEC II.4: `GEWST-BS-` for Bescheid, `GEWST-BR-` for Berechnung, then AGS, 13-digit Steuernummer, date `YYYY-MM-DD`, `_`, Nachrichten-ID, `.xml`; PDF and XML share the name apart from the extension), `/EF << /F stream >>`, stream `/Subtype /text/xml`, `/Filter /FlateDecode`, `/Params` with `/Size`, `/CheckSum`.
- It is registered in the catalog's `/Names /EmbeddedFiles` name tree, with exactly one entry in every sample. The XML bytes there are identical (SHA-256) to the separate XML file shipped beside each PDF.
- **`AFRelationship` is `/Unspecified`** in all six samples, not `/Data` or `/Alternative`. The specification text names no value **[not found]**. So a reader must not filter on `AFRelationship`.
- **The catalog's `/AF` array is unreliable in the samples**: it lists 3 to 7 filespecs, all with the same file name and `/Unspecified`, of which only the last matches the shipped XML; the others are stale earlier versions (different sizes). The files have two `%%EOF` markers, i.e. an incremental update **[inference: leftovers of the producing tool's template editing; real HKR output may differ]**. Consequence: take the attachment from the `/Names /EmbeddedFiles` tree, select by file name pattern and by root namespace, and treat several candidates as a fault to report, not to resolve silently.
- Encoding is UTF-8 with an XML declaration in all samples.
- The PDF is the Bescheid; the XML is derived. SPEC I.1.3 (a) to (c): the human-readable PDF is "maßgeblich", the embedded dataset "hat keinerlei Bescheidcharakter" and is an "Hilfsdatensatz für die elektronische Bearbeitung"; a consumer who uses the XML "sollte einen Abgleich zwischen PDF- und XML-Dokument vornehmen", responsibility lies with the recipient.

## 6. What a Rust reader needs, without rendering

Needs: parse the xref/object structure (including incremental updates and object streams), walk `/Root /Names /EmbeddedFiles` (name tree, possibly with `/Kids`), read `/EF /F` stream, inflate with a size cap. No page content, fonts or images.

| Crate | Version, date | Licence | Embedded-file support (from source) | Notes |
|---|---|---|---|---|
| `lopdf` | 0.47.0, 2026-11-04; first release 2016; 57 versions; MSRV 1.88; ~15 M recent downloads | MIT | No ready helper. Generic access suffices: `Document::load_mem`, `catalog()`, `dereference()`, `Stream::decompressed_content_with_limit(max)`; `LoadOptions` has `max_decompressed_size` | **Verified**: a 40-line probe (`default-features = false`, 82 lines of `cargo tree`) extracted the correct XML from all six samples, printed `AFRelationship` and `/Subtype`. Pure Rust, no rendering. Best fit |
| `pdf_oxide` | 0.3.78, 2026-09-08; first release 2025-11 | MIT OR Apache-2.0 | `PdfDocument::extract_embedded_files()` returns `(filename, bytes)` by walking `/Names /EmbeddedFiles` incl. `/Kids` (depth-limited) | Ready-made but large: 78 dependency tables in its manifest (ML, OCR, barcode features among them), 23 MB source, young crate. Not run |
| `pdf` (pdf-rs) | 0.10.0, 2026-03-02; first release 2015 | MIT | Typed `NameTree<FileSpec>` with `EF` only; `FileSpec` has no `F`/`UF`/`AFRelationship` fields, so the file name is not exposed; `Stream::data()` decodes | Thin for this; would need a patch or raw access. Not run |
| `oxidize-pdf` | 5.6.1, 2026-11-16; first release 2025-07 | MIT | Writer and PDF/A validator code only; no extraction API found | Not suitable |
| `pdfium-render` | 0.9.5, 2026-11-01 | MIT OR Apache-2.0 | `PdfDocument::attachments()` | Needs the native Pdfium library (dynamic binding or the `static` feature); heavy for a single binary and a renderer in the dependency |
| `mupdf` | 0.8.0 | **AGPL-3.0** | not checked | Licence excludes it for a permissive binary; renderer anyway |
| `pdf-extract` | 0.12.1 | MIT | text extraction only | Contradicts the map ("never PDF text extraction") |

**[inference]** Recommended: `lopdf` with `default-features = false`, loading from bytes, size cap on both the PDF and the inflated XML, name tree walk limited in depth. Untrusted-input hygiene: the PDF comes from a Kommune through ELSTER (virus-checked there), but treat it as untrusted. The XML parser should be a small non-validating one with DTD/entity expansion off (XML from the PDF is also external data); no XSD validation is needed at runtime.

## 7. Licence of the specification and schemas

- **No licence statement found** on any artefact I could read: not in the XR metadata (no licence element, only status, owner, documents), not in SPEC 1.4/1.6, PH, REL, BK, the XSD files (the XÖV library XSDs contain none either), the DGB-Web pages (`/hkr-hersteller`, `/kommunen/schnittstellen-fuer-kommunen`), the Lastenheft, or xunternehmen.de/downloadbereich.
- **What is stated**: SPEC II.4: embedding and reading attachments is supported by libraries "die von Fachverfahrensherstellern lizenzfrei verwendet und eingebunden werden können" (that is about PDF libraries, not the spec). FAQ-GewSt Q8: "Auch die Verwendung der Spezifikation des standardisierten Gewerbesteuerbescheides ist nicht mit Lizenzgebühren verbunden." XRepository help and `docs.xoev.de`: the XRepository is free to use; the CC BY-NC-ND 4.0 notice on `docs.xoev.de` applies to that site's own pages "sofern nicht anders angegeben", not to the standards. The XRepository REST interface has no service-level promise.
- **Publisher**: a federal ministry (Herausgeber), operated by KoSIT/d-NRW for the IT-Planungsrat; the files are published for implementers. The OpenCode group `gitlab.opencode.de/xunternehmen` exists (Kerndatenmodell) but its API returned "Retry later" and I could not read a LICENSE **[not found]**.
- **[inference]** Reading: the intent is free implementation (fee-free, public, built to be implemented by HKR vendors and tax software), but there is no explicit redistribution grant. Implementing a data format by reading it needs no licence. What KIECLI should avoid until a licence is confirmed in writing: vendoring the XSD files, the spec text or the Musterbescheide PDFs (they contain the logo) in the repo or binary. Writing Rust types from the specification and hand-writing invented fixtures needs none of these. The legal question (whether the XSDs are free to redistribute) is for the human; I did not resolve it. The Geschäftsstelle XUnternehmen (d-NRW) is the contact named in XR.
- ERiC's own licence (`lizenz.pdf`) is covered in 01 and does not touch this format.

## 8. How it arrives through ELSTER

Per 01 and SSb-BEM section 8: `DatenartBereitstellung name="Gewerbesteuerbescheid"` appears in `PostfachStatus` like `EPBescheid` (SSb-DA example responses), delivered as Anhang(e), no Datenpaket, with `bescheiddatum`, `diva` ordnungsbegriffe like the EPBescheid family. The sender gives a Base64 `Dateiinhalt`; Otto returns the decrypted, decompressed file, so KIECLI obtains the PDF bytes directly **[inference, as in 01]**. Whether the embedded XML survives the virus-check step (BEM names "Metadefender") unchanged is not stated **[not found]**; the XML-vs-PDF hash comparison needs a real sample (ticket 03).

## 9. Does reading the embedded XML count as "machine-readable data ELSTER delivers"?

For the map to decide; what the sources say:

- For: it is structured XML under a published schema, produced by the issuing Kommune for exactly this purpose ("maschinenlesbarer XML-Datensatz", FAQ-GewSt; "Die Maschinenlesbarkeit ist durch das eingebettete XML-Dokument gewährleistet", SPEC I.1.3). Extracting it is reading a PDF attachment, not text extraction or OCR; the extracted bytes can be written raw next to the PDF like the Datenpaket of other kinds.
- Against / limits: ELSTER does not define, decode or verify it; the XML is not the Bescheid and carries no legal effect; the Kommune is responsible for XML = PDF equality and the recipient for checking it; it can be absent. The parallel in the map's terms: ESB/USB/GSB are "nicht rechtsgültig" Bescheiddaten next to a binding Diva PDF; the XUnternehmen XML is the same relation inside one file.
- **[inference]** A defensible position: count it, as "Bescheiddaten from the Kommune's embedded dataset, PDF governs", and mark that provenance in the JSON (source: embedded file name, schema version, `bescheiddatum`), without claiming equality with the PDF.

## 10. Open points

1. **Real sample.** None of the above was seen on a real Postfach delivery: attachment name, `AFRelationship`, stale `/AF` entries, whether ELSTER's processing leaves the PDF byte-identical, and which schema version real Kommunen send (ticket 03). The `lopdf` probe and the structure notes are based on six invented Musterbescheide.
2. **Licence.** Written confirmation from the XUnternehmen Geschäftsstelle (or a licence file in the OpenCode project) on redistributing the XSD/spec text; until then do not vendor them.
3. **Versions in the field.** 1.3 and earlier (old namespace and address model) vs 1.4+; whether Kommunen send 1.4 from 2026-09-01 or later; how many versions the CLI must support; behaviour on an unknown namespace (report, keep raw). The XR transition period for old versions is undecided (REL).
4. **No XML in the PDF.** Share of Bescheide without attachment; whether another Anhang may carry the XML instead (BEM allows several Anhänge).
5. **Authority of PDF.** Whether KIECLI should flag XML-vs-PDF mismatch is impossible without PDF text; the JSON can only carry the XML as delivered.
6. **Which parts of `Festsetzung` are always present** per message type in practice (e.g. Zinsen-only Bescheide have no Messbetrag); the JSON must tolerate all five root elements.
7. **ESB/GSB vs Kommune Bescheid.** `GSB` (Finanzamt side, BDS-2) and this Kommune Bescheid both concern Gewerbesteuer but different assessments (Messbetrag vs Gewerbesteuer); linking by `transferticket` / `steuernummer` / Erhebungszeitraum is plausible but unverified.
8. **Crates.** `pdf_oxide` extraction not run; `lopdf` MSRV 1.88 against the project's toolchain; maintenance of a single-maintainer crate (lopdf) over the project's lifetime.
9. **Zerlegungsbescheid and Messbetragsmitteilung** (the sibling formats of the same programme, same PDF/A-3 packaging, specs on DGB-Web, delivered by the Finanzverwaltung) are out of this ticket; they do not appear as a `Gewerbesteuerbescheid` Bereitstellungsdatenart in the sources read.
