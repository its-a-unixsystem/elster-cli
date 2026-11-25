# Distribution

Type: grilling
Status: resolved
Blocked by:

## Question

How is KIECLI v1 built, released and installed, and how does a new user get from download to a first call? Settle:

- **Release binary** for `x86_64-unknown-linux-gnu` with glibc baseline 2.25 (measured from ERiC 44.3.6.0 in the pre-spike findings of [Test-system spike: ERiC end to end](11-test-system-spike.md)). Decide the build approach (an old-glibc container, `cargo-zigbuild` or other) and how to handle `libgcc_s.so.1`, which ERiC needs on the target system.
- **`cargo install`:** whether to publish to crates.io, and whether the name is available.
- **Releases:** where they are published, artifact checksums and signing; versioning follows the CLI semver that also versions the JSON output ([Agent interface](09-agent-interface.md)).
- **Licence** of the repository.
- **Onboarding docs:** how the README and `SKILL.md` lead a user to `eric install --accept-eric-terms`, to their own HerstellerID (every Hersteller needs one, per [ELSTER terms for an open-source client](04-open-source-terms.md)), to a Certificate and to `zertifikat status` as the first check; the non-affiliation notice.

From [Testing strategy](15-testing-strategy.md): CI is GitHub Actions; the release workflow must run the `test-system` job (environment `elster-test`) as a required gate before artifacts are published, and the maintainer's live check (`docs/live-check.md`) precedes 1.0.0 and any release changing the ERiC major or request XML.

macOS packaging stays out of scope ([Go or Rust](08-language.md)).

## Answer

**A zigbuilt glibc-2.25 binary on GitHub Releases, gated by the test system, also on crates.io; AGPL-3.0-only with an ERiC/Otto exception; bilingual user docs that lead to `zertifikat status`.**

### Build

1. `cargo-zigbuild --target x86_64-unknown-linux-gnu.2.25` on `ubuntu-latest`. A release step runs `objdump -T` over the binary and fails on any `GLIBC_` symbol newer than 2.25.
2. `libgcc_s.so.1` is a documented system requirement, never shipped (present on every mainstream glibc distro, missing in `distroless/static`, `scratch` and some minimal images). `eric install` and `eric status` test-load ERiC; a load failure caused by a missing `libgcc_s.so.1` is its own error code under exit 4, with a `help` naming the package (`libgcc-s1` on Debian/Ubuntu, `libgcc` on Fedora/Arch).

### Releases

3. GitHub Releases on `its-a-unixsystem/kiecli`, triggered by a `vX.Y.Z` tag, through a hand-written workflow (not `dist`, so the environment-gated job wires in plainly): `checks` + `eric-local` → `test-system` (environment `elster-test`, required) → zigbuild + glibc guard → draft release → publish → `cargo publish`.
4. Artifact `kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz`: binary, `LICENSE`, `NOTICE`, `README.md`, `README.de.md`.
5. Integrity: a `SHA256SUMS` file plus GitHub artifact attestations (Sigstore build provenance, `gh attestation verify`). No long-lived signing key.
6. crates.io: the crate `kiecli` (name free on 2026-11-25) is published by the release workflow after the GitHub Release, with an explicit `include` allowlist in `Cargo.toml` so no ELSTER material can enter the package. `elster` is not reserved (name-squatting). `cargo install` builds against the host glibc, so the baseline does not apply there.
7. `[package.metadata.binstall]` points `cargo binstall` at the release tarball. No `curl | sh` script, no distro packages in v1 (ERiC cannot be packaged anyway; community packaging welcome).
8. Versioning: `0.y.z` until the maintainer live check (`docs/live-check.md`) passes; 1.0.0 is the first release with the JSON-output stability promise (before that, a minor bump may break output, per Cargo semver). `CHANGELOG.md` in Keep a Changelog format; every JSON Schema change needs an entry.

### Licence

9. `AGPL-3.0-only` (as haben) plus a GPLv3 §7 additional permission allowing the program to be combined with ERiC and Otto as published by ELSTER and the result conveyed, without the AGPL applying to ERiC or Otto. Text in `LICENSE` and `NOTICE`. Contributions inbound = outbound with a DCO `Signed-off-by` checked in CI; no CLA, so the licence is effectively permanent. ADR `docs/adr/0003-agpl-with-eric-exception.md`.

### Docs

10. Bilingual (English canonical, German counterpart): `README.md`/`README.de.md` and `docs/getting-started.md`/`docs/getting-started.de.md`, each with 🇬🇧 English | 🇩🇪 Deutsch links at the top; the German files say in one line that English wins on disagreement. The non-affiliation notice in `NOTICE` appears in both languages. Sync is asked in `CONTRIBUTING.md` and by a PR-template checkbox ("updated `.de.md` counterpart, or not needed"), no CI check.
11. English only: `SKILL.md` (agents read it; one embedded copy), `--help`, `describe`, every JSON `message`/`help`, ADRs, `CHANGELOG.md`, `CONTRIBUTING.md`, `docs/live-check.md`.
12. Non-affiliation notice, fixed text, at the top of both READMEs, in `NOTICE`, in `SKILL.md` and in the crates.io `description`; never in `--version` or JSON. No logo or ELSTER imagery. Wording, trademark sentence included, checked before the first release:
    > KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.
13. `docs/getting-started.md` (and `.de.md`), linked from the READMEs, in this order:
    1. System requirements: Linux x86_64, glibc ≥ 2.25, `libgcc_s.so.1`.
    2. Install: release tarball (verify `SHA256SUMS`/attestation), `cargo binstall` or `cargo install`.
    3. `kiecli eric install --accept-eric-terms`, then `eric status`.
    4. Own HerstellerID: register for the ELSTER Entwicklerbereich (`https://www.elster.de/elsterweb/registrierung-entwickler/form`), wait a few days for the login, then apply under `https://www.elster.de/elsterweb/entwickler/antrag-hersteller-id/form`, which takes days to weeks. Further details from [Obtain a HerstellerID](06-obtain-herstellerid.md) once it resolves.
    5. Certificate and PIN setup per [Configuration and secrets](10-config-and-secrets.md).
    6. `kiecli zertifikat status` as the first call.
    7. `--test` against the test system.
14. `SKILL.md` tells the agent to stop and hand steps 4 and 5 back to the human (they need a person: an ELSTER login, a Certificate, a PIN source), and links the getting-started guide.

Amends [Agent interface](09-agent-interface.md), [Testing strategy](15-testing-strategy.md) and [Obtain a HerstellerID](06-obtain-herstellerid.md).

