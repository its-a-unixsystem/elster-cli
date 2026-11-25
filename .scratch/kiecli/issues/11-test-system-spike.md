# Test-system spike: ERiC end to end

Type: prototype
Status: open
Blocked by: 06, 08

## Question

Does ERiC work for us end to end? Build a throwaway call in the chosen language that loads ERiC and Otto at runtime (as decided in [ERiC for the Kontoabfrage](07-eric-decision.md)) and runs against the ELSTER test system with the test Organisationszertifikat and our own HerstellerID: PostfachStatus, a PostfachAnfrage, and one Otto download of an attachment. Record what it took and what surprised us.

Also settle: whether ERiC supports `PostfachStatus` at all (haben uses only PostfachAnfrage and Bestätigung); whether a Postfach-Nachricht is still listed and fetchable after Bestätigung; what an Otto download returns (plain, still encrypted, gzip; whether `EricDekodiereDaten` is needed); and how Abholung works on the test system, since the Objektspeicher documents no test host.

Per [Go or Rust](08-language.md) the spike is in Rust on Linux x86_64, binding ERiC and Otto through hand-written `libloading` declarations. Also record the glibc baseline ERiC needs (`objdump -T` over the `.so` files), which fixes the oldest glibc the release binary may target.

From [Agent interface](09-agent-interface.md), also check: whether a PostfachAnfrage naming every Bereitstellungsdatenart from SSb table 9.4 is accepted, including kinds the account never receives (if not, `anfrage` switches to PostfachStatus first); whether `einschraenkung="neue"` still lists a Postfach-Nachricht that has been fetched but not confirmed (state 20 ABGEHOLT); and which ERiC and Otto error codes appear for a wrong PIN, a missing HerstellerID and an OTTER 503, so the error codes map to the right exit codes.

From [Kontoabfrage requests and responses](13-kontoabfrage-requests.md), also check: one `O` Kontoabfrage with Testmerker `230000001` (the Kontoabfrage's own, not the Postfach's); which test Steuernummern the test system accepts (the examples use Finanzamt 9238); whether ERiC exports `EricMakeElsterStnr` for the Bescheid-to-ELSTER conversion; whether the test Organisationszertifikat gets a result or `230077012`; and which response shape the test Finanzamt returns. For `SKILL.md`, find out how a real account clears `230077012`/`950017011` (Vollmacht or Freischaltung in Mein ELSTER).

From [Certificate inspection](14-certificate-inspection.md), also check: what `EricHoleZertifikatEigenschaften` returns for the test certificates (both blocks present, `Testzertifikat` true, `Registrierertyp` `Organisation` for the test Organisationszertifikat, subject CN shape); and that a test certificate sent live is rejected as expected, so `test_certificate_requires_testmerker` guards a real failure.

From [Testing strategy](15-testing-strategy.md), also check: whether the test Postfach is shared by every Hersteller using the test certificates, and so whether `postfach abholung --bestaetigen` and `bestaetigung` are safe in recurring test-system runs; whether an Abholung on the test system counts toward the 24 h Bestätigung rule for our HerstellerID; and whether GitHub Actions runners (`ubuntu-latest`) can reach the test system (IP blocks, rate limits). Also note that the 44.3.6.0 package's `libericapi.so` reports version 44.3.4.0 in `EricVersion`.

When resolving, also fill the ⏳ items in [spec.md](../spec.md) (its "Pending the spike" table names where each answer lands).

## Comments

### Pre-spike findings (2026-11-19, no HerstellerID needed)

From the anonymous `ERiC-44.3.6.0-Linux-x86_64.jar` (521,062,399 bytes, SHA-256 `e36df8b02045ff3449eb23e086a39f5455b709c967e708eae44f27099ce4de95`) and `ERiC-44.3.6.0-Dokumentation.zip` (SHA-256 `bad21c27ecce56d04fc04bccfd2dfa17b9ff2455aa878758100fc73a28492ad5`), downloaded outside the repo.

