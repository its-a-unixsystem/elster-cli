# Agent interface

Type: grilling
Status: resolved
Blocked by: 02, 07

## Question

What command surface does v1 expose to agents: the commands and flags for Postfach status, listing, fetching a Postfach-Nachricht with attachments, Bestätigung and (if kept) Kontoabfrage; the JSON output schema for each; error format and exit codes; how commands describe themselves; and what the shipped `SKILL.md` teaches?

Constraint from [Postfach lifecycle and attachments](02-postfach-lifecycle.md): a Bestätigung must follow a successful Abholung within 24 h, or the HerstellerID risks a block. With a stateless CLI and explicit Bestätigung, decide how the interface keeps agents from fetching without confirming (e.g. fetch-and-confirm as one command, a loud warning in output and `SKILL.md`, or both).

## Answer

**German lifecycle commands, files to disk, and an Abholung that refuses to run until the caller decides about the Bestätigung.** Terms Anfrage, Abholung, Datenpaket and Anhang added to `GLOSSARY.md`.

### Bestätigung

1. `postfach abholung` requires exactly one of `--bestaetigen` or `--ohne-bestaetigung`; without either it fails with a usage error. Nothing is confirmed implicitly, and nothing is fetched without the caller having decided.
2. `--bestaetigen` confirms only Postfach-Nachrichten whose every file was written and fsynced. Others are reported unconfirmed.
3. `--ohne-bestaetigung` gives each Nachricht `bestaetigung_faellig_bis` (Abholung time + 24 h, RFC 3339 UTC) and a `warnings` entry. The caller confirms later with `postfach bestaetigung --id …` after storing the files where it wants.
4. A partial Abholung (some objects fetched, one failed) counts as started: the Nachricht is `teilweise`, not confirmed, carries `bestaetigung_faellig_bis`, and the run exits 7. `SKILL.md`: retry the Abholung for that id, then confirm; if retries keep failing, confirm before the deadline anyway.
5. The Anfrage never creates the obligation; only an Abholung does.

### Commands

```
kiecli postfach status        [--datenart X…] [--neue|--alle] [--von D] [--bis D]
kiecli postfach anfrage       [--datenart X…] [--neue|--alle] [--von D] [--bis D]
kiecli postfach abholung      (--id ID… | --neue) --out DIR (--bestaetigen | --ohne-bestaetigung) [--datenart X…] [--mit-meta]
kiecli postfach bestaetigung  --id ID…
kiecli kontoabfrage offene-betraege
kiecli kontoabfrage zahlungen      --ab D [--ein-jahr] [--steuerart X…]
kiecli kontoabfrage sollstellungen --steuerart X… --jahr YYYY…
                                  # output and exit codes: Kontoabfrage requests and responses
kiecli eric install | status
kiecli describe [command]
kiecli skill
```

Global: `--format json|text` (default json), `-v`, `--eric-log DIR`; credentials and test/live per [Configuration and secrets](10-config-and-secrets.md). Never interactive.

- **Bereitstellungsdatenart selection:** a built-in list of every name in SSb table 9.4, sent in one PostfachAnfrage; `--datenart` narrows it. If the spike shows ELSTER rejects kinds the account never receives, switch to PostfachStatus first.
- **Defaults:** `--neue`; all pages fetched internally (`paginierungLetzteBestId`) and merged; no cursor exposed.
- **`abholung`** runs its own PostfachAnfrage to resolve object IDs, sizes and names, so the caller passes only ids (or `--neue`).
- **`eric status`**: ERiC directory, version found, supported or not (with supported majors), terms accepted, library paths. No network, no Certificate.

### Naming

Commands are named after ELSTER's lifecycle steps in German (ASCII-folded). JSON keys: glossary and ELSTER terms German snake_case (`bereitstellungsdatenart`, `verfuegbar_bis`), generic fields English (`id`, `path`, `warnings`). ELSTER `meta` keys are passed through raw.

### Files

`--out DIR` is mandatory; no content on stdout.

```
<out>/<nachricht-id>/datenpaket.<xml|txt>
<out>/<nachricht-id>/anhaenge/<NN>-<sanitised DateinameMitErweiterung>
```

`NN` is the Anhang position (ELSTER names are not unique). Each file is written to a temp file in the same directory, fsynced, then renamed; existing files are replaced. The Datenpaket is written byte for byte as delivered (ISO-8859-15 or UTF-8), its `encoding` reported in JSON. No metadata file on disk.

### Output (stdout)

One flat JSON object per command, always with `warnings: []`, no `data` wrapper. Dates ISO 8601, timestamps RFC 3339 UTC. Schema versioned with the CLI's semver (remove or rename = major, add = minor). `-v` adds a `trace` array (ERiC calls, durations, pages).

