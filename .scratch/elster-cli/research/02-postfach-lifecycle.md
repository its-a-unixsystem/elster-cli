# 02 - Postfach-Nachricht lifecycle and Objektspeicher attachments

Ticket: `issues/02-postfach-lifecycle.md`. Terms follow `GLOSSARY.md`.

Sources (all from `docs/API/`, unzipped to a scratch directory):

- **SSb-DA**: `SSb ElsterDatenabholung - Extern V32.0.1.pdf` (page numbers are the PDF's "Seite x von 50").
- **XSD-DA**: `Schemata/datenabholung_32.xsd` (ElsterDatenabholung v32.0.1 zip).
- **EX-DA**: `Beispiele_TH11/Abholung-PostfachNachricht-Daten/*` (request/response XML).
- **SSb-OTTER**: `OTTER-SSb-v1.4.6.pdf` (page "Seite x von 17"; section numbers 5.x).
- **OAS**: `OpenAPI/otter.yml` v1.4.6 (ElsterObjektspeicher zip).
- **CSV**: `Verzeichnis_der_Datenarten.csv` (ElsterXML-Schnittstelle V11 zip).
- **FL**: `Fehlerliste.xml`.
- **Web**: elster.de public help page "FAQ zur Abholung von Bescheiddaten" (https://www.elster.de/eportal/helpGlobal?themaGlobal=help_bescheiddaten).

Claims marked **[inference]** are not stated in a source; they are my reading and need confirmation on the test system.

## 1. Lifecycle of a Postfach-Nachricht

SSb-DA p.9: the retrieval is three steps, "Anfrage, Abholung und Bestätigung. Alle drei Schritte müssen durchlaufen werden." The tax administration provides the Postfach-Nachricht; the client then does:

1. **Anfrage** (`PostfachStatus` for counts, `PostfachAnfrage` for metadata plus object IDs). SSb-DA p.5 (5.1.1.2): PostfachStatus = "Anzahl der Treffer je Bereitstellungsdatenart ohne Abholung"; PostfachAnfrage = "Abholung der Metadaten und OTTER-Objekt-ID".
2. **Abholung** of the payload and attachments from the Objektspeicher by object ID (SSb-DA p.22 section 6.3, p.18-19 `Datenpaket`/`Anhaenge`). This is not an ElsterXML call; it is OTTER `fetch` (see section 6).
3. **Bestätigung** (`PostfachBestaetigung`, DatenArt in the Transferheader, SSb-DA p.5 and p.23). "Nach erfolgreicher Abholung muss zwingend eine Bestätigung innerhalb von 24 Stunden erfolgen. Softwarehersteller, die dies nicht beachten, müssen mit einer Sperrung rechnen!" (SSb-DA p.23, section 6.4).

### Server-side states (Statusabfrage)

SSb-DA p.26 (6.6.2) lists the status of a Bereitstellung:

| Code | Meaning (SSb-DA wording) |
|---|---|
| 05 EINGESTELLT | cannot yet be fetched (e.g. Bescheiddatum in the future) |
| 07 INBEKANNTGABE | e-mail has been sent |
| 09 DATENSATZFEHLERHAFT | record has errors, cannot be announced (e.g. target account no longer exists or inactive) |
| 10 BEREITGESTELLT | can be fetched |
| 20 ABGEHOLT | fetched, not yet confirmed |
| 30 BESTAETIGT | fetched and confirmed |
| 90 GELOESCHT | deleted from the database after the Löschfrist |
| 99 UNBEKANNT | id or ticket not found |

What a client can observe at each step, via `PostfachAnfrage` `einschraenkung`: "neue" = only not yet collected records; "alle" = no restriction on collection status (SSb-DA p.11 and p.13). So `neue` corresponds to states 10 (and presumably 20, see open points); `alle` includes confirmed ones **[inference]**.

Important distinction: `Statusabfrage` is about a Bereitstellung a party made itself (sender role, e.g. EPBescheid, EPAntwort, "selbst bereitgestellten Nachricht", SSb-DA p.25 section 6.6). It uses `rolle="sender"|"empfaenger"`. A reader-only CLI (receiver) can use `rolle="empfaenger"` **[inference]**; the SSb gives example files `3_a_Statusabfrage_Request` with "Rolle" (p.30) but those are not in the Beispiele_TH11 folder shipped (only the six Vertreter examples are).

## 2. Operations (request/response shapes)

All are ElsterXML (`Verfahren` = `ElsterDatenabholung`, Empfaenger `id="L"` content `CS`, SSb-DA p.7; Nutzdaten namespace `http://finkonsens.de/elster/elsterdatenabholung/v3`, `version="32"`, SSb-DA p.8). `Vorgang` is `send-Auth` (signed with the Certificate, no Fingerprint) or `send-NoSig` (only for PostfachNachricht-Daten, Fingerprint element mandatory) (SSb-DA p.5, 5.1.1.3). CSV lists `PostfachAnfrage`, `PostfachStatus`, `PostfachBestaetigung` with both `send-Auth` and `send-NoSig` as "offene Schnittstelle". `Statusabfrage` is not in the CSV lines found for ElsterDatenabholung (only the Postfach* and `ElsterLohn*`/`ElsterKMVDaten` lines matched my grep for Datenabholung); treat it as unconfirmed Offene Schnittstelle.

### 2.1 PostfachStatus

Request, SSb-DA p.10-11 (6.2.1) and XSD-DA `PostfachStatusRequestCType`:

- `einschraenkung` "neue"|"alle" (XSD default "alle").
- `von`, `bis` `JJJJMMTT` (records before/after are not considered).
- `rolle` "alle"|"vertreter"|"inhaber" (XSD default "inhaber").
- No `DatenartBereitstellung` children in the XSD, so no per-Datenart or per-filter request. The Change-Log (SSb-DA p.3) and the changelog table (p.3 row 1, 24.11.2025) say the new filters apply to "PostfachAnfrage und PostfachStatus", but the XSD for PostfachStatus carries only von/bis/einschraenkung/rolle. **Discrepancy**; test before relying on filters in PostfachStatus.
- SSb-DA p.3: "AenderungslisteDUe nicht unterstützt" in PostfachStatus.

Response (`PostfachStatusAntwort`, SSb-DA p.11-12, XSD-DA): repeats the restrictions; one `DatenartBereitstellung name=… anzahltreffer=…` per Datenart (1 to 500,000), with `rolle` and, for `vertreter`, `postfachinhaberalias`. SSb-DA p.12 example groups by role into two `PostfachStatusAntwort` elements; the shipped EX-DA 6_c2 response instead has one element with `rolle` per `DatenartBereitstellung` (and duplicate entries). Parse both shapes.

FL: error 118 "Feature PostfachStatus ist für <Client> mit Datenabholungsversion <…> nicht mehr unterstützt" (Fehlerliste.xml around line 1596). PostfachStatus may be withdrawn for older client versions.

### 2.2 PostfachAnfrage

Request, SSb-DA p.13-14 (6.2.2.1), XSD-DA `PostfachAnfrageRequestCType`. Only one Nutzdatenblock allowed (p.13).

| Attribute | Value | Source |
|---|---|---|
| `einschraenkung` | neue / alle | p.13 |
| `von`, `bis` | JJJJMMTT | p.13 |
| `max` | 1-10000; default 1,000 if absent | p.13 |
| `paginierungLetzteBestId` | `bestid` from previous response | p.13, p.19 |
| `rolle` | alle / vertreter / inhaber (XSD default inhaber) | p.13, XSD |

Child `DatenartBereitstellung` (1..n required by XSD): "Es müssen dabei alle DatenartBereitstellung angegeben werden, welche abgeholt werden wollen." (p.13). There is no "all types" wildcard; the client must name each Bereitstellungsdatenart. Per-Datenart filter attributes (p.13-14 and XSD): `name` (required), `ag-steuernummer`, `mandantennummer`, `lieferticket`; the XSD also has `veranlagungszeitraum` and `bevorzugterempfaenger`. SSb-DA p.14 example uses `veranlagungszeitraum="2020"` on DIVABescheid, but the attribute table (p.13-14) does not list it; the Change-Log (p.3) says `veranlagungszeitraum (ab vsl. Oktober)`; the mapping tables (p.38-48) footnote `veranlagungsjahr`/`veranlagungszeitraum` "vsl. ab Mitte Juni filterbar". Treat `veranlagungszeitraum`/`veranlagungsjahr` as announced but not yet reliable. Filterable Ordnungsbegriffe per tables 9.4/9.5 (marked `*`): `lieferticket`, `mandantennummer`, `veranlagungsjahr`, `ag-steuernummer`; attributes `veranlagungszeitraum`, `bevorzugterempfaenger`. The p.13 text refers to a "Filterung" chapter that is not in this PDF (dangling reference). The example on p.14 uses `version="31"` in a v32 doc (doc typo).

Response `PostfachAnfrageAntwort`, SSb-DA p.15-19 and XSD-DA:

- Attributes repeat `einschraenkung`, `von`, `bis`, `max`, plus `paginierungLetzteBestId` (only when the page is full).
- Per `DatenartBereitstellung`: `name`, `anzahltreffer`, `rolle`, `postfachinhaberalias`; if rolle=vertreter a `VertreterInformation` (AccountId, Rolle, Vertreteralias, PostfachinhaberAlias, VertretungVon, VertretungBis, `yyyyMMddHHmmss`); then `Bereitstellungen/Bereitstellung`.
- Each `Bereitstellung`: attributes `id` (the "bestid", format like `be1237nw3…`, 32 chars lowercase), `bereitstellungsticket`, `groesse` (XSD `PAAntwortBereitstellungCType`; SSb example p.16, EX-DA 12_c2), children `MetaInformationen`, `Datenpaket`, `Anhaenge`.
  - `MetaInformationen`: `Meta name=…` key/value list plus optional `MetaInfoMaps`. Which keys exist per Datenart: mapping tables 9.4 and 9.5 (SSb-DA p.38-49). **Always** returned from v31: `Meta name="verfuegbarbis"` `yyyymmdd` "Datum bis zu dessen Erreichung der Datensatz abholbar ist / Löschdatum des Eintrags" (p.17).
  - `Datenpaket`: `ReferenzId` (OTTER object ID), `Encoding` (ISO-8859-15 or UTF-8), `Groesse` (size of the encoded, compressed and encrypted content) (p.18, XSD).
  - `Anhaenge/Anhang`: `DateinameMitErweiterung`, `Dateibezeichnung`, `Dateityp`, `DateiReferenzId` (OTTER object ID), `DateiGroesse`, optional `MetadatenAnhang/MetadatumAnhang` key/value, e.g. `DokumentHierarchieTyp` = HAUPTDOKUMENT/NEBENDOKUMENT/VORBLATT/ANLAGE/AUSFERTIGUNG (p.19). Up to 20 `MetadatumAnhang` per attachment (XSD `MetadatenAnhangCType`). DIVA2 adds keys `DateibezeichnungKurz`, `DateibezeichnungID`; they are "nicht zwingend eindeutig", so do not derive file names from them alone (p.37).
- The 12_c2 example (EX-DA) shows `anzahltreffer="0"` entries without `Bereitstellungen` allowed (XSD `minOccurs="0"` sequence).

Note p.16-17 of the text-layer drops the end of the `Bereitstellung` element; I read the XSD and example instead.

### 2.3 Pagination

SSb-DA p.19-20 (6.2.2.4): max hits per request is limited (default 1,000, up to 10,000 via `max`). When more hits exist, the response carries `paginierungLetzteBestId` = "der letzten (nach lexikografischer Reihenfolge) gelieferten Bereitstellungs-ID". Send it unchanged in the next request, with identical other restrictions ("von-bis"). Caveat: with several Datenarten in one request, a page may return nothing for one Datenart because its IDs sort later; its data then only appears on the matching page. So loop until `paginierungLetzteBestId` is absent, and merge by Datenart across pages. Page size for XML response bounded also by 500 MiB sum of Nutzdatenblöcke in the VaSt flow (p.27); not stated for Postfach.

### 2.4 PostfachBestaetigung

SSb-DA p.23-24 (6.4.1) and XSD-DA:

- Request: `PostfachBestaetigung/Bereitstellungen/Bereitstellung id="…"`; up to **100 Bereitstellungen per Nutzdatenblock**, up to **100 Nutzdatenblöcke** per ElsterXML.
- Vertreter: supported from Datenabholung v32; "Auch für eine Vertretung abgeholte Nachrichten müssen bestätigt werden."
- Response `PostfachBestaetigungAntwort/Bereitstellungen/Bereitstellung id` each with `Rueckgabe` (`Code`, `Text`; EX-DA 2_c2: `0` "Der Empfang der Daten wurde erfolgreich bestaetigt.").
- NutzdatenHeader RC: `0` bearbeitet; `190015302` "Keine Daten vorhanden" (records not found); `190017112` at least one not successfully confirmed (SSb-DA p.23-24).
- Which ID to confirm: the `Bereitstellung id` from PostfachAnfrage (EX-DA 2_a: ids look like `eh0360…`, a different prefix than the `be…` example ids; ID prefixes are not specified).

### 2.5 Statusabfrage

SSb-DA p.25-26 (6.6), XSD-DA `StatusabfrageRequestCType`/`StatusabfrageResponseCType`:

- Request `Statusabfrage rolle="sender|empfaenger"` containing either `BereitstellungsTickets/BereitstellungsTicket` or `BekanntgabeVorgangsIds/BekanntgabeVorgangsId`, max 100.
- Response (note the XSD root `Bereitstellungen/Bereitstellung`): `Status` (e.g. `30-BESTAETIGT`), `BereitstellungsTicket`, `BekanntgabeVorgangsId`, `EMailVersandZeitpunkt`, `AbrufZeitpunkt`, `LoeschZeitpunkt`, `Fehler` (`Nummer`, `Text`). Timestamps `YYYYMMDDHHMISS`.
- A `bereitstellungsticket` (attribute on each Bereitstellung in PostfachAnfrage) is the key for this call, so a receiver can check the state of a known message, including `90-GELOESCHT` and `LoeschZeitpunkt`.

## 3. Retention after Bestätigung

What the sources say:

- `verfuegbarbis` (every Bereitstellung since v31) is the date until which the record can be fetched, i.e. its deletion date (SSb-DA p.17, p.37 table, "v" = mandatory for DIVA2).
- Statusabfrage `90-GELOESCHT` = "nach der Löschfrist aus der Datenbank gelöscht", with `LoeschZeitpunkt` (SSb-DA p.26).
- Default reply deadline for formatted Bescheide uses "der DIVA-Löschzeitraum"; for Mitteilungen 60 days (SSb-DA p.33). The DIVA-Löschzeitraum is **not given as a number** in any of the provided documents.
- Public help (Web, "FAQ zur Abholung von Bescheiddaten"): "Die Bescheiddaten stehen 180 Tage nach Bekanntgabe zur Abholung zur Verfügung." and "Nicht abgeholte Bescheiddaten werden nach 180 Tagen, abgeholte Bescheiddaten 60 Tage nach Abholung gelöscht." This page is about Bescheiddaten (ESt-Bescheid for natural persons, Mein ELSTER); it says nothing about Organisationen or other Datenarten, so for DIVA* and others read `verfuegbarbis` per record.

Is a Nachricht still retrievable after Bestätigung? The SSb does not state it outright. What follows from the text:

- Status 30 BESTAETIGT is distinct from 90 GELOESCHT, so a confirmed record exists in the Bereitstellungsdatenbank until the Löschfrist (SSb-DA p.26).
- `einschraenkung="alle"` is "Keine Einschränkung auf Abholstatus" (p.13), so confirmed records can be listed again **[inference]**.
- The public FAQ states retrieved data is deleted 60 days after Abholung, i.e. data is not removed at Bestätigung but after a grace period (Web). Whether the *Bestätigung* or the *Abholung* starts that clock, and whether OTTER `fetch` of an already-confirmed object still works, is not documented. **Test on the test system**: PostfachAnfrage `alle` after a Bestätigung, then `fetch` again.
- Nothing documented about "Erneut abholen"; the forum mentions a Mein ELSTER link (Web search result, forum), not authoritative.

Practical consequence for the CLI: Bestätigung is mandatory within 24 h *of a successful Abholung* (SSb-DA p.23), but it is a stateful side effect on ELSTER's side. A stateless agent-oriented CLI that reads with `einschraenkung="alle"` can read without confirming, but a client that fetches objects and never confirms violates the stated rule ("Sperrung"). Hence confirm must be a separate explicit command, and the doc should warn that a `fetch` starts a 24 h obligation. See open points.

## 4. Bereitstellungsdatenarten: Organisationszertifikat vs natural person

Not documented. What the sources give:

- SSb-DA tables 9.4/9.5 (p.38-49) list about 100 Bereitstellungsdatenarten and which Ordnungsbegriffe (e.g. `steuernummer`, `ag-steuernummer`, `mandantennummer`, `accountid`, `idnr`, `finanzamt`) accompany them. They do not mark which certificate type can receive which Datenart.
- Lists of Datenarten in SSb-DA: DIVA stage 1: `DIVAAnzeigeEinerLotterie, DIVABescheid, DIVABuchmachersteuer, DIVALotteriesteuerEinmal, DIVALotteriesteuerMehr, DIVASportwettensteuer, DIVATotalisator, OZGBescheid, VerbindlicheAuskunft, VerbindlicheZusage` (p.34); DIVA2: `DivaBescheidESt, DivaBescheidKSt, DivaBescheidGewSt, DivaBescheidUSt, DivaBescheidFEIN, DivaSonstigerVA, DIVATroncabgabe, DIVAVirtuelleAutomatenStB, DIVAOnlinePokerStB` (p.35). Others in the table: `EPBescheid`, `EPMitteilung`, `EPKurzmitteilung`, `EPAntwort`, `ESB`, `GSB`, `USB`, `GEWMB`, `Gewerbesteuerbescheid`, `KMVProtokoll`, `ELOProtokoll`, `EBilanzRueckmeldung`, `STBKE`, etc. (STBKE appears in an EX-DA response but not in the table text I could extract; table extraction lost columns, so cell-level values may be wrong).
- `Vertretung` text names `EPBescheid`, `EPMitteilung` and DIVA Datenarten as "für eine AccountID bereitgestellt" (p.9).
- OTTER SSb 5.10 (p.10): the Vertreterregelung "soll für StNr. Konten allerdings nicht für die Id-Nr. Konten bereitgestellt werden" (Steuernummer accounts = organisations, IdNr accounts = natural persons). Vertreter exists only for Steuernummer (Organisations-) accounts and is managed only via Mein ELSTER.
- Examples (EX-DA) show `AuthZuordnungskriterium` = `Organisation` or `Steuernummer Organisation` and `AuthCA` = `ElsterSoftOrgCA`: i.e. the Transferheader's SigUser carries the account kind. A natural person's soft certificate would carry a different CA/Zuordnungskriterium **[inference]**; the values for natural persons are not in the examples.
- The Kommunen-Leitfaden (Web, https://download.elster.de/download/elstertransfer/dokumente/2023-12-21_Leitfaden_Datenuebertragung_Kommunen.pdf, section 6) says ELSTER-Transfer users register in Mein ELSTER "mit einem Organisationszertifikat (Steuernummer)"; one registration per organisation, optionally one certificate per organisational unit. That is for ELSTER-Transfer, not the Postfach.
- Reasonable reading **[inference]**: the server returns only the Bereitstellungen addressed to the authenticated account (the `AuthAccountID`); the Datenarten an account receives follow from what the sender (Finanzamt/Kommune) provides to that Steuernummer/IdNr. An organisation account sees e.g. DIVA*, GEWMB, `Gewerbesteuerbescheid`, ESB/USB; a natural person sees `EPBescheid`, `EPMitteilung`, ESt-related. The tool should therefore not hardcode a per-certificate allowlist; it should either require `--datenart` or ship a maintained default list and report `anzahltreffer=0` per type.

Consequence: because PostfachAnfrage requires naming every Datenart (no wildcard), a "list everything" command needs a built-in list of all Datenarten from table 9.4 (about 100 names) and can use `PostfachStatus` first to learn which names have hits, then call PostfachAnfrage only for those. PostfachStatus returns the Datenarten that have hits without needing the list **[inference from p.11]** (the example shows only non-empty types).

## 5. Content formats

- The Nutzdaten XML only carries metadata and references. The content itself is an OTTER object per `Datenpaket/ReferenzId` and one per attachment (`DateiReferenzId`) (SSb-DA p.18-19, p.22).
- `Datenpaket.Encoding`: ISO-8859-15 or UTF-8 (p.18). So the main payload is a text/XML document (structured Bescheiddaten, e.g. `EPBescheid` data), not a PDF; PDFs travel as `Anhaenge` with `DateinameMitErweiterung` e.g. "Bescheid.pdf" (p.19).
- The payload is "kodiert, komprimiert und verschlüsselt" (p.18/p.19 Groesse; VaSt `Datenpaket` p.29: "Kodierter (komprimierter und verschlüsselter)"). OTTER delivers it as CMS/PKCS#7 EnvelopedData for the requesting certificate (SSb-OTTER p.16 section 5.12.4.3), after decrypting you get the (gzip-compressed, per the Transferheader `Kompression` GZIP) content; the exact inner layering (gzip then base64?) is not specified in these documents. The XML docs `Einheitliche_Datenschnittstelle_XML_11.pdf` cover `Datei/Kompression`, `Verschluesselung`.
- Formatted messages: `MetaInfoMap typ="ANSCHREIBEN"` with `sendername`, `bezeichnungnachricht`, `textnachricht` (base64-encoded text) (SSb-DA p.32). HTML formatting for EPMitteilung/EPBescheid is described in "Formatierte Nachrichten" (changelog p.3 row 3), but the actual HTML structure rules are not in this PDF (section only has the ANSCHREIBEN example). Types `ANSCHREIBEN`, `ANTWORTLINK`, `ANTWORT` apply to EPBescheid, EPMitteilung, EPKurzmitteilung (p.17). `ANTWORT` carries `zuordnungskriterium`, `antwortfrist` (default: DIVA-Löschzeitraum for Bescheide, 60 days for Mitteilungen) and `MetaList typ="antwortbetreff"` (p.33). `MetaListElement` text up to 32,768 characters (p.18). Replying (EPAntwort) is a write operation: out of scope for v1 (read-only).
- Structured Bescheiddaten: DIVA metadata lists (p.34-37) give e.g. `bescheiddatum`, `steuernummer`, `veranlagungsjahr`, `vorlaeufigkeitsmerker`, `aenderungsdokument`, `verfuegbarbis`; the Bescheid itself is an attachment (HAUPTDOKUMENT).
- Size limits: each payload up to 3 GiB; one Bereitstellung may consist of several payloads totalling at most 7 GiB (SSb-DA p.22; OTTER fetch also 3 GiB, SSb-OTTER p.16 5.12.4).
- ISO-8859-15 payloads need transcoding for a JSON-on-stdout CLI.

## 6. Objektspeicher attachment flow

Base URL `https://objektspeicher.elster.de/<Restful-Endpunkt>` (DOI network: `https://objektspeicher.elster.doi-de.net/…`), HTTPS 443, SNI required, TLS 1.3 only with `TLS_AES_128_GCM_SHA256`, `TLS_AES_256_GCM_SHA384`, `TLS_AES_128_CCM_SHA256` (SSb-OTTER p.6-7, 5.2-5.4). OAS server path `/api/v1`: `GET /api/v1/obj/{objektid}` (fetch), `POST /api/v1/obj` (send), `GET /api/v1/health`.

Transport authentication: "Die Authentifizierung gegenüber OTTER erfolgt durch ein Client-Zertifikat." (SSb-OTTER p.6, 5.3), and OAS: 401 "Der Client darf die Schnittstelle nicht verwenden. z.B. Das falsche Clientzertifikat wurde verwendet." plus error text "Der Client mit den folgenden CN darf die Schnittstelle nicht verwenden". **The documents never say which TLS client certificate** (see open points). Separately, the request itself is authorised by the signed header (below). FL has OTTER-specific errors: "Im Rahmen des OTTER-Authentisierungsprozesses konnte kein aktives ELSTER-Konto gefunden werden." (component 05, number 011), "OTTER-Autorisierung … Zugriff verweigert" (020, 035), "OTTER-Authentifizierung … verweigert" (034) (Fehlerliste.xml lines ~232, 295, 379, 386), which suggests the signed header is mapped to a Mein ELSTER account.

### 6.1 fetch (download an attachment or Datenpaket)

`GET /api/v1/obj/{objektid}`; `objektid` = `ReferenzId`/`DateiReferenzId` from PostfachAnfrage, lowercase hex with hyphens, format uuid, e.g. `1b00d1f0-0300-0af0-cc0e-03400ab0ef55` (SSb-OTTER p.14 footnote *; OAS). Note: the example `id`s in the SSb-DA are a different format (`be…`, 32 chars) because those are Bereitstellung ids (for Bestätigung), not object ids.

Header table (SSb-OTTER p.14-15 Table 3, OAS), columns: A = signed with POZ or Signaturkarte; B = signed with unregistered Signaturkarte; C = signed with Fingerprint (CEZ); "unsigned" is internal use only.

| Header | A: POZ / registered card | B: unregistered card | C: CEZ |
|---|---|---|---|
| `x-elster-objektid-signiert` | CMS signature over objektid, Base64 | same | not sent |
| `x-elster-empfaenger-typ` | not sent | `ABHOLZERTIFIKAT` | `FINGERPRINT` |
| `x-elster-empfaenger-wert` | not sent | the Abholzertifikat (PEM Base64, no line breaks, without BEGIN/END labels) | fingerprint of the CEZ |
| `x-elster-empfaenger-wert-signiert` | not sent | not sent | signed fingerprint (made with the self-signed cert) |
| `x-elster-endanwender-typ` | `HERSTELLERID` (required) | same | same |
| `x-elster-endanwender-wert` | the HerstellerID (required) | same | same |
| `x-elster-client-identifier` | optional free text | optional | optional |
| `Accept-Encoding` | optional (`gzip`) | optional | optional |
| `x-elster-transferticket` | required, new ticket per request | same | same |
| `x-elster-api-version` | required, e.g. `1.4.6` | same | same |
| `x-elster-verschluesselungsart` | optional: `CMSEnvelopedData` or `PKCS7EnvelopedData` | same | not sent |
| `x-elster-unterstuetzte-verschluesselungsarten` | optional, comma list | same | not sent |

Header value alphabet limited to A-Z a-z 0-9 and `! # $ % & ' * + - . ^ _ ` | ~` and space (5.7, p.10). Content of the response: 200 `application/octet-stream`, chunked, optional gzip (when `Accept-Encoding: gzip`; "gzip-Komprimierung fällt bei fetch weg" per change log row 11, the exact status is ambiguous) (SSb-OTTER p.9, 5.6.2, OAS). Response header `x-elster-empfaenger-identifier-typ` ∈ `ACCOUNT_ID, DUEBEL_ID, CERTIFICATE_FINGERPRINT, ORGANISATIONS_ID` (OAS; for unsigned requests used to decrypt, p.16 5.12.4.2). Errors 400, 401 (JSON `Fehler.error.{fehlercode,fehlertext,retry-sinnvoll}`), 404, 500 (retry makes sense), 503 (retry makes sense).

For signed requests "werden die Objekte vom fetch-Service für den Benutzer umgeschlüsselt … kryptografische Nachrichten nach RFC 5652, Kapitel 6 (Enveloped-data)" (SSb-OTTER p.16 5.12.4.3): the object is encrypted for the public key of the Certificate (or Abholzertifikat); the client decrypts with its private key.

### 6.2 What is signed and with which certificate

SSb-OTTER p.7-9 (5.5): signatures use CMS SignedData (RFC 5652): content is an OCTET STRING of the text value; digest SHA-256; `certificates [0]` = the signer certificate (embedded); `crls` not set; SignerInfo `signedAttrs` = `signingTime` (UTCTime) and `messageDigest` (the example shows `contentType` too; order of signed attributes matters, strict implementations), `unsignedAttrs` not set; `signatureAlgorithm` in the example is rsaEncryption. The signed content for `x-elster-objektid-signiert` is the lowercase hyphenated object ID string (e.g. `468a69d4-0151-4681-9e8d-fcd87873d550`; 36 chars, example digest equals `echo -n … | sha256sum`). The signature is sent Base64-encoded (OAS: "objektid signiert und danach Base64-kodiert"; footnote **: "Base64-kodierte Signatur der objektid mit Benutzerkonto-Zertifikat/Signaturkarte").

- "POZ" = Portalzertifikat, created on registration at Mein ELSTER (SSb-OTTER p.17 abbreviations). This is the glossary **Certificate** (soft-PSE `.pfx`), so for our CLI column A applies: sign the object ID with the Certificate key, send `x-elster-objektid-signiert`, and no `empfaenger-*` headers.
- "CEZ" = Clienterzeugtes Zertifikat (self-generated). It corresponds to the Datenabholung `Vorgang=send-NoSig` with `Fingerprint` element (SSb-DA p.5 and p.8: fingerprint computed by `openssl x509 -pubkey | openssl rsa -RSAPublicKey_out -outform DER | openssl md5`, signed with the private key, Base64 PKCS#7 signed-data). On OTTER the same fingerprint goes into `x-elster-empfaenger-wert`, and a signature over it into `x-elster-empfaenger-wert-signiert`. Whether the CEZ is registered anywhere: not documented; SSb-DA only says "Nutzer der offenen Schnittstelle" may use the self-generated certificate.
- "Abholzertifikat" (column B): for users with an unregistered Signaturkarte; the Datenabholung XML element `Abholzertifikat` has the same purpose ("Wird eine nicht bei Mein ELSTER registrierte Signaturkarte zur Authentifizierung verwendet, muss immer das Element für die Umschlüsselung geliefert werden", SSb-DA p.8). Signaturkarten/ElsterStick cannot do RSAES-OAEP and get RSA 1.5 (PKCS#7 v1.5 enveloped) (SSb-DA p.25 6.5.1); the OTTER `x-elster-verschluesselungsart` header lets the client pick `CMSEnvelopedData` vs `PKCS7EnvelopedData`. Test marker `300000002` makes the Datenabholung response RSA-OAEP so one can test OAEP support (SSb-DA p.5-6).
- The same HerstellerID goes into `x-elster-endanwender-wert`. The OTTER SSb says nothing else on obtaining it (ticket 06).

### 6.3 Is a separate registration or certificate needed for the Objektspeicher?

- No separate *user* certificate is documented: signing uses the same Certificate as ElsterXML (POZ). The OTTER SSb is "für Clients außerhalb von ZPS ELSTER-Kommunikation und Schnittstellen für Postfachnutzer" (p.6, 5.1), i.e. aimed at Postfach users generally.
- A TLS **client certificate** is required at the transport layer (5.3, OAS 401 text). Not stated whether it is the generic `extern-offen.p12` public client key that ships in the ElsterXML download (`oeffentlicherClientSchluessel/liesmich.txt`: "um sich gegenüber dem Datenannahmeserver der Finanzverwaltung als Client der offenen Schnittstelle auszuweisen", PIN `change_me`, valid to 15.04.2029) or an OTTER-specific one. **Needs a spike** (ticket 11): try `extern-offen.p12` as TLS client cert against the test OTTER host. The OTTER doc does not publish a test host name; only production and DOI hostnames (p.6).
- The sender must have a Mein ELSTER account that OTTER can map ("kein aktives ELSTER-Konto gefunden", FL 05/011).
- The Fehlerliste also has `Otterprüfung gescheitert` and `Der Otter-Aufruf ist gescheitert` in component 70 (Fehlerliste.xml ~lines 3220-3250), i.e. server-side messages arriving in ElsterXML, which belong to *send* paths.

### 6.4 send (upload; not needed for a read-only v1)

`POST /api/v1/obj`: used to upload large objects, after which the client has **7 days** to submit an ElsterXML referencing the object ID; the same object ID may be reused in several ElsterXMLs; after 7 days deleted (SSb-OTTER p.10, 5.11.1). Size 10 MiB minimum, 600 MiB maximum (p.13, 5.11.4). Headers (p.11 Table 2): `x-elster-checksumme-signiert` (CMS-signed raw SHA-256 digest bytes, Base64) for signed requests, `x-elster-checksumme-algorithmus: SHA256`, `x-elster-endanwender-*`, `x-elster-transferticket`, `x-elster-api-version`. Response 201 with `x-elster-objektid` and `x-elster-objekt-groesse-in-bytes`. This is the write path (e.g. sending an Antwort with attachments): out of scope for v1 except as context.

### 6.5 health

`GET /api/v1/health` with `x-elster-api-version`; 200 `{"service":"OTTER-Health","status":0,"description":"OTTER ist erreichbar."}` (SSb-OTTER p.16, 5.13; OAS). Useful as `doctor` check.

### 6.6 Retention in the Objektspeicher

Only for `send`: 7 days (above). For objects provided for fetch the OTTER SSb gives no retention; it follows the Postfach record's `verfuegbarbis` **[inference]**.

## 7. Behaviours to design around

- A page of 1,000 default results may be missing Datenarten; always paginate until `paginierungLetzteBestId` is absent.
- Each Bereitstellung is addressed by two kinds of ID: `Bereitstellung/@id` (for Bestätigung and pagination), `bereitstellungsticket` (for Statusabfrage), and object IDs (`ReferenzId`, `DateiReferenzId`) for fetch.
- Bestätigung is per Bereitstellung, covers the full set of its payloads/attachments, and must follow a *successful* collection.
- `rolle` default is `inhaber` in the XSD; for organisations with Vertretung use `alle` to include delegated mail (EX-DA 12_c1 uses `rolle="alle"`).
- Vertretung: "Aktuell ist es noch nicht möglich für Kunden sich produktiv als Vertreter einzutragen. Das Feature befindet sich im Testbetrieb" (SSb-DA p.9). Treat the Vertreter response elements as optional in v1.
- Datenabholung request signing requires the Transferheader `SigUser/AuthInfo` (AuthAccountID, AuthCA, AuthMedium, AuthZuordnungskriterium) as in the EX-DA examples; transport and signing mechanics are ticket 01.

## Operations a v1 CLI must expose

1. **`postfach status`**: PostfachStatus; options `--einschraenkung neue|alle`, `--von`, `--bis`, `--rolle`. Output: counts per Datenart (and per role/alias). Cheap probe, no side effects.
2. **`postfach list`**: PostfachAnfrage with automatic pagination (`max` up to 10,000, `paginierungLetzteBestId` loop); options `--datenart` (repeatable; default = all names the status call reports or a built-in list from table 9.4), `--einschraenkung`, `--von/--bis`, `--rolle`, and per-Datenart filters `--lieferticket`, `--ag-steuernummer`, `--mandantennummer`, `--veranlagungszeitraum`. Output: Bereitstellung `id`, `bereitstellungsticket`, `groesse`, `Meta` map incl. `verfuegbarbis`, `Datenpaket` and `Anhaenge` references. No fetching, no side effects.
3. **`postfach get <bereitstellung-id>`** (or `--objektid`): OTTER fetch of `Datenpaket` and each `Anhang` with the Certificate-signed `x-elster-objektid-signiert`, decrypt (CMS/PKCS7 EnvelopedData), gunzip as applicable, decode ISO-8859-15/UTF-8, write attachments to a caller-given directory or stdout. Starts the 24 h Bestätigung obligation, so the output should state that.
4. **`postfach confirm <id>...`**: PostfachBestaetigung, batch up to 100 ids per block (and up to 100 blocks), explicit command only; per-id return codes; exit code non-zero on RC 190015302 / 190017112.
5. **`postfach state <bereitstellungsticket>...`**: Statusabfrage (up to 100), to see 20/30/90 and `LoeschZeitpunkt`; mark experimental until confirmed to work for `rolle="empfaenger"`.
6. **`objektspeicher health`** (could be folded into a `doctor` command): OTTER `health`.

Not in v1: OTTER send, EPAntwort/Antwort, VaSt-Beleg-Daten (`Anfrage`/`Abholung` with `idnr`, SSb-DA p.27-29; this is Belegdaten, different from the Postfach), Vertretung management.

## Open points

1. **Retrieval after Bestätigung**: not documented. Test: after Bestätigung, `PostfachAnfrage einschraenkung=alle` and `fetch` again; check `Statusabfrage` for `LoeschZeitpunkt`. Public FAQ (Bescheiddaten): deleted 60 days after Abholung, 180 days if not collected.
2. **DIVA-Löschzeitraum**: no number in the provided documents; use the per-record `verfuegbarbis`.
3. **TLS client certificate for OTTER**: unspecified (SSb-OTTER 5.3); test whether `extern-offen.p12` works, and whether a test host exists (only production and DOI host names are documented).
4. **Which Datenarten an Organisationszertifikat vs a natural person receives**: undocumented; PostfachStatus is the discovery mechanism. Table 9.4/9.5 columns were lost in PDF text extraction; re-read the PDF visually (pages 38-49) if cell-level detail is needed.
5. **CEZ semantics**: whether a self-generated certificate is acceptable for a taxpayer's Postfach, and how the Fingerprint path interacts with account mapping; likely requires registration or is for sender-defined Datenarten only. Our Certificate is a POZ, so the CEZ path is probably not needed.
6. **`PostfachStatus` filters** (lieferticket etc.) claimed by the changelog but absent in the v32 XSD for PostfachStatus.
7. **`veranlagungszeitraum`/`veranlagungsjahr` filter** announced "ab vsl. Mitte Juni / Oktober"; the p.14 example uses it already. The referenced "Filterung" chapter is missing from the PDF.
8. **Inner layering of the downloaded object** (envelope, gzip, base64) is not described in the SSb-DA/OTTER documents; read `Einheitliche_Datenschnittstelle_XML_11.pdf` (`Datei`/`Kompression`/`Verschluesselung`) and verify on the test system.
9. **Statusabfrage as receiver** and its listing in the CSV for `Offene Schnittstelle`.
10. **PostfachStatus feature withdrawal** (FL error 118, "nicht mehr unterstützt" for some client versions): confirm that v32 clients are not affected.
11. The example files shipped (Beispiele_TH11) cover only the Vertreter cases; no Postfach response with real `MetaInformationen`/`Datenpaket`/`Anhaenge` content. The test system must supply real samples (ticket 11).
12. The OTTER SSb is marked "Nur für den Dienstgebrauch" in its header and "Status: in Arbeit": check redistribution terms before quoting it (ticket 04).
