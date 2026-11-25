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

`eric install`, `eric status`, and `config show` are available. None needs a Certificate or HerstellerID. The [v1 specification](.scratch/kiecli/spec.md), [glossary](GLOSSARY.md) and [design decisions](docs/adr/) describe the rest of v1.

On Linux x86_64 with `libgcc_s.so.1` installed, review `lizenz.pdf` inside the [official ERiC package](https://download.elster.de/download/eric/eric_44/ERiC-44.3.6.0-Linux-x86_64.jar) (a ZIP archive) and the [tax administration's data-protection notice](https://www.elster.de/eportal/helpGlobal?themaGlobal=datenschutz_public#ds_steuerverw). If you accept the terms:

```sh
cargo run -- eric install --accept-eric-terms
cargo run -- eric status
```

The installer downloads the official package, checks its pinned SHA-256 and test-loads both libraries before activating it. Successful installs update `.kiecli-eric-current` in the configured ERiC directory (default: `$XDG_DATA_HOME/kiecli/eric` or `~/.local/share/kiecli/eric`). Older versions remain under `versions/`; failed installs leave the active version alone. To select an unpinned patch in a supported major, supply both `--version` and a trusted `--sha256`.

Manual setup still works: unpack the official ERiC 44 package, review its `lizenz.pdf` and data-protection notice, and record acceptance next to `lizenz.pdf` before pointing KIECLI at that directory:

```sh
touch /path/to/Linux-x86_64/.kiecli-eric-terms-accepted
cargo run -- eric status --eric-dir /path/to/Linux-x86_64
```

`--format text` makes the result easier to read. `--eric-log DIR` keeps the diagnostic log; otherwise it is temporary.

An unusable installation exits with code 4. Stdout still contains the status checks that ran; stderr contains the error. JSON uses `null` for the version and its support status when the version could not be read.

## Inspecting configuration

Run `cargo run -- config show [--profile NAME]` to see the effective settings without loading ERiC, opening the Certificate, or reading a PIN file or running a PIN command. Settings come from flags, `KIECLI_*` environment variables, the [TOML config file](.scratch/kiecli/spec.md#4-configuration-and-secrets), and defaults, in that order.

JSON reports settings as `{"value": ..., "source": ...}`. The common `profile` and `test` fields keep their usual types and use `profile_source` and `test_source`. For `pin`, the value is only the channel (`env`, `file`, `command`, or `null`), never the PIN, file path, or command. Within a layer, direct PIN environment input takes precedence over a PIN file, then a PIN command. The two PIN flags are mutually exclusive.

## Continuous integration

CI checks formatting, Clippy, tests, dependency licences and advisories, and
DCO sign-offs (`git commit --signoff`). The `eric-local` job installs verified
ERiC on the runner and exercises it offline without a HerstellerID secret.
The manual `elster-reachability` run uses only an official test Certificate
and the placeholder HerstellerID `12345`. It sends one read-only PostfachStatus
and expects ELSTER's `601015001` reply. ELSTER files remain on the ephemeral
runner and are not uploaded to caches or artifacts.

## Licence

`AGPL-3.0-only` ([LICENSE](LICENSE)), with an [additional permission](NOTICE) to combine the program with ERiC and Otto as published by ELSTER.
