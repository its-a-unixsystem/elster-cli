# elster-cli v1

Label: wayfinder:map

## Destination

An implementation-ready v1 spec at `.scratch/elster-cli/spec.md` for a stateless, agent-oriented CLI that reads the Postfach (including Objektspeicher attachments and Bestätigung) and performs the Kontoabfrage for whichever Taxpayer the user supplies. Building it is a separate effort after this map.

## Notes

- Domain language: `GLOSSARY.md`. Use its terms in tickets and the spec.
- Source material: ELSTER developer downloads in `docs/API/` (gitignored, never committed). Unzip into a scratch directory to read; `pdftotext -layout` works on the PDFs. Key files: `ElsterXML-Schnittstelle_V11_*.zip` (transport, authentication, `Verzeichnis_der_Datenarten.csv` = which procedures are Offene Schnittstelle), `ElsterDatenabholung-*.zip` (Postfach), `ElsterKontoabfrage_*.zip`, `ElsterObjektspeicher_*.zip` (OTTER OpenAPI), `Test_Zertifikate.zip` (PIN 123456, test system only, not redistributable), `Fehlerliste.xml`.
- Public open-source repo: no personal data, real tax data, real certificates or the real HerstellerID in tickets, fixtures or code.
- Standing decisions from charting:
  - The CLI is a tool, not tied to any account: the user supplies Certificate, PIN, HerstellerID and Taxpayer data per call or via config. One Taxpayer per call.
  - Stateless: no local archive; the caller owns persistence.
  - Built for AI agents: JSON on stdout by default (`--format text` for humans), never interactive, stable exit codes, JSON errors on stderr, self-describing commands, ships a `SKILL.md`.
  - Read-only in v1. Bestätigung is in scope but only on explicit request, never implicit; `postfach abholung` forces the caller to choose `--bestaetigen` or `--ohne-bestaetigung` (see [Agent interface](issues/09-agent-interface.md)).
  - Usage policy (who may run it, unattended or not) is the caller's business.
  - Full ERiC: all ELSTER traffic goes through ERiC, attachments through Otto; no self-built transport unless ERiC cannot do something (ADR `docs/adr/0001-eric-for-all-elster-traffic.md`). ERiC is user-installed via `elster-cli eric install`, never in the repo or release artifacts.
  - Rust, Linux x86_64 only in v1 (macOS later, kept possible; no Linux arm64: ELSTER ships no ERiC for it), own `libloading` binding to ERiC and Otto (ADR `docs/adr/0002-rust-with-runtime-loaded-eric.md`). HerstellerID comes from config, never code.
  - Licence `AGPL-3.0-only` with a §7 additional permission for ERiC and Otto (ADR `docs/adr/0003-agpl-with-eric-exception.md`).
  - Develop and verify against the ELSTER test system with the test certificates; a real Certificate only for a final, explicit live check.