- **`status`**: `{"bereitstellungsdatenarten": [{"name", "anzahl"}], "einschraenkung", "warnings"}`.
- **`anfrage`**: `postfach_nachrichten`, sorted by id, each with `id`, `bereitstellungsdatenart`, `bereitstellungsticket`, `verfuegbar_bis`, `groesse`, `meta` (raw name/value), `anschreiben` (`sendername`, `bezeichnung`, `text` decoded from base64, or null), `meta_maps` (other MetaInfoMaps unchanged), `datenpaket` (`objektid`, `encoding`, `groesse`), `anhaenge` (`position`, `dateiname`, `dateibezeichnung`, `dateityp`, `objektid`, `groesse`, `meta`).
- **`abholung`**: per Nachricht `id`, `status` (`vollstaendig`/`teilweise`/`fehlgeschlagen`), `abgeholt_am`, `files` (`path`, `groesse`, `sha256`, `role` `datenpaket`|`anhang`, `position`), `bestaetigt`, `bestaetigung_faellig_bis` (while unconfirmed), `fehler` (error shape below). `--mit-meta` adds the `anfrage` fields.
- **`bestaetigung`**: ids sent 100 per block; per id `bestaetigt` and ELSTER's `rueckgabe {code, text}`.

### Errors (stderr)

stderr carries only one JSON error object, never progress or log lines. Detailed in the style of rustc, so an agent can fix the call:

```json
{"error": {"code": "datenart_unknown", "message": "unknown Bereitstellungsdatenart `DivaBescheidUst`",
  "notes": ["Bereitstellungsdatenart names are case-sensitive", "…"],
  "help": "did you mean `DivaBescheidUSt`?",
  "suggestion": "kiecli postfach anfrage --datenart DivaBescheidUSt",
  "cause": [{"source": "eric|elster|otto", "code": 610301202, "text": "…"}],
  "retry": false}}
```

`suggestion` is a corrected full command when one exists; the PIN is never echoed. `cause` passes ERiC, ELSTER and Otto codes and texts verbatim. `--format text` renders it rustc-style (`error[code]:`, `= note:`, `= help:`). Every `code` and its meaning is listed in `describe`. When ERiC fails, `notes` mention `--eric-log`.

| Exit | Meaning |
|---|---|
| 0 | success |
| 1 | internal error (bug) |
| 2 | usage error |
| 3 | configuration or credentials |
| 4 | ERiC missing, unsupported version, or terms not accepted |
| 5 | ELSTER rejected the request |
| 6 | transient (network, OTTER 500/503); retry makes sense |
| 7 | partial; stdout carries the full result |

### Diagnostics

ERiC logs to a temporary directory deleted at exit (the log may contain tax data); `--eric-log DIR` keeps it.

### Self-description

`describe [command]` prints JSON: commands, flags, types, defaults, exclusive groups, exit codes, error codes, the built-in Bereitstellungsdatenart list, and JSON Schemas for stdout and error output, generated from the Rust types. `--help` stays clap's text. `--format text` rendering (other than errors) is left to the build and not covered by semver.

### `SKILL.md`

Embedded in the binary, printed by `kiecli skill` (same file in the repo). Short and task-shaped: what the tool does and does not do; setup (`eric install`, HerstellerID, Certificate, pointer to config docs); the Postfach loop (`anfrage` → `abholung --out … --bestaetigen`, or `--ohne-bestaetigung` → store → `bestaetigung` within 24 h); the 24 h rule and the HerstellerID block stated plainly; how to react per exit code (retry 6, never retry 5 unchanged, on 7 confirm only what was written, retry `teilweise`); `describe` for reference; the Kontoabfrage.

Checks for the spike added to [Test-system spike: ERiC end to end](11-test-system-spike.md): full Bereitstellungsdatenart list accepted, `neue` vs state 20, error codes for the exit-code mapping.

Amended by [Configuration and secrets](10-config-and-secrets.md): every output also carries `test` and `profile`; new command `config show`; `describe` lists settings.

Amended by [Kontoabfrage requests and responses](13-kontoabfrage-requests.md): `kontoabfrage` subcommands, its output shape and its Fehlerliste-to-exit-code mapping.

Amended by [Certificate inspection](14-certificate-inspection.md): `warnings` entries are objects `{code, message, help?}` with codes in `describe`; new command `zertifikat status` (the preflight step in `SKILL.md`); local Certificate checks and their exit-3 error codes.

Amended by [Test-system spike: ERiC end to end](11-test-system-spike.md): the built-in Bereitstellungsdatenart list is the 49 names ELSTER accepts through ERiC for version 32 (table 9.4 minus 34 refused names, plus 11 DIVA kinds from SSb 9.2/9.3), not table 9.4; no PostfachStatus-first fallback. `postfach status` undercounts and is documented as a hint. Otto returns plaintext (no `EricDekodiereDaten`); `--neue` keeps listing a fetched Nachricht until it is confirmed, and a confirmed one stays fetchable; Bestätigung results are read per id even when ERiC reports an error (some failed: exit 7).

Amended by [Distribution](16-distribution.md): `eric install` and `eric status` test-load ERiC and report a missing `libgcc_s.so.1` as its own exit-4 error code with the distro package in `help`; `SKILL.md` carries the non-affiliation notice, links `docs/getting-started.md` and tells the agent to hand the HerstellerID and Certificate/PIN setup back to the human; `SKILL.md`, `--help`, `describe` and JSON messages stay English only.
