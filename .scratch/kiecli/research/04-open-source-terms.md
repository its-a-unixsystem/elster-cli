# ELSTER terms for an open-source client

Ticket: [04-open-source-terms](../issues/04-open-source-terms.md). Researched 2026-11-19.

Legend: **[V]** verified in a primary source named next to it. **[I]** inference, not stated by any source. **[S]** secondary source (a copy or summary, not ELSTER itself).

## Source access

- The elster.de developer area (ERiC page, Schnittstellenbeschreibungen, Anträge und Formulare, Entwicklerforum) is behind a login. Fetching the pages returned only the login form; the forum returned HTTP 401. **[V]** (fetches of `elster.de/elsterweb/entwickler/infoseite/{eric,schnittstellenbeschreibungen,ressourcen}` and `forum.elster.de/herstellerforum/`).
- The ERiC licence (`lizenz.pdf`, shown before download) and the developer-area terms were therefore **not read in the original**. The only copy found is a customer-facing reproduction by Scopevisio, **[S]**, used below with that label.
- The public overview page `https://www.elster.de/eportal/infoseite/entwickler` states the sign-up path: register as Hersteller/Entwickler; the IuK area of the Bayerisches Landesamt für Steuern (BayLfSt) checks "ob Softwareherstellung beabsichtigt ist"; access data arrive by email; ERiC is downloaded; then the HerstellerID is applied for; newsletter recommended. ERiC is "kostenlos" provided. The page gives no licence, redistribution, trademark or Testlauf terms. **[V]**

## 1. One HerstellerID for all users, or one per user?

Verified statements (all from the local downloads in `docs/API/`, unzipped to scratch):

- "Die HerstellerID muss von jedem teilnehmenden Hersteller beim Bayerischen Landesamt für Steuern beantragt werden." Same wording in `ElsterDatenabholung` SSb V32.0.1 section 5.1.1.5 (with the application form URL `https://www.elster.de/elsterweb/entwickler/antrag-hersteller-id/form`), in `ElsterKontoabfrage` SSb V2.1.3 section 3.2 (application by mail to `id@elster.de`), and in the schema doc of `HerstellerIDSType` in `headerbasis_datentypen.xsd`. **[V]**
- Schema doc: the HerstellerID is "eine für jeden Hersteller eindeutige Zuordnungsnummer. Nur Fälle, die von registrierten Softwareherstellern gesendet werden, werden angenommen und verarbeitet." Format: five digits, `\d{5}`. **[V]**
- Purpose per SSb Datenabholung 5.1.1.5: "um den Hersteller möglichst schnell über Probleme in Daten seiner Software zu informieren, um den Schaden für Kunden möglichst gering halten zu können." **[V]**
- OTTER SSb v1.4.6: headers `x-elster-endanwender-typ = "HERSTELLERID"` and `x-elster-endanwender-wert` = "Die HerstellerID des Anwenders". The HerstellerID therefore also travels on Objektspeicher calls. **[V]**
- The transport spec `Einheitliche_Datenschnittstelle_XML_11.pdf` says the Offene Schnittstelle may be implemented "vollkommen selbständig ... ohne Verwendung der ... Komponente" and points manufacturers to the Entwicklerforum article "HerstellerIDs und Testlauf" (not accessible, login). **[V]**

What no accessible source says: whether an open-source tool counts as a "Hersteller", whether the end user of an open-source tool may be treated as its own Hersteller, and whether one ID may be shared by all users of a project. Nothing in the documents defines "Hersteller" or limits the number of installations per ID. **[V]** (absence, checked by grep across all text of the downloads).

Practice of existing open-source clients (primary: their own READMEs and code):

