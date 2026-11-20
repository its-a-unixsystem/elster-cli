# ERiC and the Kontoabfrage

Ticket: `.scratch/elster-cli/issues/03-eric-kontoabfrage.md`
Researched: 2026-11-19. Terms follow `GLOSSARY.md` (Kontoabfrage, ERiC, Offene Schnittstelle, HerstellerID, Certificate).

Source keys used below:

- **[KA-SB]** `docs/API/ElsterKontoabfrage_v2.1.3.zip` > `Doku/ElsterKontoabfrage_Schnittstellenbeschreibung_V2.1.3.pdf` (Stand 24.11.2025)
- **[KA-JD]** same zip > `Doku/Jahresdokumentation_fuer_Anwendung_eric_Kontoabfrage_6_1.3_hersteller.xml`
- **[X11]** `docs/API/ElsterXML-Schnittstelle_V11_2026.9.21.0.zip` > `Dokumente/Einheitliche_Datenschnittstelle_XML_11.pdf` (Stand 2026-02-12)
- **[VDA]** same zip > `Dokumente/Verzeichnis_der_Datenarten.csv`
- **[TOK]** same zip > `Dokumente/Spezifikation_ELSTER-Token_1.27.pdf`
- **[HDR]** same zip > `Schemata/header/headerbasis_datenarten.xsd`, `headerbasis_verfahren.xsd`
- **[LIC]** ERiC licence agreement, release 41, English text as published by a manufacturer (a licensee), <https://help.scopevisio.com/en/articles/467538-licence-agreement-eric-release-41>. This is a licensee's republication, not elster.de. The tool that fetched it capped quotes at 125 characters, so the wording below is excerpts.
- **[DEV]** <https://www.elster.de/elsterweb/infoseite/entwickler> (ELSTER developer page)
- **[HIST]** <https://www.elster.de/elsterweb/attachments/bereitstellungstermine/Historie_Bereitstellungstermine.pdf> (Stand 10.07.2026)
- **[eric-rs]** <https://github.com/quambene/eric-rs> (Rust bindings `eric-bindings`, SDK `eric-sdk`; repo Apache-2.0)
- **[erica]** <https://github.com/digitalservicebund/erica> (Python service on ERiC; MIT)
- **[haben]** <https://github.com/firsttris/haben> and PR <https://github.com/firsttris/haben/pull/283> (TypeScript app on ERiC via FFI; AGPL-3.0)

## Verified facts

### 1. Is the Kontoabfrage in the Offene Schnittstelle?

- The ELSTER XML spec says single data deliveries "can also be developed completely independently, without the component provided by the tax administration (offene Schnittstelle). A listing is in the document Verzeichnis_der_Datenarten." [X11 section 1, p. 5]
- [VDA] has 18 rows. All are marked `offene Schnittstelle`. They cover four Verfahren only:
  - `ElsterDatenabholung`: `ElsterLohn2Daten`, `ElsterLohnDaten`, `PostfachBestaetigung`, `PostfachAnfrage`, `PostfachStatus`, `ElsterKMVDaten`
  - `ElsterLohn2`
  - `ElsterLohn`
  - `ElsterKMV`

  `ElsterKontoabfrage` is not in the file. [VDA]
- Neither document describes an open route for the Kontoabfrage. [KA-SB] has no sentence on "offene Schnittstelle" or on how to send without ERiC. Its only mention of encryption is the example heading "vor Verschlüsselung" (sections 5.1 to 5.3).
- The Kontoabfrage is a defined Verfahren and Datenart in the header schemas. `ElsterKontoabfrage` is an enumeration value in [HDR] `headerbasis_verfahren.xsd` line 54. `Kontoabfrage` ("Verfahren ElsterKontoabfrage") is in `headerbasis_datenarten.xsd` line 2351.
- Wire parameters of a Kontoabfrage [KA-SB section 3.1]:
  - Verfahren `ElsterKontoabfrage`
  - DatenArt `Kontoabfrage`
  - Vorgang `send-Auth` ("signed data, signature in TransferHeader")
  - Testmerker `230000001`
  - Empfänger `id="F"`, the four-digit Bundesfinanzamtsnummer
  - HerstellerID requested from the Bayerisches Landesamt für Steuern (BayLfSt), section 3.2
  - UTF-8 only
