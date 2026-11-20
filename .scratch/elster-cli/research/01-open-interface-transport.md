# Open-interface transport, end to end

Research for ticket `issues/01-open-interface-transport.md`. Scope: what a client must do, byte for byte, to send a `send-Auth` request over the Offene Schnittstelle and read the response, without ERiC.

## Sources and how to read the citations

Primary sources, all from `docs/API/`:

| Short name | File | Notes |
| --- | --- | --- |
| **EDS** | `ElsterXML-Schnittstelle_V11_2026.9.21.0.zip` → `v11/Dokumente/Einheitliche_Datenschnittstelle_XML_11.pdf` (v4.3.0, 2026-02-12) | Cited as `EDS S.n` (page footer "Seite n von 27"). |
| **AUTH** | same zip → `Einheitliche_Datenschnittstelle_XML_Authentifizierung_V02.2.1.pdf` (2026-05-15) | `AUTH S.n` (page n of 12). |
| **TOKEN** | same zip → `Spezifikation_ELSTER-Token_1.27.pdf` (2017-07-11, old) | `TOKEN S.n` (page n of 37). |
| **XSD** | same zip → `v11/Schemata/header/*.xsd` | File and type named. |
| **README-*** | same zip → `oeffentlicherSchluesselClearingstelle/Readme.txt`, `oeffentlicherHTTPSSchluesselClearingstelle/Readme.txt`, `oeffentlicherClientSchluessel/liesmich.txt`, `kleine_Uebersicht.txt` | |
| **SSb-DA** | `ElsterDatenabholung-v32.0.1.zip` → `SSb ElsterDatenabholung - Extern V32.0.1.pdf` (2026-04-23) and `Beispiele_TH11/` | `SSb-DA S.n` (page n of 50). |
| **Fehlerliste** | `Fehlerliste.xml` | 477 entries. |
| **Test-Zertifikate** | `Test_Zertifikate.zip` → `readme.txt`, three `.pfx` | PIN 123456. |
| **OTTER** | `ElsterObjektspeicher_v1.4.6.zip` → `OTTER-SSb-v1.4.6.pdf` | Only for the one line about the Objektspeicher. |

Items marked **[probe]** come from a live experiment I ran against `datenannahme1.elster.de` on 2026-11-19, using only the public test material shipped in the downloads (the `extern-offen.p12` client key, `test-softorg-pse.pfx`, Testmerker, no real taxpayer data). The probe was a throwaway Python script outside the repo. It built the whole request (CMS, signature, headers) and sent it. it could not get past the HerstellerID check (see Open point 1), so everything the server does after that check is unverified. Each probe claim says what exactly was observed.

## Summary of the answer

