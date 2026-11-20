# Bescheiddaten formats per Bereitstellungsdatenart

Type: research
Status: resolved
Blocked by:

## Question

For each Bereitstellungsdatenart that carries a Bescheid, what does its Datenpaket contain, and where is the format defined?

Cover: DIVA stage 1 (`DIVABescheid`, `VerbindlicheAuskunft`, `OZGBescheid`, …), DIVA2 (`DivaBescheidESt|KSt|GewSt|USt|FEIN`, `DivaSonstigerVA`), `EPBescheid`, `ESB`/`USB`/`GSB`, `Gewerbesteuerbescheid` and any other Bescheid-carrying kind in SSb Datenabholung table 9.4. Per kind: XML with a schema, the ESB Bescheidwertnummern record format (ERiC-Entwicklerhandbuch ch. 15.4), plain text, or PDF only; encoding and inner layering after Otto (gzip, base64); whether the format is versioned per Veranlagungsjahr; which key figures (festgesetzte Steuer, Abrechnung/Fälligkeit, Vorauszahlungen, Vorbehalt der Nachprüfung, Vorläufigkeit) it carries.

Also: whether ERiC exposes anything for Bescheiddaten (decoding, rendering, a Bescheid comparison) that elster-cli should call instead of parsing itself; which documents sit in the logged-in developer area (e.g. `Bescheidnr_*.zip`) and must be fetched by hand for [Download developer-area Bescheid material](02-developer-area-material.md); and any redistribution terms on them.

Sources: `docs/API/` (Datenabholung SSb and XSD), the public ERiC Jahresdokumentation (`https://download.elster.de/download/eric/eric_44/ERiC-44.3.6.0-Dokumentation.zip`), the ERiC-Entwicklerhandbuch, elster.de public pages, existing open-source clients. Download into a scratch directory, never into the repo.

Output: a table kind → format → definition source → key figures → open points, in `.scratch/bescheid-daten/research/01-bescheiddaten-formats.md`.

## Answer

**Only `ESB`, `USB`, `GSB` (probably `GEWMB`) deliver numeric Bescheiddaten, as a fixed-width BDS-2 text Datenpaket whose number meanings sit in the developer-area `Bescheidnr_*.zip`; every `Diva*`/`DIVA*`/`OZGBescheid`/`EPBescheid` kind is PDF plus metadata only, and `Gewerbesteuerbescheid` is a PDF/A-3 with an embedded XML specified outside ELSTER.**

- BDS-2 record layout (`V`/`D`/`W` records, versions `0003` and `0004`, three Teilbescheide Festsetzung/Abrechnung/VZ) is public in ERiC-Entwicklerhandbuch ch. 15.4; the Bescheidwertnummern that name festgesetzte Steuer, Abrechnung and Vorauszahlungen are not (developer area, ticket 02). Vorbehalt/Vorläufigkeit are not in the BDS-2 header.
- DIVA kinds have no Datenpaket at all (sender specification forbids it); the only machine-readable data is the metadata, including `vorlaeufigkeitsmerker` (0-7, covers Vorbehalt der Nachprüfung and vorläufig) and `aenderungsdokument` in DIVA2, no amounts. These kinds are ruled out as PDF-only.
- `Gewerbesteuerbescheid`: PDF/A-3 with embedded XML ("XUnternehmen.Gewerbesteuer", published on xrepository.de, not read yet); whether reading the embedded XML counts as machine-readable is a decision for the map.
- ERiC offers nothing for Bescheiddaten (no decode, render or compare); Otto decrypts and decompresses, elster-cli parses BDS-2 and carries the number table itself. No open-source BDS-2 parser exists.
- ESB exists only for returns filed with `bescheid="ja"` and is fetchable only by the account or key chosen at filing.

[research/01-bescheiddaten-formats.md](../research/01-bescheiddaten-formats.md)

Amended by [Download developer-area Bescheid material](02-developer-area-material.md): Vorbehalt and Vorläufigkeit are carried in BDS-2 as Bescheidwertnummer `0680000853`, and the DIVA § 51 InvStG Feststellung has an XML Anhang beside the PDF, so "every DIVA kind is PDF plus metadata only" does not hold for Anhänge.
