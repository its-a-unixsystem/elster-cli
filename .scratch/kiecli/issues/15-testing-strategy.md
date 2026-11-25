# Testing strategy

Type: grilling
Status: resolved
Blocked by:

## Question

How is KIECLI tested, from unit tests up to the ELSTER test system, and which parts run in CI? Settle:

- **Fixtures:** hand-written with invented data, because ELSTER's `Beispiele` may not be redistributed ([ELSTER terms for an open-source client](04-open-source-terms.md)). Decide where they live and how they are kept valid against ELSTER's schemas and ERiC's plugins.
- **The fake behind the `eric` module's trait** ([Go or Rust](08-language.md)), versus haben's C mocks. Decide what it must simulate: ERiC and Otto return codes, Otto streaming, a partial Abholung, PIN and certificate errors and certificate properties ([Certificate inspection](14-certificate-inspection.md)), and Kontoabfrage responses in both formats ([Kontoabfrage requests and responses](13-kontoabfrage-requests.md)).
- **Test certificates:** downloaded from `download.elster.de` at test time, never committed. Decide where they are cached.
- **ERiC in tests:** CI downloads it anonymously at test time, with the same pinned SHA-256 as `eric install`. ERiC refuses even local validation without a HerstellerID, so CI needs `KIECLI_HERSTELLER_ID` as a secret. Decide which jobs get it (fork PRs get no secrets) and what runs without it.
- **Test-system runs:** which commands run, with the Testmerker, and how often (manual, nightly, per release). Whether they can run in CI waits on facts from [Test-system spike: ERiC end to end](11-test-system-spike.md); the rest of the strategy does not.
- **Contract tests:** `describe` output, the JSON Schemas for stdout and errors, and the semver rule from [Agent interface](09-agent-interface.md) (removing or renaming a field is a major change).
- **The final live check** with a real Certificate: who runs it, when, against which checklist, and that nothing from it reaches the repo.
- **CI provider:** the repo has no remote yet.

## Answer

