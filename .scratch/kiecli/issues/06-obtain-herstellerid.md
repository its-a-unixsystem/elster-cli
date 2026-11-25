# Obtain a HerstellerID

Type: task
Status: resolved
Blocked by:

## Question

Our own HerstellerID must exist before the test-system spike can run. [Open-interface transport, end to end](01-open-interface-transport.md) showed the server rejects the example ID 74931 even with a Testmerker, so this is on the critical path.

The application was submitted through the ELSTER developer portal on 2026-11-19, for product "elster-cli", interfaces "ERiC, Sonstige". HITL: when the ID arrives, store it outside this repository and record here only where it is kept, never the ID itself.

Amended by [Distribution](16-distribution.md): when this resolves, fill step 4 of `docs/getting-started.md` (and `.de.md`) with what every user must do: Entwicklerbereich registration (login after a few days), then the HerstellerID application (days to weeks), plus anything learned here, such as eligibility or form fields, without personal data.

## Answer

**Assigned by 2026-11-25, six days after the application. The ID is kept on the maintainer's machine in `~/.config/kiecli/config.toml` (top-level `hersteller_id`, mode 0600), never in this repository.**

- ELSTER refused the product name "elster-cli". The project was renamed to KIECLI (KI Elster CLI) on 2026-11-25: display name KIECLI; command, crate, repository (`its-a-unixsystem/kiecli`) and config directory `kiecli`; env prefix `KIECLI_`.
- The `test-system` CI secret `KIECLI_HERSTELLER_ID` is not set yet: there are no workflows. Set it in the `elster-test` and `elster-test-scheduled` environments when they exist ([Testing strategy](15-testing-strategy.md), item 15).
- For `docs/getting-started.md` step 4: register in the Entwicklerbereich (login works after a few days), apply for a HerstellerID in the developer portal naming the product and the interfaces used (ERiC, Sonstige); the ID came within a week. A product name containing "elster" may be refused.
