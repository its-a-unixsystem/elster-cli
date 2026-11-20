# XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid

Type: research
Status: resolved
Blocked by:

## Question

What does the XML embedded in a `Gewerbesteuerbescheid` PDF/A-3 contain, and how is it specified?

[Bescheiddaten formats per Bereitstellungsdatenart](01-bescheiddaten-formats.md) found that a Kommune's Gewerbesteuerbescheid is delivered as a PDF/A-3 with embedded XML, specified as "XUnternehmen.Gewerbesteuer" on xrepository.de (v1.4 and later), outside ELSTER. Cover: the schema and its versions; which key figures it carries (Messbetrag, Hebesatz, festgesetzte Gewerbesteuer, Abrechnung/Fälligkeit, Vorauszahlungen); whether embedding is mandatory for every Kommune sending through ELSTER or optional; how the XML is attached inside the PDF/A-3 (embedded file name, `AFRelationship`) and what a Rust reader needs to get it out without rendering the PDF; the licence of the specification and schemas (may an open-source binary ship or generate from them).

Output: `.scratch/bescheid-daten/research/05-xunternehmen-gewerbesteuer.md`.

## Answer

**The Gewerbesteuerbescheid's embedded XML is a fully specified, public XÖV dataset (XUnternehmen.Gewerbesteuer, versions 1.4 to 1.6 on xrepository.de) that carries Messbetrag, Hebesatz, festgesetzte Steuer, Vorauszahlungen, Zinsen, Fälligkeit and Vorläufigkeit/Vorbehalt codes, and a Rust reader can pull it out of the PDF/A-3 with `lopdf` without rendering; but it is an auxiliary dataset (the PDF governs), not guaranteed present, not backward compatible between versions, and published without any licence statement.**

- Five root messages (`bescheide.gewerbesteuer.0001`, `bescheide.zinsen.0002`, `bescheide.vorauszahlung.0003`, `bescheide.gewerbesteuer.generisch.0010`, `berechnung.gewerbesteuer.0021`); version in the namespace, productive 1.4 from 2026-09-01, 1.5 from 2027-03-01, 1.6 from 2027-09-01; older 1.2.x/1.3 use another namespace; the releases are explicitly not backward compatible.
- `bescheidKennzeichnung` codes cover § 165/§ 164 AO Vorläufigkeit and Vorbehalt der Nachprüfung, the only defined Vorläufigkeit value in any Bescheiddaten format so far; amounts are decimals (negative = Erstattung); `transferticket` links to the filed Erklärung.
- Embedding is required of HKR software (spec II.4, Lastenheft) and recommended in spec I.1.3, but ELSTER's Bereitstellung only demands an Anhang and I found no check; the XML "hat keinerlei Bescheidcharakter", so the JSON should mark provenance and not claim equality with the PDF.
- In the six Musterbescheide the XML is the single entry of the `/Names /EmbeddedFiles` tree (name `GEWST-BS|BR-<AGS>-<Steuernummer>-<Datum>_<ID>.xml`, `AFRelationship` `/Unspecified`), while the catalog `/AF` array also lists stale copies; `lopdf` (MIT) extracted all six correctly; `mupdf` is AGPL and `pdf-extract` is text-only.
- Licence: none stated anywhere; only "no licence fees" (FAQ-GewSt Q8). Do not vendor the XSDs, spec text or Musterbescheide until confirmed with the XUnternehmen Geschäftsstelle.

[research/05-xunternehmen-gewerbesteuer.md](../research/05-xunternehmen-gewerbesteuer.md)
