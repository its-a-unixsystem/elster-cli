# KIECLI

> KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.

🇬🇧 English | [🇩🇪 Deutsch](https://github.com/its-a-unixsystem/kiecli/blob/main/README.de.md)

KIECLI is a stateless, non-interactive command-line client for AI agents that talks to ELSTER, the German tax authorities' online service. For one Taxpayer per call it reads the Postfach (messages, Bescheide and their attachments), runs the Kontoabfrage (open amounts, payments, assessments) and reports on the Certificate it uses. Output is JSON by default.

## The name

KIECLI stands for **KI Elster CLI**:

- **KI**: Künstliche Intelligenz, German for AI. The tool is built to be driven by AI agents.
- **Elster**: the ELSTER service it talks to.
- **CLI**: a command-line interface.

Written KIECLI in prose; the command, crate and repository are lowercase `kiecli`. The project started as "elster-cli", but ELSTER did not accept that product name when the project applied for its HerstellerID.

## Status

`eric install`, `eric status`, `config show`, `zertifikat status`, `postfach status|anfrage|abholung|bestaetigung`, `kontoabfrage offene-betraege|zahlungen|sollstellungen`, `describe`, and `skill` are available. `zertifikat status` needs a Certificate and PIN; Postfach and Kontoabfrage commands also need a HerstellerID.

## Install

KIECLI runs on Linux x86_64. The [release archive](https://github.com/its-a-unixsystem/kiecli/releases) requires glibc 2.25 or newer and `libgcc_s.so.1`. See [Getting started](https://github.com/its-a-unixsystem/kiecli/blob/main/docs/getting-started.md) for installation from the archive, `cargo binstall`, or `cargo install`, then install ERiC separately. Versions before 1.0 do not promise a stable JSON contract across minor releases.

`--format text` makes the result easier to read. `--eric-log DIR` keeps the diagnostic log; otherwise it is temporary.

An unusable installation exits with code 4. Stdout still contains the status checks that ran; stderr contains the error. JSON uses `null` for the version and its support status when the version could not be read.

## Inspecting configuration

Run `kiecli config show [--profile NAME]` to see the effective settings without loading ERiC, opening the Certificate, or reading a PIN file or running a PIN command. Settings come from flags, `KIECLI_*` environment variables, the [TOML config file](https://github.com/its-a-unixsystem/kiecli/blob/main/.scratch/kiecli/spec.md#4-configuration-and-secrets), and defaults, in that order.

JSON reports settings as `{"value": ..., "source": ...}`. The common `profile` and `test` fields keep their usual types and use `profile_source` and `test_source`. For `pin`, the value is only the channel (`env`, `file`, `command`, or `null`), never the PIN, file path, or command. Within a layer, direct PIN environment input takes precedence over a PIN file, then a PIN command. The two PIN flags are mutually exclusive.

## Inspecting a Certificate

Run `kiecli zertifikat status --profile NAME` with Certificate and PIN settings in the profile, or supply `--certificate PATH --pin-file PATH`. This opens the Certificate locally with ERiC and sends nothing to ELSTER. JSON includes the validity dates, the Konto-ID, and whether this is an Organisationszertifikat or a test Certificate. Expired and not-yet-valid Certificates are reported with a warning and exit 0; a wrong or locked PIN exits 3.

PINs can also come from `KIECLI_PIN` or a non-interactive `--pin-command`. The command must write the PIN only to stdout; stderr is for non-secret diagnostics. If the command fails without providing a PIN to redact, KIECLI reports the failure without forwarding its stderr.

## Checking Postfach counts

Run `kiecli postfach status --profile NAME --von YYYY-MM-DD` to ask ELSTER for counts without collecting or confirming any Nachrichten. The default is `--neue` (not yet confirmed); use `--alle` to include confirmed Nachrichten. ELSTER may omit some Bereitstellungsdatenarten, so treat the counts as a hint. `--datenart NAME` filters the returned counts locally; use `--von` or `--bis` to narrow a request that exceeds ELSTER's limit. Use `--test` with a test Certificate; otherwise the command uses the live service by default.

## Listing Postfach metadata

Run `kiecli postfach anfrage --profile NAME --von YYYY-MM-DD` to list Postfach-Nachrichten. It asks ELSTER directly rather than relying on `postfach status`, follows all pages, and returns metadata in ID order without collecting or confirming anything. The default is `--neue`; use `--alle` for already confirmed Nachrichten too. Repeat `--datenart NAME` to limit what ELSTER searches. Use `--test` with a test Certificate.

## Collecting and confirming Postfach files

Run `kiecli postfach abholung --id ID --out DIR --bestaetigen` to write the Datenpaket and Anhänge to a directory you control, then confirm receipt only once every file for that Nachricht is safely written. IDs come from `postfach anfrage`; repeat `--id` for multiple Nachrichten or use `--neue` to collect all unconfirmed Nachrichten in the selected range. Use `--von` and `--bis` to narrow the search in a large Postfach. Add `--mit-meta` for the Anfrage metadata on stdout. File contents stay on disk; no separate metadata file is created.

Use an output directory you own. The CLI refuses Postfach output directories writable by group or others, and does not follow symlinks at Nachricht or Anhang directories.

Use `--ohne-bestaetigung` instead to defer confirmation, then run `kiecli postfach bestaetigung --id ID` before the reported `bestaetigung_faellig_bis` (24 hours after collection starts). On exit 7, inspect each Nachricht: a partial Abholung is not confirmed but still has a deadline. Retry that ID; if retries keep failing, confirm before the deadline to avoid ELSTER blocking the HerstellerID. Neither command silently switches into test mode.

## Querying a tax account

Run `kiecli kontoabfrage offene-betraege --profile NAME --steuernummer NUMBER` to query open amounts. `zahlungen` also requires `--ab YYYY-MM-DD`, optionally accepts `--ein-jahr` and repeatable `--steuerart`. `sollstellungen` requires repeatable `--steuerart` and `--jahr`; the request is limited to 75 combinations. Tax types are case-insensitive and amounts are returned as decimal strings. A Bescheid-format Steuernummer requires `--bundesland`; ERiC converts it locally. Kontoabfragen use the live service by default; pass `--test` with a test Certificate to use ELSTER's test system. Add `--mit-xml` to include the raw Kontoabfrage Nutzdaten in the output.

## Using KIECLI from an agent

Run `kiecli skill` for the [agent playbook](https://github.com/its-a-unixsystem/kiecli/blob/main/SKILL.md) and `kiecli describe` for the machine-readable command reference. Use `kiecli describe postfach abholung` to narrow it to one command. Both work offline without configuration, credentials, or ERiC.

The [contract snapshot](https://github.com/its-a-unixsystem/kiecli/blob/main/schemas/describe.json) contains the complete command reference and the stdout and stderr JSON Schemas. After changing it, regenerate the snapshot with `cargo run --locked --quiet -- describe | jq -S . > schemas/describe.json` and record schema changes in the changelog. CI compares the snapshot with live `describe` output and checks breaking changes against the latest reachable, published GitHub release.

## Continuous integration

CI checks formatting, Clippy, tests, dependency licences and advisories, and
DCO sign-offs (`git commit --signoff`). The `eric-local` job installs verified
ERiC on the runner and exercises it offline without a HerstellerID secret.
The manual `elster-reachability` run uses only an official test Certificate
and the placeholder HerstellerID `12345`. It sends one read-only PostfachStatus
and expects ELSTER's `601015001` reply. ELSTER files remain on the ephemeral
runner and are not uploaded to caches or artifacts.

To exercise the Postfach lifecycle against the ELSTER test system, dispatch
CI on `main` with `test_system` enabled. CI collects and re-confirms only a
Nachricht that is already confirmed in the shared test Postfach. The `elster-test`
environment must supply the `KIECLI_HERSTELLER_ID` secret and require maintainer
approval. The weekly `main` run uses the same secret from
`elster-test-scheduled`. Neither job sends a live request.

See [Contributing](https://github.com/its-a-unixsystem/kiecli/blob/main/CONTRIBUTING.md) for release prerequisites and the maintainer checklist.

## Licence

`AGPL-3.0-only` ([LICENSE](LICENSE)), with an [additional permission](NOTICE) to combine the program with ERiC and Otto as published by ELSTER.
