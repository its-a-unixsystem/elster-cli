# Bescheiddaten

Label: wayfinder:map

## Destination

An implementation-ready spec at `.scratch/bescheid-daten/spec.md` for an elster-cli v1.x addition that turns the Bescheiddaten ELSTER delivers into JSON from three sources: the BDS-2 Datenpaket of `ESB`/`USB`/`GSB`, the XUnternehmen.Gewerbesteuer XML embedded in a `Gewerbesteuerbescheid`, and the DIVA2 metadata of every Verwaltungsakt. Each is a faithful copy, plus named key figures (festgesetzte Steuer, Abrechnung, Vorauszahlungen, Bescheidkennzeichnung) where the format defines them. Building it is a separate effort after this map.

## Notes

- **Parked (2026-11-19)** until v1 is built and [Collect real Bescheid samples](issues/03-collect-real-samples.md) has run against the real Postfach. Real samples may make parts of the current scope moot (e.g. BDS-2 if ESB/USB/GSB never arrive), and the command surface and JSON shape are easier to decide on top of v1's actual output. Resume by taking that ticket first.
- Domain language: `GLOSSARY.md` (Verwaltungsakt, Bescheid, Bescheiddaten, Bescheidkennzeichnung, Datenpaket, Anhang, Bereitstellungsdatenart). Use its terms in tickets and the spec.
- Builds on the [elster-cli v1 map](../elster-cli/map.md) and its decisions: Rust, everything through ERiC, stateless, JSON for agents, semver'd output. v1 writes the Datenpaket raw to disk ([Agent interface](../elster-cli/issues/09-agent-interface.md)); this effort only adds to that (a minor version), the raw file stays.
- Standing decisions from charting:
  - Faithful copy first, interpretation second: like the Kontoabfrage ([Kontoabfrage requests and responses](../elster-cli/issues/13-kontoabfrage-requests.md)), the JSON mirrors ELSTER's structure; named key figures are added only where the format defines them unambiguously.
  - elster-cli itself produces the JSON; consumers do not parse Datenpakete. Command shape is open.
  - Scope is three sources ([Redraw the destination after the format finding](issues/04-redraw-destination.md)): BDS-2 of `ESB`/`USB`/`GSB` (and `GEWMB` if confirmed), the XUnternehmen.Gewerbesteuer XML in a `Gewerbesteuerbescheid`, and decoded DIVA2 metadata (keyed on the fields present, so every Verwaltungsakt). Other XML Anhänge are detected by namespace and reported, never converted.
  - Only data ELSTER delivers machine-readably: files embedded in a PDF count; reading the printed page (text extraction, OCR) does not.
  - The Gewerbesteuer XML is carried as delivered, marked as not the Bescheid and not compared with the PDF; a PDF without XML is an ordinary result. XUnternehmen is implemented from the public spec only: no vendored XSDs, spec text or Musterbescheide while its licence is unconfirmed; fixtures are invented.
  - The spec states plainly that ESB/USB/GSB exist only for returns filed with `bescheid="ja"` by the same account, so v1.x users may rarely see them.