- `send-Auth` is the same Vorgang that the open Datenabholung/Postfach rows use. [VDA]
- The `send-Auth` mechanism is documented publicly, in [X11] section 2.4.1 and in `Einheitliche_Datenschnittstelle_XML_Authentifizierung_V02.2.1.pdf`:
  - gzip, then CMS hybrid encryption (RSA-OAEP + AES-256-CBC), then Base64
  - XML-DSig signature over `DatenTeil`, stored in `SigUser` in the TransferHeader
- The ELSTER token spec says the Clearingstelle interface allows only data submission. It says an individual client "can currently only use the Clearingstelle interface, since the specification of the ELSTER Online portal is not public". The Mein ELSTER portal itself offers "Steuerkontoabfrage" [TOK section 2, p. 7]. This means ELSTER offers a second, non-public way to query the tax account, through the portal. That route is the web UI, not a client API.
- Kontoabfrage request types [KA-SB sections 4.1.3 to 4.1.5]:
  - `O`: open amounts, by Steuernummer
  - `I`: payments made, with `steuerart` and `wertstellungsdatum`
  - `ZS`: amounts assessed, with `steuerart` and `zeitraum`
- A Vollmacht (authorisation) is required: "wird eine erteilte Vollmacht vorausgesetzt" [KA-SB section 3].
- All Steuernummern in one request must match the Bundesfinanzamtsnummer in Empfänger [KA-SB section 4.1.2]. The server processes one Nutzdatenblock with one Steuerfall [KA-SB section 3].

### 2. What ERiC does for the Kontoabfrage

- ERiC "compresses, encrypts, authenticates and sends via https" the ElsterXML. [X11 section 1, p. 5]
- ERiC contains an application definition for the Kontoabfrage. The Jahresdokumentation is titled "Jahresdokumentation für Datenart Kontoabfrage (Kontoabfrage) für Anwendung eric", for schema version 6 and label 1.3 [KA-JD sheet "Allg. Information"]. It defines:
  - the character set `Standard_E_V2`
  - a "Formale Fehler" sheet (155 rows) of formal error rules
  - field formats such as the 13-digit Steuernummer (format code A)
  - the namespace `http://finkonsens.de/elster/elsterkontoabfrage/kontoabfrage/v6`
  - the root element `kontoabfrage_antwort`, which is documented for the response only
- The "Formale Fehler" rules in [KA-JD] are the plausibility checks ERiC applies. This applies to the new response structure (`kontoabfrage_antwort`) in that file, and the request side is covered by the XSD only. How much of this ERiC applies to the request is not stated in the downloads. This is an inference from the "für Anwendung eric" title.
- Server-side processing is JAXB unmarshal plus XSD validation, then marshal with arbitrary namespace prefixes. [KA-SB section 4.1.1]
- The Kontoabfrage v6 schemas ship in the zip [KA-SB section 4.1]: `eko_kontoabfrage_000006.xsd`, `eko_o_abfrage_000006.xsd`, `eko_i_abfrage_000006.xsd`, `eko_zs_abfrage_000006.xsd`, `eko_basistypen_000003.xsd`, `allg000008.xsd`.
- The response is changing [KA-SB section 4.1.6]:
  - BIENE Steuerkontoauszug replaces the old Steuerkonto system.
  - Migrated Finanzämter return `kontoabfrage_antwort`. Others still return `kontoabfrage-output`.
  - Migration is per Finanzamt, and "began in November 2025".
  - A client must therefore handle both response shapes.

### 3. ERiC licence and distribution

- ERiC is free of charge and gated. [DEV, LIC]
  - [DEV]: it is "provided free of charge" for integration into the developer's own software.
  - [DEV]: the developer registers, and the IuK division of the BayLfSt checks that software development is intended.
  - [DEV]: access data is emailed "within a few days", and ERiC is then downloaded in the developer area.
  - [DEV]: a HerstellerID is applied for separately under "Anträge und Formulare".
  - [LIC §2(2)]: "provided in machine-readable form and free of charge".