- Research findings live in `.scratch/elster-cli/research/`, not on research branches. Remote: [its-a-unixsystem/elster-cli](https://github.com/its-a-unixsystem/elster-cli) on GitHub.
- Reference implementation: haben (`../haben`, `packages/elster/src/eric.ts`, `install.ts`) binds ERiC 43–45 and Otto and downloads ERiC; consult it before designing ERiC handling.

## Decisions so far

<!-- one line per resolved ticket: [title](issues/NN-slug.md): gist -->

- [Postfach lifecycle and attachments](issues/02-postfach-lifecycle.md): Anfrage (ElsterXML) → Abholung (Objektspeicher `fetch`, signed objektid, HerstellerID headers) → Bestätigung within 24 h of Abholung or the manufacturer risks a block; retrieval after Bestätigung undocumented; Objektspeicher needs an unspecified TLS client certificate (moot since the ERiC decision: Otto handles the Abholung).
- [ERiC and the Kontoabfrage](issues/03-eric-kontoabfrage.md): Kontoabfrage is outside the Offene Schnittstelle by policy, not protocol; ERiC licence forbids bundling it in a public release (users must supply it, as erica/eric-rs/haben do); Rust has bindings, Go has none; macOS support unverified.
- [ELSTER terms for an open-source client](issues/04-open-source-terms.md): every Hersteller needs its own HerstellerID, sharing undocumented, so user-supplied only; downloads are public but unlicensed for redistribution (nothing from them in the repo, test certificates fetched at test time); "elster-cli" is fine with a non-affiliation notice and no logo; open questions parked in Ask ELSTER the open terms questions (not sent).
- [Open-interface transport, end to end](issues/01-open-interface-transport.md): (background since the ERiC decision) gzip → CMS EnvelopedData (RSA-OAEP-SHA256, AES-256-CBC) → Base64, XML-DSig RSASSA-PSS over the encrypted DatenTeil, reply as CMSEncryptedData under our session key; TLS client certificate `extern-offen.p12` required; example HerstellerID 74931 is rejected, so a real HerstellerID is needed even for the test system.
- [Go vs Rust library support](issues/05-go-vs-rust-libraries.md): (transport part superseded by the ERiC decision) Go leads: pure cross-builds for macOS without a C toolchain, working CMS decrypt/sign (`smallstep/pkcs7`), only the OAEP EnvelopedData encoder to hand-roll (~0.5 day, verified against OpenSSL); Rust is better only for XML-DSig and ERiC bindings, and its CMS crates lack OAEP and decrypt.
- [ERiC for the Kontoabfrage](issues/07-eric-decision.md): v1 runs entirely on ERiC and Otto, which are mandatory and installed by `eric install --accept-eric-terms` (anonymous download, pinned SHA-256, tested majors only); Linux x86_64 and macOS only (narrowed to Linux by Go or Rust); hand-written runtime bindings.
- [Go or Rust](issues/08-language.md): Rust on `x86_64-unknown-linux-gnu`, Linux only in v1 with a single platform table for later macOS; own `libloading` binding (not eric-sdk, which links ERiC at build time and pins one version) in a safe `eric` module behind a trait; one ERiC session per process on the main thread; ERiC ≥ 44 signatures only; blocking, `ureq` + `rustls`.
- [Ask ELSTER the open terms questions](issues/12-ask-elster.md): not sent; v1 proceeds on the research readings (nothing from the downloads in the repo, user-supplied HerstellerID, explicit Bestätigung, ERiC user-downloaded, name with non-affiliation notice); an objection from ELSTER reopens the affected ticket.
- [Agent interface](issues/09-agent-interface.md): German lifecycle commands (`postfach status|anfrage|abholung|bestaetigung`, `kontoabfrage`, `eric install|status`, `describe`, `skill`); `abholung` requires `--out DIR` and an explicit `--bestaetigen`/`--ohne-bestaetigung`, confirms only fully written Nachrichten, reports `bestaetigung_faellig_bis`; flat JSON with `warnings`, semver-versioned; rustc-style JSON errors on stderr, exit codes 0–7; `describe` generated from types; `SKILL.md` embedded.
- [Configuration and secrets](issues/10-config-and-secrets.md): flag > `ELSTER_CLI_*` env > TOML config (`$XDG_CONFIG_HOME/elster-cli/config.toml`) > default, no keychain; PIN only via env, PIN file or `pin_command` (never a flag or literal in config, never a prompt); named profiles per Taxpayer, HerstellerID and ERiC directory top-level; live by default, `--test` sets the Testmerker; outputs carry `test` and `profile`; new `config show`.
- [Kontoabfrage requests and responses](issues/13-kontoabfrage-requests.md): all three Abfragearten as `kontoabfrage offene-betraege|zahlungen|sollstellungen`, repeated filters fan out into one request; one Steuernummer per call (profile `steuernummer` + optional `bundesland` for the Bescheid format), Finanzamt derived from it; faithful JSON per response shape with `antwort_format: biene|alt`, amounts as decimal strings, `--mit-xml` for the raw XML; Fehlerliste codes mapped to exit 0/5/6/7.
- [Certificate inspection](issues/14-certificate-inspection.md): new local `zertifikat status` (PIN needed, no ELSTER request, the agent's credentials preflight) reporting validity, type and Konto-ID; every command opening the Certificate warns under 60 days and fails locally with exit 3 on expiry, wrong PIN or a test certificate without `--test`; `warnings` entries become `{code, message, help?}` objects.
- [Testing strategy](issues/15-testing-strategy.md): in-process fake at ERiC-call level for commands, real ERiC offline for the binding and golden requests (no C mocks), hand-written invented fixtures validated against XSDs downloaded at test time; GitHub Actions with `checks` and `eric-local` on every PR incl. forks (ERiC validates with placeholder HerstellerID `12345`, only `74931` is blocked) and an environment-gated `test-system` job holding the HerstellerID; committed JSON Schemas with a semver test; maintainer-run live check against `docs/live-check.md`.
- [Distribution](issues/16-distribution.md): `cargo-zigbuild` for glibc 2.25 with an `objdump` guard, `libgcc_s.so.1` a documented requirement diagnosed by `eric install|status`; tag-triggered GitHub Releases gated by `test-system`, `SHA256SUMS` + GitHub attestations, crates.io `elster-cli` with an `include` allowlist, binstall metadata, 0.x until the live check; `AGPL-3.0-only` + §7 ERiC/Otto exception, DCO, no CLA (ADR 0003); README and getting-started in English and German (English canonical, no CI sync check), `SKILL.md` and CLI output English only; non-affiliation notice in READMEs, `NOTICE`, `SKILL.md`, crates.io; onboarding ends at `zertifikat status`.

## Not yet specified

- Finishing `spec.md`: drafted on 2026-11-20 from every resolved ticket; the items marked ⏳ (listed in its "Pending the spike" table) are filled in once [Test-system spike: ERiC end to end](issues/11-test-system-spike.md) and [Obtain a HerstellerID](issues/06-obtain-herstellerid.md) resolve. Then the map is done.

## Out of scope

- Sending of any kind (UStVA, ZM, returns, replies to Postfach-Nachrichten): v2.
- Abholung als Vertretung and multiple Taxpayers per call.
- Local archive or any persisted state.
- Integration into consuming repos (e.g. a `pull_elster` step in a bookkeeping repo): the consumer's business.
- Policy on agent usage (unattended runs, approvals).
- Mapping Bescheiddaten to JSON: v1 writes the Datenpaket raw to disk ([Agent interface](issues/09-agent-interface.md)); the mapping is an additive v1.x feature charted in its own map, [Bescheiddaten](../bescheid-daten/map.md).
- VaSt-Beleg-Daten: provided for natural persons' income tax pre-fill, not needed for v1.
- macOS support (release binary, signing, notarization, Homebrew): a later effort per [Go or Rust](issues/08-language.md); v1 only keeps platform details in one place.
