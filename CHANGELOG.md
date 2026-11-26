# Changelog

## [Unreleased]

### Added

- `eric status` checks a local ERiC and Otto installation and reports the loaded ERiC version. It runs offline without a Certificate or HerstellerID.
- GitHub CI checks Rust code, dependency licences and advisories, and DCO sign-offs. Separate jobs exercise official ERiC offline and probe ELSTER test-system reachability without a secret.
- `config show` reports the effective settings and their sources without opening a Certificate, loading ERiC, or disclosing PIN details.
- `zertifikat status` opens a Certificate with its PIN and reports validity, Konto-ID, and Certificate properties offline. It requires no HerstellerID and warns about expired, not-yet-valid, and soon-to-expire Certificates.
- `postfach status` reports ELSTER's approximate Postfach counts without collecting or confirming Nachrichten. It supports date bounds and a local Bereitstellungsdatenart filter.
- `postfach anfrage` lists Postfach metadata, including object references and decoded Anschreiben, in ID order across all pages without collecting or confirming Nachrichten.
- `postfach abholung` writes Datenpakete and Anhänge atomically, reports their hashes and Bestätigung deadlines, and requires an explicit confirmation choice. `postfach bestaetigung` confirms by ID and reports each ELSTER result separately.
- `kontoabfrage offene-betraege`, `zahlungen` and `sollstellungen` send ERiC Kontoabfragen, convert Bescheid-format Steuernummern, and return legacy or BIENE results as JSON.
- `describe [command]` reports command arguments, settings, diagnostics, and JSON Schemas. `skill` prints the bundled agent playbook. Both run without configuration, credentials, or ERiC.
- Check in the initial `describe` and JSON Schema contract. Tests check for snapshot drift, validate fake-driven command output, and enforce the version bump required for breaking changes.
- Gate tagged releases on CI and the ELSTER test system. Publish an attested Linux x86_64 archive for glibc 2.25 or newer, plus a crates.io source package, and document installation in English and German.

### Fixed

- Kontoabfrage reports ELSTER's authorization denial even when the reply has an empty header stack code and echoes the query without returning account data.

- Kontoabfrage lets ERiC derive the four-digit Finanzamt number from full Bavarian and Berlin Bescheid-format Steuernummern instead of passing a three-digit prefix.

- Kontoabfrage now reports retryable ELSTER backend results as exit 6 and preserves the ELSTER code even when there is no result text.
- Fixed Kontoabfrage warnings that blocked the strict CI lint check without changing requests or output.
- Allow the permissive MIT-0 license in dependency checks for `borrow-or-share`, a transitive test dependency.

### Security

- Updated the XML and ZIP dependencies to versions that fix known RustSec advisories.