- The licence is between the BayLfSt (as ELSTER coordinator) and software manufacturers. [LIC]
  - §4(1): the manufacturer gets a "non-exclusive right, unlimited in time and territory" and may "distribute, lease and make publicly accessible together with its own software as a uniform product". This means distribution only as part of the manufacturer's own product, and the manufacturer may not hand ERiC out on its own.
  - §4(2): transfer of the rights to third parties and "sub-licensing beyond paragraph 1" is not permitted.
  - §4(3): open-source components inside ERiC are licensed under their own licences by their rights holders.
  - §5(1) and (2): the manufacturer must give end users the tax authority's data-protection letter, with acknowledgement, and tell them how log files are stored (they stay local).
  - §5(4) to (7): the manufacturer "must regularly check" for new versions, must use the version the LfSt labels "minimum version", and the LfSt has no obligation to provide updates.
  - §9: the LfSt may stop use of ERiC, "for example by blocking the manufacturer ID".
- Wording note on [LIC]: it is the release-41 text from one licensee. Later releases may differ. The primary text is in the developer area, which was not accessible.
- Nothing in [DEV], [LIC] or the open-source projects states an open-source-specific rule. No source says an open-source project may or may not publish ERiC binaries.
- How existing open-source projects handle it. None of them ships ERiC.
  - [erica]: ERiC is not in the repo. External users download `ERiC-35.2.8.0-Linux-x86_64.jar` from the developer portal and copy `libericapi.so`, `libericxerces.so`, `libeSigner.so` and the `plugins2/` libs into the project. It also says Mac needs the `*.dylib` files.
  - [eric-rs]: needs `libericapi.so` and `ericapi.h` from the developer portal "after requesting access". Not bundled.
  - [haben]: its docs say the app downloads ERiC itself, "unter Einstellungen > ELSTER > ERiC selbst von der Finanzverwaltung" (fetched from `docs/elster-abgabe.md`). Direct ERiC sending needs "an own Hersteller-ID", which the BayLfSt checks and which "takes days to weeks".
- On whether each user needs their own developer login:
  - Per [DEV] and [LIC], access is granted to manufacturers. The licence says nothing about end users downloading.
  - Haben has the user's installation fetch ERiC from the Finanzverwaltung at runtime. It does so with the user's own HerstellerID, per its docs. Whether that download is anonymous or needs the developer login was not shown in the fetched pages.

### 4. Platforms and architectures

- Verified: Linux x86_64. [erica] documents it, [eric-rs] uses `ERiC-43.3.2.0-Linux-x86_64`, and [haben] loads `libericapi.so`.
- Verified: [eric-rs] `eric-bindings/build.rs` selects pre-generated bindings by `CARGO_CFG_TARGET_OS` and `CARGO_CFG_TARGET_ARCH`. Only `linux/x86_64` and `macos/aarch64` have files, and "only some versions cover macOS". This suggests an ERiC macOS arm64 build exists for some versions.
- Verified: [erica] says Mac users need `*.dylib` files.
- ELSTER's own history page lists ERiC for Linux (2008) and Mac OS X (2011). This came from a search-result snippet (<https://www.elster.de/eportal/infoseite/elster_eine_erfolgsstory>), not from a page I read in full.
- Not verified from any source: Linux arm64 and macOS x86_64 builds, and the current official platform list. The official ERiC platform list sits in the developer area, which was not accessible.

### 5. C API surface

No ERiC header or documentation was available. The following comes from the call sites of open-source bindings.

- [eric-rs] `eric-sdk/src/eric.rs` calls:
  - `EricInitialisiere(plugin_path, log_path)`
  - `EricCheckXML`
  - `EricBearbeiteVorgang`
  - `EricHoleFehlerText`
  - `EricHoleZertifikatEigenschaften`
  - `EricDekodiereDaten`
  - `EricEntladePlugins`
  - `EricBeende`
- [eric-rs] `eric.rs` builds the Datenart argument as `"{taxonomy_type}_{taxonomy_version}"`. It passes processing flags `Validate`, `Print`, `Send`, `SendAndPrint`, plus `CheckHints` and `ValidateWithoutDate`, a certificate parameter struct (path + password), and two response buffers (validation and server response).
- [eric-rs] `build.rs`: no function allowlist. Bindgen runs over the whole `ericapi.h`, and linking uses `ERIC_PATH`, with `lib/` and `include/ericapi.h` under it. It sets no rpath. The runtime library path is left to the user.
- [haben] PR 283: it loads ERiC dynamically with `koffi`, calls `EricVersion()` first, and chooses a function signature per major version.
  - ERiC 43 and older: `EricBearbeiteVorgang()` takes a transfer handle.
  - From ERiC 44: it does not take one.
  - Calling the old signature on ERiC 45 fails with code 610001526.
  - The PR states ERiC 44 libraries are not compatible with 43, and that each job starts a new worker.
