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
