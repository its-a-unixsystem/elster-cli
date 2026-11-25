# KIECLI

> KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.

KIECLI is a stateless, non-interactive command-line client for AI agents that talks to ELSTER, the German tax authorities' online service. For one Taxpayer per call it reads the Postfach (messages, Bescheide and their attachments), runs the Kontoabfrage (open amounts, payments, assessments) and reports on the Certificate it uses. Output is JSON by default.

## The name

KIECLI stands for **KI Elster CLI**:

- **KI**: Künstliche Intelligenz, German for AI. The tool is built to be driven by AI agents.
- **Elster**: the ELSTER service it talks to.
- **CLI**: a command-line interface.

Written KIECLI in prose; the command, crate and repository are lowercase `kiecli`. The project started as "elster-cli", but ELSTER did not accept that product name when the project applied for its HerstellerID.

## Status

Planning. There is no code yet. The v1 specification is drafted in [`.scratch/kiecli/spec.md`](.scratch/kiecli/spec.md); domain terms are in [`GLOSSARY.md`](GLOSSARY.md) and design decisions in [`docs/adr/`](docs/adr/).

## Licence

Planned: `AGPL-3.0-only`, with an additional permission to combine the program with ERiC and Otto as published by ELSTER ([ADR 0003](docs/adr/0003-agpl-with-eric-exception.md)).