- [haben] also reports that `ElsterDatenabholung` v32 has been available since ERiC 43.4. That is the Postfach Datenart, so it also runs through ERiC there.
- The Fehlerliste for the Kontoabfrage is in the zip: `Doku/Fehlerliste ElsterKontoabfrage 2.1.1.pdf`. Not read in detail.
- No Go binding exists that I could find. A search for ERiC Go/cgo packages found none.
- Rust has `eric-bindings` and `eric-sdk` (crates.io). Their README ties each SDK release to one ERiC version, for example `eric-sdk 0.5.0` to 43.3.2.0. The "only the latest ERiC version is supported" remark comes from a docs.rs search snippet.

### 6. Update cadence and end of life

- [HIST] shows the ERiC versions for the ESt year-end release:
  - Version 21.2 on 13.11.2014
  - 23.2 on 19.11.2015
  - 25.2 on 10.11.2016
  - 27.2 on 16.11.2017
  - 29.2 on 15.11.2018
  - 31.2 on 14.11.2019
  - 33.2 on 19.11.2020
  - 35.2 on 25.11.2021
  - 37.2 on 23.11.2022
  - 39.2 on 27.11.2023
  - 41.2 on 25.11.2024
  - 43.2 on 24.11.2025

  So there is one odd-numbered major version per year, with the ".2" release in the second half of November. The February release (for example 43.4 on 23.02.2026) adds the remaining tax types.
