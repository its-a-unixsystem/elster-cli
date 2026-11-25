# AGPL-3.0-only with an additional permission for ERiC and Otto

KIECLI is licensed `AGPL-3.0-only`, like haben, so modified versions, including ones run as a service, stay open. Because the CLI loads the proprietary ERiC and Otto into its own process and they are not System Libraries, the plain AGPL would forbid conveying a modified KIECLI meant to use them, or running one as a network service, since their source cannot be provided. A GPLv3 §7 additional permission allows combining the program with ERiC and Otto as published by ELSTER and conveying the result without the AGPL applying to them.

## Considered Options

- **`MIT OR Apache-2.0`** (Rust default): no question about ERiC, but no copyleft.
- **AGPL without an exception**: the licence would work against the tool's only use case.

## Consequences

- Contributions come in under the same licence with a DCO sign-off and no CLA, so relicensing later needs every contributor's consent: treat the licence as permanent.
- Dependencies must be AGPL-compatible; `cargo deny`'s licence allowlist enforces it.