- **glibc baseline: 2.25.** `objdump -T`: `libericapi.so` and `libotto.so` need `GLIBC_2.25`, `libeSigner.so` 2.17, `libericxerces.so` 2.14, all 221 plugins 2.17. No `GLIBCXX`/`CXXABI` symbols, so libstdc++ is linked statically; `libgcc_s.so.1` is required (`NEEDED` of `libericapi`, `libotto`, `libericxerces`). The release binary must not target a glibc older than 2.25.
- **`EricMakeElsterStnr` is exported** (plus `EricMtMakeElsterStnr`): `(steuernrBescheid, landesnr[3], bundesfinanzamtsnr[5], rueckgabepuffer)`. One of Landesnummer or Bundesfinanzamtsnummer is required; for Bavarian and Berlin numbers in `BBB/UUUUP` format the Bundesfinanzamtsnummer is mandatory. It also validates the result (`ERIC_GLOBAL_STEUERNUMMER_UNGUELTIG`, `ERIC_GLOBAL_LANDESNUMMER_UNBEKANNT`). `EricFormatStNr`, `EricPruefeSteuernummer` and `EricPruefeBuFaNummer` are exported too.
- **PostfachStatus is supported.** The `libcheckElsterDatenabholung.so` plugin knows `PostfachStatus_32`, `PostfachAnfrage_32` and `PostfachBestaetigung_32`. `ElsterDatenabholung-v32.pdf` § 6.2.1 documents it as the per-Bereitstellungsdatenart hit count without Abholung, with the same `einschraenkung`/`von` filters as PostfachAnfrage, and answers split by `rolle` (`inhaber`/`vertreter`) in `PostfachStatusAntwort`. It is a request type sent through `EricBearbeiteVorgang`, not a separate API function. Still to see on the test system: whether it behaves as documented.
- Plugins on the v1 path: `libcheckElsterDatenabholung.so` and `libcheckElsterKontoabfrage_6.so` (plus `libcheckKontoinformation.so`, role unknown).
- `EricHoleZertifikatEigenschaften` needs the PIN for software certificates; details in [Certificate inspection](14-certificate-inspection.md).

### Step 1: ERiC offline (2026-11-25)

Throwaway Rust crate (`libloading` 0.8, edition 2024, rustc 1.89) loading `libericapi.so` from the unpacked `ERiC-44.3.6.0-Linux-x86_64.jar` (SHA-256 matches the pin). No network. Code stays outside the repo.

- **Loading:** `libericapi.so` has `RPATH $ORIGIN`, so `dlopen` by absolute path finds `libericxerces.so` and `libeSigner.so` without `LD_LIBRARY_PATH`. `EricInitialisiere(<home>/lib/plugins, <logdir>)` returns 0 and writes `eric.log` into the log directory.
- **`EricVersion` before `EricInitialisiere`** returns `610001081` (calling `EricHoleFehlerText` before init was not tried). After init, `libericapi.so` reports `Produktversion 44, 3, 4, 0`, as noted above.
- **`EricGetHandleToCertificate`** returns handle 1 and `iInfoPinSupport` `0x17` for every test file. A missing path gives `610201105` ("Ungültiger PSE Pfad"), which falls under `certificate_unreadable` (the `ERIC_CRYPT_*` range) in spec §8.1. The spec may want a dedicated `certificate_not_found` checked locally before ERiC.
- **Wrong PIN:** `EricHoleZertifikatEigenschaften` returns `610201106` (`ERIC_CRYPT_E_PIN_WRONG`), as §8.1 assumes. So the PIN is checked locally, before any request.
- **`EricHoleZertifikatEigenschaften`** on the three `.pfx` test certificates: both blocks present, identical apart from the key hashes; `Testzertifikat` `true`; `TokenTyp` `Software`; `Verifikationsart` `Postweg`; RSA-PSS/SHA-256, 3072 bit; valid 2025-07-10 to 2028-07-10. Subject has only `CN`, a 10-digit Konto-ID.

  | File | Registrierertyp | Identifikationsmerkmaltyp | Issuer CN |
  |---|---|---|---|
  | `test-softorg-pse.pfx` | `Organisation` | `Steuernummer` | `ElsterSoftOrgTestCA` |
  | `test-soft-pse.pfx` | `Person` | `Steuernummer` | `ElsterSoftTestCA` |
  | `test-softidnr-pse.pfx` | `Person` | `Identifikationsnummer` | `ElsterSoftTestCA` |

- **Date format differs from ticket 14:** `AusgestelltAm`/`GueltigBis` come as GeneralizedTime `YYYYMMDDhhmmssZ` (`20280710092311Z`), not the UTCTime `YYMMDDhhmmssZ` of the header example quoted in [Certificate inspection](14-certificate-inspection.md). The parser must take the 14-digit form, and should accept both.
- **`eric-zertifikate-bescheidabholung/PSE/eric_private.p12`** is a legacy file: it expired 2020-04-16, SHA-1, its subject CN is the CA name, every type field is `Unbekannt` and `Testzertifikat` is `false`. KIECLI would refuse it as expired (exit 3) before `test_certificate_requires_testmerker` comes into play. Not useful for the spike, so leave it out of the test-system runs.