- Source material: public ELSTER downloads in `docs/API/` (gitignored, never committed), the ERiC Jahresdokumentation (`https://download.elster.de/download/eric/eric_<major>/ERiC-<version>-Dokumentation.zip`, no login), the ERiC-Entwicklerhandbuch (ch. 15.4 Bescheiddaten record format per haben's roadmap). Material from the logged-in developer area is downloaded by the human into `docs/API/`.
- Real samples: this effort may use the real Postfach with the real Certificate to collect sample Datenpakete (explicit Bestätigung). Files stay outside the repo; only anonymised structure notes (element names, field numbers, encodings, never values) enter it. Public repo rule from v1 still holds: no personal or real tax data in tickets, fixtures or code.
- Reference implementation: haben (`../haben`, `packages/elster/src/postfach.ts`) parses the Anfrage metadata of DIVA Bescheide (`bescheiddatum` etc.), not the Datenpaket; its `docs/roadmap.md` names the ESB Bescheidwertnummern as missing.
- Research findings live in `.scratch/bescheid-daten/research/` (no remote, so no research branches).

## Decisions so far

<!-- one line per resolved ticket: [title](issues/NN-slug.md): gist -->

- [Bescheiddaten formats per Bereitstellungsdatenart](issues/01-bescheiddaten-formats.md): only `ESB`/`USB`/`GSB` (probably `GEWMB`) carry numeric Bescheiddaten, as fixed-width BDS-2 text (layout public in ERiC-Entwicklerhandbuch 15.4, number meanings only in developer-area `Bescheidnr_*.zip`), and only for returns filed with `bescheid="ja"` by the same account; every `Diva*`/`DIVA*`/`OZGBescheid`/`EPBescheid` kind is PDF plus metadata only (DIVA2 metadata has `vorlaeufigkeitsmerker`, no amounts); `Gewerbesteuerbescheid` is PDF/A-3 with embedded XUnternehmen XML; ERiC offers nothing for Bescheiddaten.
- [XUnternehmen.Gewerbesteuer in the Gewerbesteuerbescheid](issues/05-xunternehmen-gewerbesteuer.md): the embedded XML is the public XÖV standard XUnternehmen.Gewerbesteuer (1.4 productive from 2026-09-01, 1.5 and 1.6 to follow, not backward compatible), carrying Messbetrag, Hebesatz, festgesetzte Steuer, Vorauszahlungen, Zinsen, Fälligkeit and Vorläufigkeit/Vorbehalt codes; `lopdf` extracts it from the name tree without rendering; it has no Bescheid character (PDF governs), is mandatory only for HKR software so may be absent, and has no licence statement (don't vendor XSDs or Musterbescheide).
- [Download developer-area Bescheid material](issues/02-developer-area-material.md): `Bescheidnr_Juni_2026.zip` holds ODS tables (1,865 ESt, 401 GSt, 229 USt Bescheidwertnummern, each typed Festsetzung/Abrechnung/Vorauszahlung), no licence text anywhere; corrections to the formats ticket: Vorbehalt/Vorläufigkeit are in BDS-2 (`W` record `0680000853`), and the DIVA § 51 InvStG Feststellung carries an XML Anhang beside the PDF, so DIVA kinds are not all PDF-only.
- [Redraw the destination after the format finding](issues/04-redraw-destination.md): keep, narrowed to three sources (BDS-2 of ESB/USB/GSB, the embedded Gewerbesteuer XML as a marked non-binding copy, decoded DIVA2 metadata for every Verwaltungsakt); unknown XML Anhänge reported, not converted; glossary gains Verwaltungsakt and Bescheidkennzeichnung.

## Not yet specified

- Linking the two deliveries of one tax: ESB/USB/GSB data (not legally binding) and the Diva PDF (binding) arrive as separate Postfach-Nachrichten; whether and how the JSON relates them (`veranlagungsjahr`, `bescheiddatum`, Steuernummer). The Gewerbesteuer XML's `transferticket` may link a Kommune Bescheid to the filed Erklärung (unverified).
- Format evolution: BDS-2 versions `0003`/`0004` and later, XUnternehmen.Gewerbesteuer namespaces 1.4–1.6 and later (how many to support at once); how the CLI recognises and reports an unknown version, how new versions and Bescheidwertnummern get added.
- Fixtures and tests: hand-written invented Datenpakete derived from the anonymised structure notes.
- Assembling `spec.md` and folding it into the v1 spec's command and output sections.

## Out of scope

- Comparing a Bescheid with the filed return or with the Kontoabfrage (Sollstellungen): the agent's job on top of the JSON.
- Deadline calculation (Einspruchsfrist from `bescheiddatum`): the agent's job.
- PDF text extraction or OCR of the Bescheid Anhang.
- Bescheide received as Vertretung (out of scope in v1 too).
- Writing back (EPAntwort, Einspruch): sending is v2.
- Mapping the § 51 InvStG Feststellung XML Anhang: niche recipients (Spezial-Investmentfonds investors); recognised by namespace and reported, addable later as a minor version ([Redraw the destination after the format finding](issues/04-redraw-destination.md)).
- Generic XML→JSON conversion of unknown Anhänge: lossy, and its shape is the sender's, which breaks semver'd output.
- Checking the Gewerbesteuer XML against its PDF: would need PDF text extraction.
