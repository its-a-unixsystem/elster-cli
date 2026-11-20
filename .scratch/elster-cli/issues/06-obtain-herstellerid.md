# Obtain a HerstellerID

Type: task
Status: open
Blocked by:

## Question

Our own HerstellerID must exist before the test-system spike can run. [Open-interface transport, end to end](01-open-interface-transport.md) showed the server rejects the example ID 74931 even with a Testmerker, so this is on the critical path.

The application was submitted through the ELSTER developer portal on 2026-11-19, for product "elster-cli", interfaces "ERiC, Sonstige". HITL: when the ID arrives, store it outside this repository and record here only where it is kept, never the ID itself.

Amended by [Distribution](16-distribution.md): when this resolves, fill step 4 of `docs/getting-started.md` (and `.de.md`) with what every user must do: Entwicklerbereich registration (login after a few days), then the HerstellerID application (days to weeks), plus anything learned here, such as eligibility or form fields, without personal data.