| Project | Licence | HerstellerID handling |
|---|---|---|
| rechtlogisch/csotto, rechtlogisch/cotto (Otto/OTTER download demos, C# and C++) | MIT | Each user sets their own five-digit ID in env var `DEVELOPER_ID`; README: "You should set your five-digit Developer-ID (German: Hersteller-ID)". Marked "not intended for productive usage". **[V]** (`README.md` on GitHub `main`) |
| vog/geierlein (own implementation of UStVA transmission, no ERiC) | AGPL | A single ID `herstellerID: '00616'` is hard-coded in `chrome/content/lib/geierlein/steuerfall.js` and used for every user; the generic documentation ID `74931` is sent when test mode is on. **[V]** (source read on GitHub `master`). Whether ELSTER agreed to this is unknown. |
| digitalservicebund/erica, tech4germany/steuerlotse (ERiC wrappers) | MIT | README sends the reader to the developer portal for ERiC and test certificates; no HerstellerID in the repo. **[V]** |

So existing practice splits two ways: user-supplied ID (cotto family, and this project's standing decision "HerstellerID comes from config, never code") versus one project-wide ID in source (geierlein, legacy). **[V]**

Inference **[I]**: since ELSTER links the ID to a registered manufacturer that it can contact and block (see section 2), a single shared ID for an anonymous user base is the riskier model; one ID per user or organisation, supplied by the user, keeps every blocking and every support contact with the party that caused it. Whether ELSTER will actually issue an ID to a private person or company that only runs someone else's open-source tool is **not answered by any source** and needs a question to ELSTER.

## 2. Obligations attached to a HerstellerID

- Bestätigung: "Nach erfolgreicher Abholung muss zwingend eine Bestätigung innerhalb von 24 Stunden erfolgen. Softwarehersteller, die dies nicht beachten, müssen mit einer Sperrung rechnen!" (SSb Datenabholung V32.0.1, 6.4). **[V]** The sanction is stated against the "Softwarehersteller", i.e. the ID holder. **[I]**: with a shared ID, one user who collects without confirming could get every user blocked.
- Testmerker: "Wenn bei einer Datenlieferung kein Testmerker gesetzt wird, handelt es sich um echte/produktive Daten" (Einheitliche_Datenschnittstelle_XML_11, p. 6) and, in the Datenabholung SSb, "Wird ein Testmerker angegeben, dürfen nur Testdaten übermittelt werden (Test-IdNr, Test-Steuernummer, etc)". **[V]**
- Test certificates only work with Testmerker; "Werden Fälle ohne Testmerker versendet, so wird ein serverseitiger Fehler provoziert." (`Test_Zertifikate/readme.txt`). **[V]**
- Optional `Hersteller/ProduktName` and `ProduktVersion` in the NutzdatenHeader "z.B. ... bei Supportanfragen" (SSb Datenabholung 5.1.2.3). Nothing makes product registration or version registration mandatory in these documents. **[V]** (absence in the local downloads)
- `DatenLieferant` is "der Hersteller der Software" (Kontoabfrage SSb 3.3); TransferHeader and NutzdatenHeader entries should match. **[V]**
- Testlauf: the only trace is the pointer to the forum article "HerstellerIDs und Testlauf" in the transport spec. A user asked in the (public) Anwenderforum whether proof of a successful test submission is needed before a HerstellerID is issued; no ELSTER answer in the thread (21.07.2023, `forum.elster.de/anwenderforum/.../429126-obtaining-herstellerid-for-new-software`). **[V]** that it is unanswered; the requirement itself is **unknown**.
- Applying: processing time reported as long, a user waited without reply (same thread). **[V]** as an anecdote.
- ERiC licence (applies only if ERiC is used; **[S]**, Scopevisio's copy of the Release 41 agreement at `help.scopevisio.com/en/articles/467538-licence-agreement-eric-release-41`): licence grants integration of ERiC into the manufacturer's own software; ERiC is "intended exclusively for use in connection with the electronic submission of tax returns"; distribution only "together with its own software as a uniform product"; no sublicensing; users must get the data-protection notice; breaches may lead to blocking "for example by blocking the manufacturer ID" (section 9); open-source components are outside the agreement. These terms bind the ERiC licensee; they do not apply to the Offene Schnittstelle path, but the same registration and ID underlie both. **[I]** that the Offene Schnittstelle needs no ERiC licence acceptance; the SSbs do not mention one.

## 3. Redistribution of schemas, documentation, examples, test certificates

What the local files contain (all unzipped under the scratchpad, checked by grep over every `.xsd`, `.xml`, `.yml`, `.txt`, `.csv` and the extracted text of every PDF):

- **No licence text anywhere.** No `.xsd`, example XML, `otter.yml` or CSV carries a copyright or licence header; the schemas carry only a version comment (e.g. `<!-- Schemaversion 11 aus dem Downloadpaket ElsterDatenabholung Version 32.0.0 -->`, `<!-- Version: 2026.9.21.0 Erstelldatum: 2026-09-16 -->`). **[V]**
- PDFs carry only a page footer "Copyright © ELSTER" or "Copyright © Verfahren ELSTER". No grant, no restriction. **[V]**
- `Test_Zertifikate/readme.txt`: the test PSEs "dürfen nicht weitergereicht werden und dürfen nur zu Tests durch den Softwarehersteller verwendet werden"; PIN 123456; test CA, test account only; no login to the ElsterOnline portal. **[V]**
- `oeffentlicherClientSchluessel/liesmich.txt`: `extern-offen.p12` is "das Zertifkats/Schluesselpaar um sich gegenüber dem Datenannahmeserver ... als Client der offenen Schnittstelle auszuweisen", password `change_me`, valid until 15.04.2029. No redistribution statement. **[V]**
- The Clearingstelle public keys come with a Readme stating they are needed "wenn nicht die Clientkomponente ERiC verwendet wird". Public keys, no terms. **[V]**
- `kleine_Uebersicht.txt` describes the package ("Downloadpaket") and its purpose, no terms. **[V]**
- OTTER `otter.yml` has `contact: name: OTTER, url: https://forum.elster.de/herstellerforum/` and no licence field. **[V]**

Public availability (checked today with HTTP HEAD, no login, no cookies): `https://download.elster.de/download/schnittstellen/` serves `Test_Zertifikate.zip`, `Fehlerliste.xml`, `ElsterDatenabholung-v32.0.1.zip`, `ElsterObjektspeicher_v1.4.6.zip`, `ElsterKontoabfrage_v2.1.3.zip` and `ElsterXML-Schnittstelle_V11_2026.9.21.0.zip` with HTTP 200. **[V]** This contradicts the comment in the repo's `.gitignore` ("login required"). The cotto repo relies on the same URL: its `get-test-certificate.sh` downloads `Test_Zertifikate.zip` from there at setup time and extracts only `test-softorg-pse.pfx`, instead of committing it. **[V]** (script on GitHub `main`).

Practice of others: Erica `.gitignore`s its `cert.pfx` and the ERiC libs; Steuerlotse states the test certificate is "not included in the repo"; cotto/csotto do not ship the ERiC libs or the certificate and say ERiC "is subject to a separate license agreement". **[V]**

Reading **[I]**: public download is not a licence. With no stated terms, the safe reading under German copyright law is "all rights reserved by the Freistaat Bayern / ELSTER" for PDFs, and unknown for schemas and examples. Schemas are interface definitions needed to interoperate, and there is no sign ELSTER objects to their use; copying the files into a public repository is still a redistribution nobody has permitted in writing. The test certificates are the one case with an explicit prohibition ("nicht weitergereicht").

## 4. Names, logo, trademark

- "ELSTER" is a registered trademark of the Freistaat Bayern: elster.de footer shows "ELSTER ®" (`elster.de/eportal/infoseite/rechtliches`) and the ELSTER Anwenderforum imprint calls it "ein eingetragenes Markenzeichen des Freistaates Bayern" (via search result summary, `forum.elster.de/anwenderforum/impressum`, not fetched in full). **[V]** for the footer; the imprint wording is search-summary level. The cotto README adds its own disclaimer: "ELSTER is a registered trademark of the Freistaat Bayern, represented by the Bayerische Staatsministerium der Finanzen." **[V]** as that project's practice; I found no DPMA register number.
- Logo: BayVV 206-D-12940-32, section 9 (`gesetze-bayern.de/Content/Document/BayVV_206_D_12940-32`), regulates the ELSTER logo for authorities ("nutzungsberechtigte Stelle"): use only per the Freistaat's templates, only where it relates to ELSTER-certificate authentication in the authority's web application, no alteration, Freistaat as "Markenrechtsinhaber". **[V]** The rule addresses authorities, not software makers. No logo terms for manufacturers were found. **[I]**: do not use the logo.
- Naming: no source restricts product names that contain "elster". Existing projects use it freely ("Elster client" Geierlein, "Erica", "ElsterAuthenticator" is ELSTER's own). **[V]** for the projects' names; absence of a rule is **[I]** because the developer-area terms were not accessible. The glossary term "Elster client" is listed under _Avoid_ for ERiC; that is a project convention, not a legal one.
- Descriptive use ("a client for ELSTER") with a non-affiliation disclaimer is the pattern cotto uses ("This demo was developed by ... should be used only for test purposes"). **[V]** for cotto.

## Implications

What may go in the public repo:

- Own code and own text, under the project's chosen licence. **[I]**
- Do **not** commit: any file from `Test_Zertifikate.zip` (explicit "nicht weitergereicht"), `extern-offen.p12` (client key of ELSTER; no terms; password and file are in the download), the PDFs, the `Fehlerliste.xml`, the zips. Keep `docs/API/` gitignored (the comment in `.gitignore` should be corrected: the files are publicly downloadable but not licensed for redistribution).
- Schemas (`.xsd`), `otter.yml`, example XML: no terms found, no explicit permission. Default **[I]**: do not commit copies until ELSTER answers (open point 1). Instead, fetch them at build/test time from `download.elster.de/download/schnittstellen/` as cotto does for the test certificate (script + documented URL, files git-ignored), or write fixtures from scratch with invented data in the same shape. Hand-written minimal fixtures with fake Steuernummern are the lowest-risk option for CI.
- The Kontoabfrage example files contain complete (test-shaped) output including a Steuernummer; do not copy them even if permission arrives without checking that they hold only test data.
- Never the real HerstellerID in repo, docs, tests or logs. The documentation ID `74931` appears in every ELSTER specification as an example (`Einheitliche_Datenschnittstelle_XML_11.pdf`, Kontoabfrage and Authentifizierung SSbs, and geierlein's test mode); using it only in obviously-labelled examples is standard, but note that geierlein sends it for test submissions, so it may be accepted by the test system. Whether ELSTER intends it as a shared test ID is **unverified**.

How the HerstellerID must be handled:

- User-supplied via config/env/flag (`ELSTER_HERSTELLER_ID`-style), validated as five digits (`\d{5}`), never defaulted, never embedded. Matches cotto's practice and the map's standing decision.
- Be explicit in README and `SKILL.md`: the user must hold their own HerstellerID (or an ID they are authorised to use) and is responsible for the obligations attached to it; the project does not provide one. This is the conservative reading given the wording "jeder teilnehmende Hersteller". **[I]**
- Bestätigung: the SSb makes it mandatory within 24 h and threatens blocking of the ID holder. The map's decision "Bestätigung only on explicit request, never implicit" conflicts with this when an ID is used for real collection; see "Changes to other tickets" in the hand-off and open point 3. Minimum: document the 24 h rule prominently and make the CLI warn when it retrieved a Postfach-Nachricht that is still unconfirmed.
- Send `Hersteller/ProduktName` = the tool name and `ProduktVersion` = the release (optional per SSb 5.1.2.3) so ELSTER can recognise the client when they contact the ID holder; this also helps ELSTER distinguish the tool from a bespoke integration. **[I]**
- Use Testmerker on every test-system call; never send real data with test certificates (server error) and never send a Testmerker with real data. **[V]** from the readme and SSb.

Naming:

- "elster-cli" is a descriptive nominative use of "ELSTER", not endorsed by anything found. No source forbids it; the trademark owner is the Freistaat Bayern. Low risk, not zero. Keep it, add a non-affiliation / trademark notice in the README (cotto's wording is a precedent), do not use the ELSTER logo or imitate its look, and consider whether the ID holder wants a less ELSTER-branded name if ELSTER objects. **[I]**

## Open points (email to ELSTER; the BayLfSt developer contact is `id@elster.de` for HerstellerID per the Kontoabfrage SSb, the forum `forum.elster.de/herstellerforum/` for technical questions)

Do not include personal or company details in public forum posts; ask via the developer area, as the registered manufacturer.

1. Redistribution: may the `.xsd` schemas, `otter.yml`, example XML and the interface PDFs from `download.elster.de/download/schnittstellen/` be committed to a public, open-source repository (and under what attribution)? If PDFs are no, are schemas and examples yes?
2. HerstellerID model for an open-source tool: may one ID be shared by all users of the project, must each user or organisation hold its own, or may a user act as "Hersteller" simply by running the tool? Who is blocked if one user misbehaves? Is a private individual or in-house developer eligible for an ID?
3. Bestätigung and the 24 h rule: confirm that the rule binds every ID holder collecting real data, and whether a tool that deliberately leaves Bestätigung to the caller is acceptable (it conflicts with the project's decision to confirm only on explicit request).
4. Testlauf: is a successful test-system run required before an ID is issued or activated, and is it per product, per version, or per ID? Is product/version registration required (the SSb makes `Hersteller/ProduktName` and `ProduktVersion` optional)?
5. Is the ID `74931` from the specs a valid shared test ID on the test system, and is the test certificate bundle (`test-softorg-pse.pfx`, PIN 123456) acceptable to use in public CI (the readme limits use to "Softwarehersteller" and forbids passing the files on; downloading them on the CI runner from the public URL is not "passing on", but that is **[I]**)?
6. Use of the name "ELSTER" in a product name ("elster-cli") and of the term in package names; confirm no objection, and whether a non-affiliation notice suffices.
7. Is `extern-offen.p12` (client certificate of the Offene Schnittstelle, public in the download, password `change_me`) meant to be embedded in manufacturer software and redistributed, or must each manufacturer use it only from the download? This decides whether a Go/Rust binary may embed it.
8. If ERiC ends up in scope (Kontoabfrage): the ERiC licence must be read in the original (`lizenz.pdf` in the ERiC package) for the points "uniform product", "no sublicensing", and the open-source clause; likely an ERiC-linking release cannot be a standalone open-source binary (users would download ERiC themselves, as cotto/erica require). Not needed if Kontoabfrage works on the Offene Schnittstelle.

## Sources

Local (unzipped to the session scratchpad, not in the repo): `docs/API/ElsterDatenabholung-v32.0.1.zip` (SSb V32.0.1, schemas), `ElsterKontoabfrage_v2.1.3.zip` (SSb V2.1.3), `ElsterObjektspeicher_v1.4.6.zip` (OTTER SSb, `otter.yml`), `ElsterXML-Schnittstelle_V11_2026.9.21.0.zip` (TDoc 2026-02-12, header schemas, `kleine_Uebersicht.txt`, key readmes), `Test_Zertifikate.zip` (`readme.txt`).

Web (fetched 2026-11-19): `https://www.elster.de/eportal/infoseite/entwickler`; `https://www.elster.de/eportal/infoseite/rechtliches`; `https://www.gesetze-bayern.de/Content/Document/BayVV_206_D_12940-32`; `https://forum.elster.de/anwenderforum/forum/allgemein/allgemein-und-projekt/429126-obtaining-herstellerid-for-new-software`; `https://help.scopevisio.com/en/articles/467538-licence-agreement-eric-release-41` **[S]**; `https://download.elster.de/download/schnittstellen/` (HEAD checks); GitHub raw files: `rechtlogisch/cotto` and `rechtlogisch/csotto` (README, LICENSE.md, `get-test-certificate.sh`), `digitalservicebund/erica` (README, LICENSE, `.gitignore`), `tech4germany/steuerlotse` (README, LICENSE), `vog/geierlein` (README, `steuerfall.js`).
