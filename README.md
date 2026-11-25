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

Implementation has started. The first command, `eric status`, checks a local ERiC installation without a Certificate or a connection to ELSTER. The [v1 specification](.scratch/kiecli/spec.md), [glossary](GLOSSARY.md) and [design decisions](docs/adr/) describe the rest of v1.

On Linux x86_64 with `libgcc_s.so.1` installed, unpack the official ERiC 44 package. Read its `lizenz.pdf` and data-protection notice. If you accept the terms, record acceptance next to `lizenz.pdf` and point KIECLI at that directory:

```sh
touch /path/to/Linux-x86_64/.kiecli-eric-terms-accepted
cargo run -- eric status --eric-dir /path/to/Linux-x86_64
```

`--format text` makes the result easier to read. `--eric-log DIR` keeps the diagnostic log; otherwise it is temporary. Use `eric install --accept-eric-terms` to install ERiC once that command ships.

An unusable installation exits with code 4. Stdout still contains the status checks that ran; stderr contains the error. JSON uses `null` for the version and its support status when the version could not be read.

## Licence

`AGPL-3.0-only` ([LICENSE](LICENSE)), with an [additional permission](NOTICE) to combine the program with ERiC and Otto as published by ELSTER.
