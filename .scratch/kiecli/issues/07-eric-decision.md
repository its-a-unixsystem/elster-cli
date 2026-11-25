# ERiC for the Kontoabfrage

Type: grilling
Status: resolved
Blocked by: 03, 04

## Question

Given what ERiC requires, how does v1 handle the Kontoabfrage: use ERiC as an optional, user-supplied dependency for the Kontoabfrage only; move the Kontoabfrage to v2 and keep v1 ERiC-free; or take another route the research surfaced?

## Answer

**Option A, widened: v1 runs entirely on ERiC.** Sending (UStVA, ZM) in v2 is ERiC-only anyway, so ERiC is unavoidable; once it is there, nothing is duplicated in a self-built transport unless ERiC cannot do it. ADR: [ERiC for all ELSTER traffic, no self-built transport](../../../docs/adr/0001-eric-for-all-elster-traffic.md).

1. **Full ERiC.** PostfachAnfrage, Bestätigung and Kontoabfrage go through `libericapi`; Objektspeicher attachments through Otto (`libotto`, same package). No Offene Schnittstelle transport, no fallback.
2. **ERiC is mandatory.** No Postfach-only mode. Without ERiC every command fails with a dedicated exit code and a JSON error naming `eric install`.
3. **Platforms:** Linux x86_64 and macOS (`Darwin-universal` package). Linux arm64 is not supported (ELSTER ships no build); not documented as a feature. *Narrowed by [Go or Rust](08-language.md): Linux x86_64 only in v1, macOS a later effort.*
4. **`kiecli eric install [--version X] --accept-eric-terms [--sha256 H]`**, modelled on haben (`packages/elster/src/install.ts`): anonymous download from `https://download.elster.de/download/eric/eric_<major>/ERiC-<version>-<platform>.jar`, extract only the current platform into a versioned directory (OS data directory by default, configurable), switch only after a complete extract. Never implicit. A hand-unpacked ERiC directory also works. The ERiC directory is tooling, not state.
5. **Licence §5 duties:** `--accept-eric-terms` is mandatory; output links `lizenz.pdf` and the data-protection notice; a marker in the ERiC directory records acceptance; commands refuse to call ERiC without it.
6. **Integrity:** a SHA-256 per tested version is pinned in the CLI; redirects to other hosts are refused; `--sha256` admits an untested patch version within a supported major.
7. **Versions:** check `EricVersion()` at start; accept only majors ELSTER currently permits and we have tested (today 44, 45 once 45.2 ships; haben reports 43/44 blocked for sending at the April minimum-version change). Anything else fails naming found and supported versions.
8. **Binding:** hand-written declarations for the few ERiC and Otto functions used, loaded at runtime; nothing header-derived committed; no build-time dependency on the ERiC package.

Facts gathered in this session:

- ERiC downloads are anonymous (HTTP 200 without credentials). ERiC 44.3.6.0 exists for `Linux-x86_64` (496 MB), `Darwin-universal` (804 MB, contains `libericapi.dylib`, `libotto.dylib`, `lizenz.pdf`) and `Windows-x86_64`; no Linux arm64 package.
- haben (`../haben`, `packages/elster/src/eric.ts`) binds ERiC 43–45 via koffi with hand-written signatures, switches the `EricBearbeiteVorgang` signature on `EricVersion()` (transfer handle dropped in 44), runs ERiC in a one-shot worker process, uses Datenabholung v32 (ERiC ≥ 43.4) for PostfachAnfrage/Bestätigung and `OttoDatenAbholen(objektId, size, cert, pin, herstellerId, …)` for attachments. It does not use PostfachStatus.
- ERiC refuses even validation without an own HerstellerID (ERiC ≥ 39.4; haben docs).

Amended by [Testing strategy](15-testing-strategy.md): the last fact is narrower than stated. ERiC 44 refuses the blocked former test HerstellerID 74931 even for validation; any other five-digit ID validates locally (experiment recorded there). An own HerstellerID is needed for traffic to ELSTER, not for `ERIC_VALIDIERE`.
