# Changelog

## [Unreleased]

### Added

- `eric status` checks a local ERiC and Otto installation and reports the loaded ERiC version. It runs offline without a Certificate or HerstellerID.
- GitHub CI checks Rust code, dependency licences and advisories, and DCO sign-offs. Separate jobs exercise official ERiC offline and probe ELSTER test-system reachability without a secret.
- `config show` reports the effective settings and their sources without opening a Certificate, loading ERiC, or disclosing PIN details.
- `zertifikat status` opens a Certificate with its PIN and reports validity, Konto-ID, and Certificate properties offline. It requires no HerstellerID and warns about expired, not-yet-valid, and soon-to-expire Certificates.

### Security

- Updated the XML and ZIP dependencies to versions that fix known RustSec advisories.