**Three layers: command logic against an in-process fake, the binding and our XML against real ERiC offline, and the ELSTER test system behind a gated job. Only the last needs the real HerstellerID: ERiC validates locally with any five-digit ID except the blocked 74931 (experiment below), so fork PRs run real ERiC too.** CI is GitHub Actions on [its-a-unixsystem/kiecli](https://github.com/its-a-unixsystem/kiecli).

### Fake and binding

1. **The `eric` trait sits at ERiC-call level:** `bearbeite_vorgang(xml, flags) -> raw response XML`, `otto_abholen(objektid, &mut Write)`, `zertifikat_eigenschaften(pin) -> raw XML`, `make_elster_stnr(..)`, `version()`. XML building and parsing stay in tested code; the fake returns fixture XML.
2. **The fake is scriptable per test** (a response or an ERiC/Otto code per call) and records every call, so tests assert the request XML, Testmerker and HerstellerID sent. It simulates ERiC and Otto return codes, Otto streaming in chunks, a failure after N bytes on object K (partial Abholung, exit 7), PIN and certificate errors, Certificate properties (from `EricHoleZertifikatEigenschaften` XML) and Kontoabfrage answers in biene and alt.
3. **Commands are tested in-process:** `main` is a thin shell around `run(args, env, &dyn Eric) -> (stdout, stderr, exit)`. The release binary has no fake mode and no hidden switch. `assert_cmd` tests on the real binary cover only what needs no ERiC: usage errors (exit 2), `describe`, `skill`, `config show`, `eric status` without ERiC (exit 4).
4. **No C mocks** (unlike haben). The `unsafe` binding is tested against real ERiC through its local functions: `EricVersion`, init and teardown, `EricHoleZertifikatEigenschaften` with the test certificates (PIN 123456), `EricMakeElsterStnr`, error texts, `ERIC_VALIDIERE`. The Otto binding has no offline path and is covered by test-system runs only.

### Fixtures

5. **`tests/fixtures/`**, by source: `elster/` (PostfachStatus, PostfachAnfrage and Bestätigung answers, Kontoabfrage biene and alt, Fehlerliste-shaped error answers), `eric/` (`EricHoleZertifikatEigenschaften`, `EricVersion`), `requests/` (golden request XML our builders must produce byte for byte).
6. **Hand-written with invented data:** test Finanzamt 9198/9238 Steuernummern, invented tickets and objektids, Konto-IDs only from a reserved invented range (prefix `TEST`).
7. **Kept valid:** response fixtures are validated against ELSTER's XSDs with `uppsala`, the schema zips downloaded at test time (item 9), never committed. Golden requests are validated by real ERiC (`ERIC_VALIDIERE`, nothing sent) with the placeholder HerstellerID.
8. **Test-system captures** may shape fixtures but are never committed verbatim: rewritten with invented identifiers (Konto-ID, Steuernummer, tickets, objektids, names) and reviewed before commit.

### Downloads

9. ERiC jar, `Test_Zertifikate.zip` and the schema zips come from `download.elster.de` at test time, each pinned by SHA-256 in one checked-in manifest `tests/downloads.toml`; a mismatch fails loudly. Cache: `$XDG_CACHE_HOME/kiecli-test/<sha256>/`. ERiC is installed there by our own `kiecli eric install --eric-dir …`, dogfooding the install path. CI caches the directory keyed on the manifest hash.
10. Tests needing a download skip when offline locally; `KIECLI_TEST_REQUIRE_DOWNLOADS=1` (set in CI) turns a skip into a failure.

### Contract tests

11. `describe` output and the stdout and stderr JSON Schemas are generated from the Rust types (`schemars`) and committed as `schemas/*.json`; a test fails when the generated files differ, so every interface change shows in review.
12. A semver test compares `schemas/` with the last release tag: a removed or renamed property, a changed type, a removed enum value or a removed error or warning code requires a major bump in `Cargo.toml`. New enum values (such as warning codes) stay minor; `SKILL.md` tells agents to tolerate unknown codes.
13. Every command output from the fake-driven tests is validated against its schema.

### CI (GitHub Actions)

| Job | When | Secret | Runs |
|---|---|---|---|
| `checks` | every push and PR, forks included | none | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (unit, fake-driven, contract, `assert_cmd`), `cargo deny check` (licences, RustSec) |
| `eric-local` | every push and PR, forks included | none | real ERiC from the cache: binding tests, golden requests validated with `PLATZHALTER_HERSTELLER_ID = "12345"`, response fixtures against the XSDs, canary asserting `74931` → 610301202, so an ERiC change that blocks placeholder IDs fails as one clear test |
| `test-system` | manual, release gate, weekly | `KIECLI_HERSTELLER_ID` | the test-system run (items 16–18) and the leak guard (item 19) |

14. Toolchain pinned in `rust-toolchain.toml`; no MSRV matrix, no coverage gate; `ubuntu-latest` only.
15. **Secret:** `KIECLI_HERSTELLER_ID` exists only as an environment secret, never a repository secret. Environment `elster-test`: deployments from `main` and release tags, the maintainer as required reviewer (manual and release runs). Environment `elster-test-scheduled`: `main` only, no reviewer (the weekly run). Never `pull_request_target`, never PR code checked out with the secret.

### Test-system runs

16. One entry point for a developer's machine and CI: `cargo test --features test-system`, HerstellerID from env, test certificates from the download cache, Testmerker on every request.
17. Always: `zertifikat status`, `postfach status`, `postfach anfrage`, all three `kontoabfrage` kinds, and the live-call refusal (`test_certificate_requires_testmerker`). `postfach abholung --bestaetigen` and `postfach bestaetigung` join only once [Test-system spike: ERiC end to end](11-test-system-spike.md) shows the test Postfach (likely shared by every Hersteller using the test certificates) is safe for them. Never `--ohne-bestaetigung` without a Bestätigung in the same run.
18. Triggers: `workflow_dispatch`, a required gate in the release workflow before artifacts are published, and a weekly `schedule` on `main`. Whether GitHub's runners can reach the test system at all is a spike fact; if not, the same command runs on the maintainer's machine before each release and the CI triggers are dropped.

### Leak guard

19. Whenever `KIECLI_HERSTELLER_ID` is set, a test scans every tracked file for that value and for the subject CN of the Certificate in `KIECLI_CERTIFICATE` (if set), and fails on a hit.

### Final live check

20. Run by the maintainer with their own Certificate, before 1.0.0 and before any release that changes the ERiC major or request XML, following a checked-in `docs/live-check.md` (commands and the expected shape of each result, no values). It includes one real `postfach abholung --bestaetigen` of an already-known Nachricht. `--out` points outside the repo; ERiC logs stay at their deleted-at-exit default. The only record is a ticked checklist in the release PR: CLI and ERiC versions, pass or fail per step, no output.

Spike checks added to [Test-system spike: ERiC end to end](11-test-system-spike.md). Amends [ERiC for the Kontoabfrage](07-eric-decision.md).

No ADR: nothing here is hard to reverse.

Amended by [Distribution](16-distribution.md): `checks` also verifies a DCO `Signed-off-by` on every commit; `cargo deny`'s licence allowlist must be AGPL-3.0-compatible; the release workflow adds a zigbuild step for glibc 2.25 with an `objdump -T` guard after the `test-system` gate.

## Comments

### Experiment: ERiC validation and the HerstellerID (2026-11-20)

ERiC 44.3.6.0 package (`libericapi.so` reports itself as 44.3.4.0; `libcheckElsterDatenabholung.so` 44.1.4.0), downloaded anonymously outside the repo, SHA-256 matching the pin in [Test-system spike: ERiC end to end](11-test-system-spike.md). `EricInitialisiere`, then the ERiC ≥ 44 `EricBearbeiteVorgang` with `ERIC_VALIDIERE` only, no certificate, no `EricSetzeEinstellung`. Requests with invented data: a PostfachAnfrage (`PostfachAnfrage_32`, Testmerker 700000004) and an `O` Kontoabfrage (`ElsterKontoabfrage_6`, Finanzamt 9238, Testmerker 230000001; `Kontoabfrage_6` is rejected as unknown with 610001042). Both behave identically:

| HerstellerID | Result |
|---|---|
| `12345`, `54321` | 0, `<Erfolg/>`; log shows `EbvCheckHerstellerAllowed … ERIC_OK` |
| `74931` | 610301202 `ERIC_IO_TESTHERSTELLERID_GESPERRT` ("Bitte verwenden Sie Ihre eigene Hersteller-ID auch für Testfälle.") |
| empty, missing, `1234`, `123456`, `abcde` | 610301200, base-schema violation (`\d{5}`, required element) |

- The blocklist check is in the ERiC core (`libericapi.so`), not in a plugin; the format check comes from the ELSTER base schema.
- Validation needs no network (same results inside an empty network namespace).
- Matches haben's real-ERiC suite, which validates with placeholder `12345` (`packages/elster/src/eric-echt.test.ts`).
