# Configuration and secrets

Type: grilling
Status: resolved
Blocked by: 04, 07

## Question

How does the user supply the Certificate path, its PIN, the HerstellerID, the test vs live switch, the ERiC directory (default: OS data directory, see [ERiC for the Kontoabfrage](07-eric-decision.md)) and (for the Kontoabfrage) Steuernummer and Finanzamt: flags, environment variables, a config file, the OS keychain, and in what precedence, without any interactive prompt?

Constraints from research: the OS keychain on Linux needs a running D-Bus Secret Service, so headless and CI runs need a non-keychain path ([Go vs Rust library support](05-go-vs-rust-libraries.md)). The TLS client certificate `extern-offen.p12` is no longer needed: ERiC handles transport. ERiC takes the Certificate as a file path, not bytes.

## Answer

**Four layers (flag > env > config file > default), named profiles per Taxpayer, and a PIN that never appears in flags or config.** No OS keychain backend; `pin_command` reaches keychains and password managers instead. Term Testmerker added to `GLOSSARY.md`.

### Settings and layers

1. Precedence: flag > environment > config file > built-in default. Env vars are `ELSTER_CLI_` + the upper-cased key. Key names follow the glossary (`certificate`, not `zertifikat`).

| Setting | Flag | Env | Config key | Scope |
|---|---|---|---|---|
| Certificate path | `--certificate` | `ELSTER_CLI_CERTIFICATE` | `certificate` | profile |
| PIN | `--pin-file`, `--pin-command` | `ELSTER_CLI_PIN`, `ELSTER_CLI_PIN_FILE`, `ELSTER_CLI_PIN_COMMAND` | `pin_file`, `pin_command` | profile |
| Testmerker | `--test` / `--no-test` | `ELSTER_CLI_TEST=0\|1` | `test` | profile |
| Steuernummer (ELSTER or Bescheid format) | `--steuernummer` | `ELSTER_CLI_STEUERNUMMER` | `steuernummer` | profile |
| Bundesland (only for a Bescheid-format Steuernummer) | `--bundesland` | `ELSTER_CLI_BUNDESLAND` | `bundesland` | profile |
| HerstellerID | `--hersteller-id` | `ELSTER_CLI_HERSTELLER_ID` | `hersteller_id` | top-level |
| ERiC directory | `--eric-dir` | `ELSTER_CLI_ERIC_DIR` | `eric_dir` (default: OS data directory) | top-level |
| Profile | `--profile` | `ELSTER_CLI_PROFILE` | `default_profile` | top-level |
| Config file | `--config` | `ELSTER_CLI_CONFIG` | | |

2. **Flag-only:** `--format`, `-v`, `--eric-log`. A shared `format = "text"` would break every agent using the file; diagnostics belong to one call.
3. **Test vs live:** live by default; `test` puts the Testmerker on every request.

### PIN

4. Only via `ELSTER_CLI_PIN`, a PIN file or a PIN command. Never a `--pin` flag (leaks into `ps`, shell history, agent transcripts), never a literal PIN in the config file. No PIN available: exit 3 with an error listing the three ways; never a prompt, even on a TTY.
5. **PIN file:** contains only the PIN, one trailing newline stripped. Readable by group or others: a `warnings` entry, not a refusal (CI and Docker secret mounts are often 0644).
6. **PIN command:** run via `/bin/sh -c`, stdin `/dev/null`; the PIN is stdout minus one trailing newline; empty output, non-zero exit or a 30 s timeout is exit 3 with the captured stderr in `cause`. Runs only for commands that need the PIN (not `eric install|status`, `describe`, `skill`, `config show`).
7. The PIN is held in a zeroizing buffer and dropped after ERiC and Otto have used it.

### Config file

8. TOML at `$XDG_CONFIG_HOME/elster-cli/config.toml` (default `~/.config/elster-cli/config.toml`). A missing default file is fine (env-only CI); a missing file named by `--config`/`ELSTER_CLI_CONFIG` is exit 3. Unknown keys are exit 3 with a did-you-mean `help`. `~` is expanded; relative paths resolve against the config file's directory, paths from flags and env against the working directory.
9. **Profiles:** `[profile.NAME]` holds the per-Taxpayer fields; `hersteller_id` and `eric_dir` stay top-level (they belong to the Hersteller and the tool). Per-Taxpayer keys are not allowed at top level.

```toml
hersteller_id = "…"
default_profile = "privat"

[profile.privat]
certificate = "~/elster/privat.pfx"
pin_command = "pass show elster/privat"

[profile.firma]
certificate = "~/elster/firma.pfx"
pin_file = "~/elster/firma.pin"
test = true
```

10. **Selection:** `--profile` > `ELSTER_CLI_PROFILE` > `default_profile` > the only profile in the file. Several profiles and none chosen: exit 3 listing the names. Flags and env override single fields of the selected profile.

### Visibility and errors

11. Every command's JSON output carries `test` (bool) and `profile` (name or null), additive to [Agent interface](09-agent-interface.md).
12. When an env var overrides a field the selected profile sets, a `warnings` entry names the field (guards against a stale `ELSTER_CLI_CERTIFICATE` signing for the wrong profile).
13. A missing setting is exit 3; the error's `notes` name its flag, env var and config key. A Certificate path that does not exist or cannot be read is exit 3 before ERiC starts.
14. **`elster-cli config show [--profile X]`** (new command, additive to [Agent interface](09-agent-interface.md)): JSON with each resolved setting, its value and its source (`flag`, `env`, `file:<path>`, `default`). The PIN shows only its channel. Never runs the PIN command, never touches ERiC or the Certificate (opening it stays with Certificate inspection in the map's fog).
15. `describe` lists every setting with its flag, env var, config key, scope and default.

No ADR: none of this is hard to reverse.

Amended by [Kontoabfrage requests and responses](13-kontoabfrage-requests.md): the Taxpayer-identifier row became `steuernummer` and `bundesland`.