- [HIST] lists tax types and Datenarten only. A search of its text found no "Kontoabfrage" row. It gives no end-of-life dates.
- Even-numbered versions now exist. SAP sources call ERiC 44 a "pure preparation release" before ERiC 45, with functional changes at the turn of 2026/2027 (<https://www.hcm-ssc.de/eric-release-44-sap-elster/>, secondary).
- [haben] PR 283 (date not captured):
  - It uses 44.3.6.0 as default and describes it as "the newest version released for real transmissions".
  - It lists 45.1.5.0 as a beta that sends only test submissions.
  - It expects 45.2 in mid-November 2026.
  - I could not confirm that 45.2 has shipped.
- Obligation: the manufacturer must use the LfSt-designated "minimum version" and check regularly. [LIC §5(4) to (7)]
- No official end-of-life schedule per ERiC version was found. [X11] and the SSb (Datenabholung zip) say nothing about EOL.

### 7. Other findings that affect the map

- The `ElsterXML` TransferHeader is identical for all Verfahren. The Kontoabfrage differs only in Nutzdaten and the `send-Auth` Vorgang. [X11, `kleine_Uebersicht.txt`]
- [X11] Section 2.7.1: the four Clearingstelle URLs are `https://datenannahme{1..4}.elster.de:443/ERiC_IAS/ERiClet`. The sender must distribute requests across them. The same page names Verfahren-specific URL suffixes only for `ElsterAnmeldung` and `ElsterLohn`.
- [X11] revision history: "2021-08-26 / 4.2.13: ... ElsterKontoabfrage entfernt" appears in the same entry as the removal of the TESTA-network URL for authorities. I could not tell what exactly was removed. It likely refers to that URL list.

## Inferences (labelled as such)

1. **The Kontoabfrage is ERiC-only by documented policy, not by technical necessity.** The Kontoabfrage uses the same ElsterXML + `send-Auth` + Clearingstelle protocol as the open Datenabholung and Lohn rows. Whether the Clearingstelle accepts a self-built Kontoabfrage is not documented. It is not sanctioned. A self-built client would be unsupported, could be blocked, and the HerstellerID licence may not cover it. Evidence: absence from [VDA], and [KA-SB] saying nothing about an open route.
2. **ERiC probably adds plausibility checks (formal errors per [KA-JD]) before sending**, so a self-built client would also lose these. The server validates again against the XSD [KA-SB 4.1.1].
3. **An open-source CLI cannot bundle ERiC.** [LIC §4(1) and (2)] allow distribution only inside the manufacturer's own product, with no sublicensing. A public GitHub release containing `libericapi` would be a redistribution. This is a reading of the licence text, not legal advice.
4. **"Optional user-supplied ERiC" is the model existing open-source projects use** ([erica], [eric-rs], [haben]). None ships ERiC. Haben automates the download from the Finanzverwaltung.
5. **ERiC ABI instability** (function signature change in ERiC 44, per [haben] PR 283) means a CLI would need per-major-version signature handling, or must pin and check the ERiC version.
6. **FFI choice.** Rust bindgen needs `ericapi.h`, which the licence also controls, so generated bindings cannot be committed for all versions. [eric-rs] commits pre-generated bindings anyway. Whether its licence allows that is a legal question. `koffi`-style runtime FFI (as [haben]) avoids the header. Go can do the same with a `dlopen` wrapper without cgo. That Go option is a technical inference, not something a source states.

## Implications for the ERiC decision

Options:

**A. Optional user-supplied ERiC, for the Kontoabfrage only.**
- This is feasible and matches the existing open-source practice.
- Everything else (Postfach, Bestätigung, Objektspeicher, Datenabholung) stays on the Offene Schnittstelle, per [VDA].
- Costs:
  - The user must register with ELSTER and obtain ERiC.
  - The user must point the CLI at the library, or the CLI downloads it.
  - The CLI must cope with major-version API changes.
  - Linux arm64 and macOS x86_64 support is unconfirmed.
  - The ERiC licence terms apply to whoever holds the HerstellerID, which is the CLI project.
- Note that the licence obligations in §5 (end-user letters, version duty) fall on the "manufacturer".

**B. Move the Kontoabfrage out of v1, to v2 (recommended default).**
- The Kontoabfrage is one of several retrieval features. [VDA] shows all others fit the Offene Schnittstelle. Dropping it from v1 keeps v1 free of ERiC, C-FFI, licence and platform risk.
- [KA-SB 4.1.6]: the Kontoabfrage response is mid-migration to BIENE (two response shapes, finanzamtsweise since Nov 2025). Waiting reduces rework.
- Costs: no tax-account data in v1.

**C. Other route.**
- Self-build the Kontoabfrage over the open protocol anyway. It is technically plausible (same `send-Auth` format). It is unsanctioned. A test against the real Clearingstelle would show whether it is accepted, but relying on it would need explicit confirmation from the BayLfSt.
- Ask the BayLfSt whether a Kontoabfrage route outside ERiC is planned or permitted.

Suggested direction: option B for v1, with the architecture leaving a seam for A in v2. Option C is worth a short question to the BayLfSt before v2 starts. This should be confirmed in the decision ticket, not here.

Effects on other tickets:
- Anything about HerstellerID: with option A the CLI project's HerstellerID is exposed to ERiC licence terms (block risk §9). [haben] says its users each apply for their own HerstellerID, a different model from one project-wide ID.
- Language choice (Go vs Rust): ERiC FFI is the only part where this matters. Rust has `eric-bindings` and `eric-sdk`. Go has nothing. Without ERiC in v1 the choice does not turn on it.
- Release and packaging tickets: a binary release cannot contain ERiC.

## Open points

1. Primary ERiC licence text for the current release (45) and the ERiC developer documentation (platform list, API reference, EOL policy). Both are in the developer area and were not accessible. [LIC] is an old licensee copy.
2. Whether users may download ERiC themselves (anonymous or with the manufacturer's login), and whether a project-wide HerstellerID may serve end users who run their own ERiC copy.
3. Platform support: Linux arm64 and macOS x86_64 are unverified. macOS arm64 is only suggested by [eric-rs] build.rs.
4. Whether ERiC's Kontoabfrage application (`Kontoabfrage_6`) also checks the request side. [KA-JD] documents the response side only.
5. Whether the Clearingstelle accepts a Kontoabfrage that was not built with ERiC. Test, and ask BayLfSt.
6. Exact end-of-life dates per ERiC version, and which ERiC versions can handle the BIENE `kontoabfrage_antwort` response. [KA-JD] 6/1.3 says it is in the "eric" application, so ERiC 44 or 45 likely carries it. Unconfirmed.
7. Whether `ericapi.h`-derived bindings may be committed to a public repository.
8. The Kontoabfrage Fehlerliste (`Fehlerliste ElsterKontoabfrage 2.1.1.pdf`) was not analysed.