1. The request is an ElsterXML document. Four header/body elements carry `gzip → CMS → Base64` payloads: `SigUser`, `DatenLieferant`, `TransportSchluessel` and `DatenTeil` (EDS S.10, XSD `KompressionSType`).
2. For `send-Auth` the client puts an XML-DSig signature (RSASSA-PSS, SHA-256, detached, over the already encrypted and Base64-encoded `DatenTeil`) inside `SigUser/Sig`, then encrypts `SigUser` (EDS S.10, AUTH S.5-S.10).
3. Encryption is CMS (RFC 5652) `EnvelopedData` for the Clearingstelle: RSA-3072 key, RSAES-OAEP with SHA-256 and MGF1-SHA-256, content AES-256-CBC (EDS S.11).
4. The reply is encrypted either symmetrically (`CMSEncryptedData`, the request's AES key is reused, no client certificate needed) or with `CMSEnvelopedData` to a certificate that the client uploads in `TransportSchluessel` (EDS S.11).
5. Transport is HTTPS POST to `datenannahme[1-4].elster.de/ERiC_IAS/ERiClet` (EDS S.12). **[probe]** The server demands a TLS client certificate. The public `extern-offen.p12` from the download package is accepted for the handshake.
6. **[probe]** The example HerstellerID 74931 is **not** accepted. The server answers `HerstellerID ist nicht zugelassen` for 74931 exactly as it does for 99999 or 12345. A real HerstellerID is needed even for test requests.

## Ordered steps: request

### R0. Load the Certificate (PKCS#12 soft-PSE)

- Container is PKCS#12 (TOKEN S.10). Private keys are in KeyBags with `friendlyName` `SignatureKey` (SK.USR.AUT) and `EncryptionKey` (SK.USR.ENC). The matching certificates are found through the `localKeyID` attribute (OID 1.2.840.113549.1.9.21), value is the ASCII text `Time <millis>` (TOKEN S.11-S.12, S.19).
- Spec says: private keys `pbeWithSHAAnd3-KeyTripleDES-CBC` (1.2.840.113549.1.12.1.3), certificate bag `pbeWithSHAAnd40BitRC2-CBC` (1.2.840.113549.1.12.1.6), 1024 iterations (TOKEN S.11).
- **[probe]** The shipped test files differ from that spec (`openssl pkcs12 -info`): MAC SHA-1 with 100000 iterations; key bags `PBES2, PBKDF2, AES-256-CBC, 10000 iterations, hmacWithSHA256`; certificate bag still `pbeWithSHA1And40BitRC2-CBC`, 50000 iterations. `friendlyName`s are lower case (`signaturekey`, `encryptionkey`). A parser must therefore support both PBES2/AES-256 and the legacy RC2-40 PBE.
- Keys are RSA. TOKEN S.9 says 2048 bit; **[probe]** the test PSE has RSA-3072 keys and certificates signed with RSASSA-PSS (SHA-256, MGF1-SHA-256, salt length 32). The signature cert has Key Usage `digitalSignature`, the encryption cert `keyEncipherment`. The spec also says the issuer certificates use RSASSA-PSS (TOKEN S.12).
- Real taxpayer Certificates come from Mein ELSTER; the spec only describes the file (TOKEN S.7, S.13).

### R1. Build the plain ElsterXML

- Encoding: `<?xml version="1.0" encoding="utf-8"?>` expected (EDS S.8). **[probe]** The prolog must carry the `encoding` attribute; a body `<x/>` was answered with `010025005 ... Der Encoding Parameter wurde im Prolog nicht gesetzt` (Fehlerliste: Komponente 01, Unterkomponente 002, Klassifizierung 5, Nummer 005).
- Allowed characters: only a subset of ISO-8859-15 (listed `xs:pattern`, EDS S.8-S.9).
- Root `<Elster xmlns="http://www.elster.de/elsterxml/schema/v11">`. No other namespace may be set in `Elster`, `TransferHeader`, `NutzdatenHeader` (EDS S.9). The Datenabholung `Nutzdaten` element sets its own namespace `http://finkonsens.de/elster/elsterdatenabholung/v3`, `version="32"` (SSb-DA S.8).
- `TransferHeader` children, in this order (XSD `th000011_extern.xsd`, `TransferHeaderCType`): `Verfahren`, `DatenArt`, `Vorgang`, `TransferTicket?`, `Testmerker?`, `SigUser?`, `Empfaenger?`, `HerstellerID`, `DatenLieferant`, `EingangsDatum?`, `Datei`, `RC?`, `VersionClient?`, `Zusatz?`. Attribute `version="11"` (EDS S.13).
- For the Datenabholung: `Verfahren` = `ElsterDatenabholung`; `DatenArt` ∈ {`PostfachStatus`, `PostfachAnfrage`, `PostfachBestaetigung`, `Statusabfrage`, `ElsterVaStDaten`}; `Vorgang` = `send-Auth` (SSb-DA S.5). The combinations marked "offene Schnittstelle" are in `Dokumente/Verzeichnis_der_Datenarten.csv` (for example `ElsterDatenabholung;PostfachStatus;send-Auth`).
- `HerstellerID`: exactly 5 digits (`\d{5}`, XSD `HerstellerIDSType`). Must be requested at the Bayerisches Landesamt für Steuern (SSb-DA S.6, XSD). See Open point 1.
- `DatenLieferant` (TransferHeader): at most 256 characters before encryption (XSD, `DatenLieferant*SType` documentation).
- `Datei`: `Verschluesselung` ∈ {`CMSEncryptedData`, `CMSEnvelopedData`}; `Kompression` = `GZIP`; `TransportSchluessel` (EDS S.17-S.18, XSD `VerschluesselungSType`). `PKCS#7v1.5` and `PKCS#7v1.5enveloped` are deprecated (EDS S.2, S.18). `EnvelopedData;RSA-OAEP;AES-128;GZip;B64` is "noch nicht unterstützt" (EDS S.18).
- `DatenTeil` contains one `Nutzdatenblock` (several only for Sammellieferungen) with `NutzdatenHeader version="11"` (`NutzdatenTicket`, `Empfaenger`, optional `Hersteller`, optional `DatenLieferant`) and `Nutzdaten` (EDS S.16, S.20-S.22). For the Datenabholung the only valid `Empfaenger` is `<Empfaenger id="L">CS</Empfaenger>` (SSb-DA S.7). `NutzdatenTicket` is mandatory, `1` for single requests (SSb-DA S.7).
- Example requests for PostfachStatus, PostfachAnfrage and PostfachBestaetigung: `ElsterDatenabholung-v32.0.1.zip` → `Beispiele_TH11/Abholung-PostfachNachricht-Daten/*/` (decrypted illustration form; the `SigUser` in those examples shows the server-side post-check content, not what a client sends).

### R2. Compress, encrypt and Base64 each protected element

Applies to the *content* of `DatenTeil`, `SigUser`, `DatenLieferant` and `TransportSchluessel` (not the element tags) (EDS S.10-S.11, S.25; XSD `KompressionSType`, `VerschluesselungSType`). The server reads them "mit demselben Schlüssel" (XSD documentation of `DatenLieferant`).

For each element:

1. Serialise the plain content as UTF-8 bytes.
2. GZIP (RFC 1952) (EDS S.11).
3. Wrap in CMS and encrypt (R2a).
4. Base64 (RFC 4648) (EDS S.12). **[probe]** A single unwrapped line was accepted in the header elements (the server got far enough to decrypt `DatenLieferant` and `TransportSchluessel`).

What the plain content is:

- `DatenTeil`: the serialised children of `DatenTeil`, i.e. `<Nutzdatenblock>…</Nutzdatenblock>`. The docs say "Inhalt des Elements DatenTeil" (EDS S.10) but not whether the fragment needs an `xmlns` declaration or a prolog. See Open points.
- `SigUser`: the serialised `<Sig>…</Sig>` (R3).
- `DatenLieferant`: the string.
- `TransportSchluessel` (only with `CMSEnvelopedData`): the certificate. **[probe]** The server accepted the Base64 *text* of the DER certificate as the plain content. Raw DER bytes as plain content were rejected with `060015005 Fall konnte nicht erfolgreich entschluesselt werden (Zertifikatsfehler) … Das Einlesen des Abholzertifikats ist fehlgeschlagen`. XSD `TransportSchluesselSType` says "Base 64-codierter String des X509-Zertifikats", consistent with this. With `CMSEncryptedData` the element stays empty (EDS S.20; SSb-DA example requests).

**R2a. CMS EnvelopedData for the Clearingstelle (online request)** (EDS S.11):

- ContentInfo type `id-envelopedData` (1.2.840.113549.1.7.3), RFC 5652. EDS S.11 also says "Die CMS-Syntax muss konform zu RFC 2560 sein" (sic, probably a typo; see Open points).
- Random 256-bit session key. Content encrypted with AES-256-CBC (OID 2.16.840.1.101.3.4.1.42, IV in the algorithm parameters, PKCS#7 padding; the OID and padding are RFC 5652/standard CMS practice, the doc only says "AES-256-CBC").
- Session key wrapped for the recipient with RSAES-OAEP (OID 1.2.840.113549.1.1.7): hash SHA-256, MGF1 with SHA-256, `pSourceFunc` = `id-pSpecified` with empty string (RFC 4055). Mixing SHA-1 and SHA-256 is forbidden; SHA-1 only "falls dieser nicht unterstützt wird" (EDS S.27).
- Recipient key: the Clearingstelle certificate `frontendkey2024.p10.crt` (README-Schluessel; EDS S.11 says "3072 Bit"). **[openssl x509]** `CN=FrontendKey`, RSA-3072, valid 2024-06-07 to 2029-11-07, issuer `ElsterKryptoCA`, certificate signature RSASSA-PSS/SHA-256. Key Usage is only `digitalSignature` (not `keyEncipherment`); a library that enforces KU on the OAEP recipient would refuse it, so use the public key directly.
- RecipientInfo form: not specified. **[probe]** A `KeyTransRecipientInfo` v0 with `IssuerAndSerialNumber` of the Clearingstelle certificate, built by hand, was accepted (header decrypt succeeded). OpenSSL's `cms -encrypt` output (also issuerAndSerialNumber) was accepted too.
- "Es sind auch mehrere Empfänger möglich" (EDS S.11); not needed.

### R3. Build the signature (`SigUser`)

Vorgang `send-Auth` requires `SigUser` (EDS S.10, S.14). Content before encryption is `<Sig><dsig:Signature …/></Sig>` (EDS S.14, AUTH S.9-S.10). The signature is created **after** `DatenTeil` is compressed, encrypted and Base64-encoded, and covers that final element (EDS S.10; AUTH S.5).

Profile, from AUTH Tabelle 1 (S.5-S.6) and S.6-S.8:

| Part | Value |
| --- | --- |
| Prefix | `dsig` mandatory; `xmlns:dsig="http://www.w3.org/2000/09/xmldsig#"` |
| `Signature@Id` | `Sign1` for the first signature |
| Form | detached; one `Reference URI=""` |
| `CanonicalizationMethod` | `http://www.w3.org/TR/2001/REC-xml-c14n-20010315` (Canonical XML 1.0, no comments) |
| `SignatureMethod` | `http://www.w3.org/2007/05/xmldsig-more#sha256-rsa-MGF1` (RSASSA-PSS, RFC 6931). `…#rsa-sha256` (PKCS#1 v1.5) is listed as legacy, but AUTH S.7 says "RSA Signatur nach PKCS#1V1.5 ist nicht mehr zulässig". |
| `DigestMethod` | `http://www.w3.org/2001/04/xmlenc#sha256`; "Andere Hashverfahren als SHA-256 sind nicht zulässig" (AUTH S.6) |
| Reference transform 1 | `http://www.w3.org/2002/06/xmldsig-filter2`, `XPath` element in namespace `http://www.w3.org/2002/06/xmldsig-filter2` with `Filter="intersect"` and `xmlns:elsterDefaultNs="http://www.elster.de/elsterxml/schema/v11"`, expression `//elsterDefaultNs:DatenTeil` (AUTH S.7 sample) |
| Reference transform 2 | `http://www.w3.org/TR/2001/REC-xml-c14n-20010315` (AUTH S.9 sample) |
| `KeyInfo/X509Data` | one `X509Certificate` (Base64 DER of the **signature** certificate C.USR.AUT) or a sequence where exactly one has `basicConstraints cA=false` (AUTH S.6) |

Steps:

1. Canonicalise (c14n 1.0, inclusive) the `DatenTeil` element as the filter-2 node set (the element with its inherited default namespace declaration, its text, nothing else). SHA-256 → `DigestValue` (Base64).
2. Canonicalise `SignedInfo` (c14n 1.0). Sign with the SK.USR.AUT RSA key: RSASSA-PSS, SHA-256, MGF1-SHA-256 (TOKEN S.19; for tokens `CKM_RSA_PKCS_PSS` with `CKM_SHA256`/`CKG_MGF1_SHA256`, TOKEN S.19). Result → `SignatureValue` (Base64).
3. Put the certificate in `KeyInfo`.
4. Serialise `<Sig>…</Sig>` and encrypt as in R2 → `SigUser` content.

Limits: signature over the encrypted `DatenTeil` is accepted up to 5 MB (AUTH S.8). Overall XML limit 50 MB, 15 MB when authenticated (EDS S.20).

The PSS **salt length** is not stated anywhere in AUTH (only "RFC 6931"). See Open points.

### R4. Assemble and POST

- Assemble the XML with the encrypted values in place (R1 order). Do not change `DatenTeil` after signing (not even whitespace inside it, since the signature covers its exact text).
- HTTPS `POST`, body = the XML (EDS S.12). URLs: `https://datenannahme1.elster.de:443/ERiC_IAS/ERiClet` … `datenannahme4`; "Die Verteilung und Zuordnung auf die einzelnen 4 URLs muss vom Sender sichergestellt werden" (EDS S.12). Optional suffix `/ElsterAnmeldung` or `/ElsterLohn`; no suffix is documented for `ElsterDatenabholung`.
- **[probe]** All four names resolve to the same address today (`datenannahme.elster.de` is the canonical name). `Content-Type: text/xml` was accepted; the docs do not prescribe one.
- **TLS** (details in "Transport security"): TLS 1.3 negotiated (`TLS_AES_256_GCM_SHA384`), server key RSA-4096; the server **requires a client certificate**.

## Ordered steps: response

### P1. Read the HTTP response

- **[probe]** Status is `200` even for rejections. Headers seen: `x-elster-transferticket`, `ERIC_XMLLEN`, `Content-Type: text/xml`, a `JSESSIONID` cookie. The `TransferTicket` is also inside the XML (EDS S.14-S.15, S.19).
- **[probe]** The declared encoding varied: a structural rejection came back `encoding="ISO-8859-15"`, requests that parsed came back `encoding="UTF-8"`. Honour the prolog.

### P2. Check the TransferHeader result

- `RC/Rueckgabe/Code` = `0` in the `TransferHeader` and in the `NutzdatenHeader` means success. A `TransferHeader` error returns an empty `DatenTeil`; a Nutzdaten error returns `TransferHeader` code 0, a non-zero code in the `NutzdatenHeader` and empty `Nutzdaten` (EDS S.7).
- `RC/Stack` carries the same shape; **[probe]** it was empty in the error cases seen.
- The server does not return `SigUser` (EDS S.10).
- **[probe]** The returned `DatenLieferant` (and `TransportSchluessel` if you sent one) are returned encrypted too, in the same mode as the request, so they decrypt with the same routine.

### P3. Decrypt according to `Datei/Verschluesselung`

- `CMSEncryptedData`: CMS `EncryptedData` (RFC 5652 §8), AES-256-CBC with the **session key of the request** (EDS S.11). **[probe]** Verified for `DatenLieferant`: ContentInfo `pkcs7-encryptedData` (1.2.840.113549.1.7.6), version 0, content type `data`, algorithm AES-256-CBC, IV in the algorithm parameter, PKCS#7 padding, decrypts with the request key (OpenSSL: `cms -EncryptedData_decrypt -secretkey`). The bytes are **BER with indefinite lengths** (`30 80 …`) and the encrypted content is a constructed/chunked OCTET STRING, so a DER-only parser fails (a DER-only ASN.1 decoder raised "Indefinite length encoding not supported").
- `CMSEnvelopedData`: ContentInfo `envelopedData`; encrypted with a fresh AES-256 key wrapped to the public key from the request's `TransportSchluessel`, RSAES-OAEP with SHA-256/MGF1-SHA-256, empty label; the doc says the key is a "2048 Bit" key (EDS S.11), the SoftPSE key is 3072 bit (**[probe]**). Decrypt with SK.USR.ENC (the `encryptionkey` KeyBag) (TOKEN S.19). **[probe]** The server returned `issuerAndSerialNumber` as recipient (issuer `ElsterSoftOrgTestCA`), key encryption `rsaesOaep`, content `aes-256-cbc`; OpenSSL decrypted it with the PSE encryption key. This was with Testmerker 700000001, not the 300000002 that the docs say forces OAEP (SSb-DA S.5-S.6), so RSA-OAEP is already the default for the soft-PSE test certificate.
- Older `PKCS#7v1.5enveloped` replies (RSA 1.5 + 3DES, OID 1.2.840.113549.3.7 `DESede/CBC/PKCS5Padding`) exist for signature cards and ElsterStick only (SSb-DA S.25; TOKEN S.20). Not needed for a soft-PSE.

### P4. Decompress, parse, validate

- GZIP-decompress, parse the `Nutzdatenblock` XML. Validation: the `NutzdatenHeader` RC code must be `0`; Datenabholung result structures are in `datenabholung_32.xsd` and `elster11_datenabholung_32.xsd` (in the SSb-DA zip, `Schemata/`). The header schemas (`th000011_extern.xsd`, `ndh000011.xsd`) validate headers only (`kleine_Uebersicht.txt`).
- No signature on the response is documented (TOKEN/AUTH describe only request signing; "Bei der Rückantwort vom Server wird das Tag <SigUser> nicht mehr übermittelt", AUTH S.5). Authenticity of the reply rests on TLS plus the encryption to the client's own key.
- Large attachments are not part of this response: they come from the Objektspeicher (`ReferenzId`, SSb-DA S.18); OTTER also authenticates with a TLS client certificate and requires TLS 1.3 (OTTER S.6-S.7). That flow belongs to ticket 02/other tickets.

### P5. Error reporting

- Codes are 9 digits = Komponente (2) + Unterkomponente (3) + Klassifizierung (1) + Nummer (3). Mapping verified against `Fehlerliste.xml` (elements `Komponente`, `Unterkomponente`, `Klassifizierung`, `Nummer`, `Text`, namespace `http://www.elster.de/fehlerliste/v1`):
  - `010025005` → 01/002/5/005 "Es konnte keine Versionsnummer aus dem uebermittelten TransferHeader ermittelt werden."
  - `060015005` → 06/001/5/005 "Fall konnte nicht erfolgreich entschluesselt werden (Zertifikatsfehler)."
  - `601015001` → 60/101/5/001 "Es ist ein Fehler bei der Verfahrenspruefung aufgetreten -" (the server appends detail text after the list text; see Open point 1).
- The meaning of the `Klassifizierung` digit (values seen in the file: 0, 1, 3, 5, 7, 9) is not documented in the files provided.
- Fachverfahren-specific codes (for example `190015302 Keine Daten vorhanden`, `190017112`) are in the SSb-DA (S.23-S.24).
- The server may add detail text that is not in the list (**[probe]**: the combination and HerstellerID messages below), so surface the server's `Text`, then look up the code.

## Test versus live

- No `Testmerker` element = "echte/produktive Daten" (EDS S.18; XSD `TestmerkerSType`). Test requests need a `Testmerker` and "dürfen nur Testdaten übermittelt werden (Test-IdNr, Test-Steuernummer, etc)" (SSb-DA S.6).
- The test certificates in `Test_Zertifikate.zip` "können nur Fälle mit Testmerker an den Server authentifiziert verschickt, bzw. abgeholt werden. Werden Fälle ohne Testmerker versendet, so wird ein serverseitiger Fehler provoziert." They map to a test account, so the signature verifies positively but they do not work for the ElsterOnline portal (Test-Zertifikate `readme.txt`). They "dürfen nicht weitergereicht werden und nur zu Tests durch den Softwarehersteller verwendet werden" (same readme). **Do not commit them to the public repo.**
- Same endpoints for test and live; there is no separate test host in the docs (EDS S.12). Testmerker selects test routing.
- Datenabholung Testmerker per docs: `700000001`, `700000004` (read from the test database), `300000002` (reply encrypted with RSA-OAEP, only with `send-Auth`) (SSb-DA S.5-S.6). **[probe]** With the `extern-offen` client certificate, `ElsterDatenabholung / PostfachStatus / send-Auth` was **accepted only with `700000001`**. With `700000004`, `300000002`, `300000001`, `700000002`, `700000003`, `700000005` the server answered `601015001 … Die Kombination aus Transferheaderversion: 11, Verfahren: ElsterDatenabholung, DatenArt: PostfachStatus, Vorgang: send-Auth, Testmerker: <n>, Ziel: , Eingang: elster-offeneschnittstelle ist fuer ihr Client-Zertifikat mit CN: extern-offen nicht zugelassen.` So the server maps the TLS client certificate to a list of permitted (Verfahren, DatenArt, Vorgang, Testmerker) combinations and the docs' Testmerker list does not match that for this certificate.

## Transport security

- **Server certificate / pinning.** README-HTTPS (`datenannahme.elster.de-public.crt`) says it is for checking the Clearingstelle identity when ERiC is not used. **[openssl x509]** That file is `CN=*.elster.de`, RSA-4096, issued by `D-TRUST SSL Class 3 CA 1 2009`, **expired 2025-03-09**. **[probe]** The live server today presents a different `*.elster.de` certificate (RSA-4096, valid 2026-09-18 to 2027-04-04) issued by `D-TRUST BR CA 2-23-1 2023` under `D-TRUST BR Root CA 2 2023` (cross-signed by `D-TRUST Root Class 3 CA 2 2009`). OpenSSL verified it against the system trust store. Pinning the shipped certificate would fail today, and leaf pinning would break on every renewal. The practical options are system trust store, or pinning the D-TRUST CA/root (policy decision, see Open points).
- **Client certificate.** README-Client (`liesmich.txt`, `kleine_Uebersicht.txt`): `extern-offen.p12` "enthält das Zertifikats/Schluesselpaar um sich gegenüber dem Datenannahmeserver der Finanzverwaltung als Client der offenen Schnittstelle auszuweisen", password `change_me`, valid until 2029-04-15. **[openssl]** RSA-4096, `CN=extern-offen`, issuer `root-edako-ca`, EKU TLS Web Client Authentication, PKCS#12 with PBES2/AES-256 + SHA-256 MAC. **[probe]** Without a client certificate the server aborts the handshake with TLS alert 116 `certificate required`; with `extern-offen.p12` the request reaches the application. The server's list of accepted client CAs includes `CN=extern-offen`, `CN=offeneschnittstelle`, `CN=root-edako-ca`, `CN=ERiC Client Certificate` and others. This client certificate is public (shared by every open-interface client), so it identifies the *software channel*, not the taxpayer; the taxpayer is identified by the XML signature.
- The EDS (S.12) mentions only "https" and `POST`; it says nothing about mutual TLS. The requirement is documented only in the two READMEs and visible in the handshake.

## Primitives needed

For the Go-vs-Rust library ticket (05):

1. PKCS#12 reading: PBES2 (PBKDF2-HMAC-SHA256 + AES-256-CBC) for key bags **and** legacy `pbeWithSHA1And40BitRC2-CBC` (and per spec `pbeWithSHAAnd3-KeyTripleDES-CBC`) for certificate bags; PKCS#12 MAC SHA-1 (100000 iterations) and SHA-256; bag attributes `friendlyName` and `localKeyID`; the same for loading `extern-offen.p12` (PBES2/AES-256, MAC SHA-256).
2. X.509 parsing including certificates signed with RSASSA-PSS (SHA-256, MGF1-SHA-256, salt 32) and RSA-3072/4096 keys; extraction of issuer DN and serial (for CMS `IssuerAndSerialNumber`). Chain validation of the user certificate is not needed client-side.
3. RSA-OAEP encryption with SHA-256 / MGF1-SHA-256 / empty label (RSA-3072 public key from a certificate whose KU lacks keyEncipherment) and RSA-OAEP decryption with the same parameters.
4. AES-256-CBC with PKCS#7 padding (encrypt and decrypt); secure random for key and IV.
5. CMS (RFC 5652) **generation** of `EnvelopedData` with a `KeyTransRecipientInfo`, `RSAES-OAEP-params` (RFC 4055) and AES-256-CBC content.
6. CMS **parsing/decryption** of `EnvelopedData` and `EncryptedData`, including **BER** (indefinite-length, constructed OCTET STRING content), with an externally supplied symmetric key for `EncryptedData`.
7. RSASSA-PSS signing with SHA-256, MGF1-SHA-256, salt 32 (assumed, see Open points) using a key loaded from PKCS#12.
8. XML Signature assembly: Canonical XML 1.0 (inclusive, no comments), for an element subtree with inherited namespace declarations; SHA-256 digest; Base64. XPath Filter 2.0 only needs to be *written* as a fixed template; the client does not have to evaluate it (the node set equals the `DatenTeil` subtree).
9. GZIP (RFC 1952) compress and decompress; Base64 (RFC 4648) encode and decode.
10. XML build and parse (namespaces, UTF-8 and ISO-8859-15 input decoding, restricted character set); optional XSD validation of headers.
11. HTTPS client with TLS 1.2/1.3, SNI, **client certificate from PKCS#12 (RSA-4096 key)**, system trust store or custom pins.
12. SHA-256, SHA-1 (PKCS#12 MAC); for fingerprints in `send-NoSig` only: MD5 (not needed for `send-Auth`, SSb-DA S.8).

## Open points

1. **HerstellerID.** The example 74931 does not work. **[probe]** Server answer `601015001 … HerstellerID ist nicht zugelassen.` for 74931, 99999 and 12345 alike, with Testmerker 700000001. A registered HerstellerID is needed even to send test requests. The request form is behind the developer login (`https://www.elster.de/elsterweb/entwickler/antrag-hersteller-id/form` returns a login page); the forum article "HerstellerIDs und Testlauf" named in EDS S.5 is members only. Whether ELSTER issues a separate test HerstellerID could not be established from the public material.
2. **Signature never verified by the server.** The HerstellerID check fails first, so the XML-DSig details are untested: PSS salt length (AUTH never states it; I used 32 = digest length, same as the PSS parameters in the ELSTER certificates), exact c14n context of `DatenTeil`, whether line breaks inside the Base64 `DatenTeil` text are tolerated, and whether the `Sig` fragment needs a namespace declaration.
3. **Plain content of `DatenTeil` / `SigUser` before gzip.** Documented only as "Inhalt des Elements"; not stated whether an XML prolog, an `xmlns` declaration on the fragment, or whitespace is expected. The header elements I could test (`DatenLieferant`, `TransportSchluessel`) were read by the server; `SigUser` and `DatenTeil` parsing was not reached.
4. **EDS typo/ambiguity.** EDS S.11 says the CMS syntax must conform to "RFC 2560" (that is OCSP). RFC 5652 is almost certainly meant (footnote on the same page cites RFC 5652). The `DatenTeil`/`Datenteil` case in the XPath text (AUTH S.7 text vs sample) is inconsistent; the sample and EDS element name `DatenTeil` were used.
5. **Stale parts of the docs.** AUTH S.9-S.10 sample still shows `encoding="ISO-8859-1"` and `PKCS#7v1.5`; EDS S.14/S.20-S.25 examples still contain 3DES-era CMS blobs (the EDS change log says 3DES → AES in 2025-12, EDS S.2). TOKEN (2017) says RSA-2048 and 3DES/RC2 PKCS#12; the current test PSE differs (R0).
6. **Testmerker mismatch.** SSb-DA lists 700000004 and 300000002 for the Datenabholung; the server rejected them for the `extern-offen` client certificate (only 700000001 passed). Needs checking with a real HerstellerID and, possibly, with a different client certificate.
7. **Which TLS client certificate for production.** Only the shared `extern-offen.p12` is documented for the open interface; it is public and expires 2029-04-15. Whether ELSTER rotates it silently, and whether redistribution inside an open-source CLI is permitted, is not stated (relevant to ticket 04). The server also lists other client CAs (`ERiC Client Certificate`, `offeneschnittstelle`); their meaning is not documented.
8. **Server certificate trust policy.** The shipped TLS certificate is stale; the live chain changed to D-TRUST BR CA 2-23-1 2023. No document says what to pin. A CA/root pin or the system trust store is the realistic choice; ELSTER can rotate both without notice.
9. **Clearingstelle key rotation.** The file is named `frontendkey2024`; the README gives no rotation schedule or distribution channel besides the download package (valid until 2029-11-07).
10. **`Klassifizierung` digit** of the 9-digit error code is not explained in `Fehlerliste.xml` or the SSb-DA.
11. **`CMSEncryptedData` vs `CMSEnvelopedData` for the Postfach payload.** The transport reply choice is the client's (both verified decryptable with the keys in the Certificate or the request key). Separately, the Objektspeicher payload and the metadata `Datenpaket` content are encrypted for the taxpayer's *encryption* key (SSb-DA S.18, S.25 and ticket 02); that needs SK.USR.ENC RSA-OAEP decryption from the same PKCS#12, so a `.pfx` that lacks the `encryptionkey` bag (for example a signature card) is not enough.
12. **Which Verfahren URL suffix** (if any) belongs to `ElsterDatenabholung` is not documented (EDS S.12 lists only `ElsterAnmeldung` and `ElsterLohn`); the unsuffixed URL worked for header parsing.