### Step 2: PostfachStatus and PostfachAnfrage on the test system (2026-11-25)

Same crate, now with `EricBearbeiteVorgang` (ERiC ≥ 44 signature, `ERIC_VALIDIERE | ERIC_SENDE`, `eric_verschluesselungs_parameter_t` version 3, no print parameters), `send-Auth` with `test-softorg-pse.pfx`, Datenartversion `PostfachStatus_32` / `PostfachAnfrage_32`, and our HerstellerID about an hour after it was assigned.

- **The HerstellerID works right away.** No activation delay was seen. Every call got a `TransferTicket`.
- **Testmerker:** `700000001` works for PostfachStatus and PostfachAnfrage. `700000004` (haben's choice, listed as equal in SSb 5.1.1.4) is refused for both with `601015001`: "Die Kombination aus … Testmerker: 700000004, Ziel: CS, Eingang: eric ist fuer ihr Client-Zertifikat mit CN: extern-eric nicht zugelassen". Through ERiC the Postfach Testmerker is `700000001` only.
- **Server-side failures come back as ERiC `610101210`** ("Fehler im Transferheader … Bitte werten Sie die Serverantwort aus"), with an empty Rückgabe buffer. ELSTER's own code and text are in the `RC` of the server answer's TransferHeader (`Rueckgabe` and `Stack`). Errors in the Nutzdaten show two codes: `190010801` (generic) in `Rueckgabe` and the specific one in `Stack`. KIECLI must read the `RC` to map exit codes; `610101210` alone says nothing.
- **The test Postfach is shared.** The test Organisationszertifikat's Postfach answers with a single `accountId` but holds **807,144** entries, and about 300 new protocols arrive per day (`KMVProtokoll` from many unrelated test submissions, `AenderungslisteDUe` for 4 different Steuernummern). Every Hersteller sending with `test-softorg-pse.pfx` fills it. An unfiltered PostfachStatus is refused with `190017608` ("… 807144 Datensätze gefunden, es können maximal 500000 zurückgegeben werden"). Test-system calls must always pass `von`.
- **PostfachStatus works but undercounts.** With `von=20261124` it reported `KMVProtokoll` 299, `ELOProtokoll` 4 and `OVMVRM` 1, matching the Anfrage, but it left out `AenderungslisteDUe`, which the Anfrage returned 71 times. So `postfach status` is a hint, not a count to rely on.
- **Table 9.4 cannot be sent as is.** A PostfachAnfrage naming all 73 kinds from SSb table 9.4 is refused. The server names one offending kind per answer, so the request was resent without it until accepted (34 rounds). Refused:
  - `190010802` "ist in der EDS-Konfig fuer die Version nicht freigegeben" (same answer as an invented name): `AbrufcodeEMail`, `DivaStundungZinsVS`, `ISAKKAufteilung`, plus `DIVAAnzeigeEinerLotterie` and `DIVATroncabgabe` from SSb 9.2/9.3.
  - "ist fuer die Version 32 fuer den Eingang eric nicht freigegeben": `AntwortCoronaSoforthilfe`, `DEICH`, `ESSGS`, `ESTGS`, `EURS`, `FEIGS`, `GELF`, `GEWGS`, `GEWTP`, `GEWXX`, `GewStZerlegungGemeinde`, `GMB`, `GMBAE`, `GMBAX`, `GMBV`, `GMBVX`, `GMBX`, `GRSTE`, `Grundsteuerhebesatz`, `KISTM`, `KSTGS`, `KV`, `RDAL`, `StAbEntlastungVA`, `SV`, `USTGS`, `UStVEUVA`, `UVVGS`, `VERSVA`, `VWDVG`, `WPMBR`.
- **Accepted through ERiC, 49 kinds,** all in one request:
  - from table 9.4: `ANAuskunft`, `AenderungslisteDUe`, `AnhangRueckmeldung`, `DIVABescheid`, `DIVALotteriesteuerEinmal`, `DivaAdvMitteilung`, `DivaAdvVA`, `DivaAdvZinsVA`, `DivaBescheidESt`, `DivaBescheidErbSt`, `DivaBescheidGewSt`, `DivaBescheidKSt`, `DivaBescheidSchenkSt`, `DivaEinwilligungRM`, `DivaErlassMitteilung`, `DivaErlassVA`, `DivaSonstigeMitteilung`, `DivaSonstigerVA`, `DivaStundungVA`, `EBilanzRueckmeldung`, `ELOProtokoll`, `EPAntwort`, `EPAntwortKopie`, `EPBescheid`, `EPKurzmitteilung`, `EPMitteilung`, `ESB`, `GEWAbmeldung`, `GEWMB`, `Gewerbesteuerbescheid`, `GSB`, `KDBVollmachtRM`, `KMVProtokoll`, `LStHVVMRM`, `OVMVRM`, `OZGMitteilung`, `USB`, `VDBKDBRueckmeldungLief`;
  - not in table 9.4 but named in SSb 9.2/9.3: `DivaBescheidUSt`, `DivaBescheidFEIN`, `OZGBescheid`, `DIVABuchmachersteuer`, `DIVALotteriesteuerMehr`, `DIVASportwettensteuer`, `DIVATotalisator`, `VerbindlicheAuskunft`, `VerbindlicheZusage`, `DIVAVirtuelleAutomatenStB`, `DIVAOnlinePokerStB`.
- **Anfrage answer shape:** every `DatenartBereitstellung` named comes back with `anzahltreffer` (also 0). Each `Bereitstellung` has `id`, `bereitstellungsticket`, `groesse`, `MetaInformationen` (`Meta name=…`, e.g. `verfuegbarbis`, `schemaversion`, `lieferticket`, `accountId`, `kmArt`, `eingangsTS`, `ag-steuernummer`, `fehlerliste`) and a `Datenpaket` with `ReferenzId` (a UUID, the Otto object), `Encoding` and `Groesse`. No `Anhang` appeared in the window (no DIVA PDF since 2026-11-24). The answer for 375 Nachrichten was 307 KB.
- The Rückgabe buffer on success holds only `<EricBearbeiteVorgang><Erfolg/></EricBearbeiteVorgang>`. Everything useful is in the server answer.

### Step 3: Otto Abholung and Bestätigung (2026-11-25)

One Nachricht only: the one `OVMVRM` listed under `einschraenkung="neue"` since 2026-11-24 (`groesse` 1372). Everything else in the window was already confirmed by other Hersteller, including the 299 `KMVProtokoll` from the same day. Fetched at 16:32:59Z, confirmed 13 s later.

- **Otto works on the test system without anything extra.** `OttoInstanzErzeugen(logPfad, NULL, NULL, &instanz)`, then `OttoDatenAbholen(instanz, ReferenzId, Groesse, pfxPath, PIN, HerstellerID, NULL, puffer)` returned 0. There is no Testmerker parameter; the Objektspeicher evidently goes by the test certificate. No host or proxy setting was needed.
- **Otto returns plaintext.** 626 bytes of UTF-8 XML (`<OVMVRM version="1" xmlns="http://finkonsens.de/elster/elsterbereitstellung/ovmvrm/v1">`), not gzip, not CMS. `EricDekodiereDaten` on those bytes fails with `610201108` ("Fehler beim Lesen des PKCS#7-Objekts"), so it is not part of the path. The Anfrage's `Groesse` (1372) is the transport size, not the delivered size.
- **`neue` and state 20:** the Nachricht was listed under `neue` before the Abholung and still afterwards (fetched, not confirmed). After the Bestätigung it was gone from `neue` and still in `alle`.
- **After Bestätigung** it could be fetched again through Otto, with identical bytes. It stays available until `verfuegbarbis` (here `20270524`).
- **Bestätigung answers:**
  - a valid id: ERiC 0, per id `0` "Der Empfang der Daten wurde erfolgreich bestaetigt.";
  - the same id again: the same success, so a repeated Bestätigung is harmless;
  - an unknown id: ERiC `610101292` ("Fehler im NutzdatenHeader … Bei Sammeldaten sind alle Nutzdatenblöcke zu prüfen"), ELSTER `190015302` "Keine Daten vorhanden." in the NutzdatenHeader `RC` and per id;
  - a valid and an unknown id in one block: ERiC `610101292`, NutzdatenHeader `190017112` ("Der Empfang von mindestens einer der Daten wurde nicht erfolgreich bestätigt."), per id `0` for the valid one (confirmed) and `190015302` for the unknown one. Per-id results must be read even when ERiC reports an error.
- **Safe pattern for recurring test-system runs:** take a Nachricht that is in `alle` but not in `neue` (already confirmed by someone), fetch it and confirm it. Nobody else sees a change. Confirming a `neue` one hides it from other Hersteller's `neue`. The question whether a test-system Abholung counts toward our 24 h obligation goes away if every run confirms what it fetched.
- Still open from this ticket: a DIVA Nachricht with `Anhang` (none for the test Organisationszertifikat in 30 days; the personal test certificates are untried); OTTER 503 and Otto error codes; missing HerstellerID and wrong PIN on the wire; the Kontoabfrage; a test certificate sent live; GitHub runner reachability.
