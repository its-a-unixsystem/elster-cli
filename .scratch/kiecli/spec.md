# KIECLI v1 specification

Status: **draft**. Items marked ⏳ wait on [Test-system spike: ERiC end to end](issues/11-test-system-spike.md). [Pending the spike](#15-pending-the-spike) lists all of them. Everything else is decided.

This spec assembles the decisions of the [KIECLI v1 map](map.md). Each section names the ticket that holds the reasoning; where this spec and a ticket disagree, the ticket wins and this spec is wrong. Domain terms follow [`GLOSSARY.md`](../../GLOSSARY.md).

## 1. Product

KIECLI is a stateless, non-interactive command-line client for AI agents. For one Taxpayer per call it:

- reads the Postfach: Anfrage, Abholung of Datenpaket and Anhänge from the Objektspeicher, and Bestätigung on explicit request;
- runs the Kontoabfrage in all three Abfragearten;
- reports on the Certificate it uses.

It is a tool, not tied to any account: the user supplies Certificate, PIN, HerstellerID and Taxpayer data per call or through config. The caller owns persistence and usage policy (who may run it, unattended or not).

### Non-goals in v1

- Sending of any kind (UStVA, ZM, returns, replies to Postfach-Nachrichten): v2.
- Abholung als Vertretung; several Taxpayers per call.
- A local archive or any persisted state.
- Mapping Bescheiddaten to JSON: the Datenpaket is written to disk raw; the mapping is the separate [Bescheiddaten](../bescheid-daten/map.md) effort (v1.x).
- VaSt-Beleg-Daten.
- macOS, Windows, Linux arm64 (ELSTER ships no ERiC for arm64).
- Integration into consuming repositories; policy on agent usage.

## 2. Architecture

Sources: [Go or Rust](issues/08-language.md), [ERiC for the Kontoabfrage](issues/07-eric-decision.md), ADRs [0001](../../docs/adr/0001-eric-for-all-elster-traffic.md) and [0002](../../docs/adr/0002-rust-with-runtime-loaded-eric.md).

1. **Rust**, edition 2024, stable toolchain pinned in `rust-toolchain.toml`, single crate, target `x86_64-unknown-linux-gnu` (not musl: a static musl binary cannot `dlopen` ERiC's glibc libraries).
2. **All ELSTER traffic goes through ERiC** (`libericapi`); Objektspeicher objects through Otto (`libotto`, same package). No self-built transport, no fallback, no Postfach-only mode.
3. **Own runtime binding** with `libloading`: hand-written `extern "C"` function-pointer types and `#[repr(C)]` copies of `eric_druck_parameter_t` and `eric_verschluesselungs_parameter_t`. Nothing derived from ELSTER headers is committed; no build-time dependency on the ERiC package, so the binary starts without ERiC. Functions in use (haben's set plus certificate inspection and Steuernummer conversion):
   - ERiC: `EricInitialisiere`, `EricBeende`, `EricVersion`, `EricBearbeiteVorgang` (ERiC ≥ 44 signature, no transfer handle), `EricGetHandleToCertificate`, `EricCloseHandleToCertificate`, `EricHoleZertifikatEigenschaften`, `EricMakeElsterStnr`, `EricHoleFehlerText`, `EricRueckgabepufferErzeugen`, `EricRueckgabepufferInhalt`, `EricRueckgabepufferFreigeben`, and `EricDekodiereDaten` ⏳ (only if Otto returns encrypted data).
   - Otto: `OttoInstanzErzeugen`, `OttoInstanzFreigeben`, `OttoDatenAbholen`, `OttoHoleFehlertext`, `OttoRueckgabepufferErzeugen`, `OttoRueckgabepufferInhalt`, `OttoRueckgabepufferGroesse`, `OttoRueckgabepufferFreigeben`.
4. **One `eric` module** owns all `unsafe` and gives every handle (Rückgabepuffer, certificate handle, `OttoInstanz`, Otto buffer) a `Drop`. Commands see it only through a trait at ERiC-call level ([Testing strategy](issues/15-testing-strategy.md) item 1):
   - `bearbeite_vorgang(xml, flags) -> raw response XML`
   - `otto_abholen(objektid, &mut Write)`
   - `zertifikat_eigenschaften(pin) -> raw XML`
   - `make_elster_stnr(..)`
   - `version()`

   XML building and parsing live outside the module, in tested code.
5. **One ERiC session per process:** `EricInitialisiere` once, every ERiC and Otto call on the main thread, `EricBeende` before exit. No `EricMt*`, no threads, no worker process. `EricVersion()` is checked before any other call; a new ERiC major adds a signature variant chosen after that check.
6. **Blocking, no async runtime.** The `eric install` download uses `ureq` with `rustls` (no system OpenSSL).
7. **Platform table:** library file names, ERiC package name (`Linux-x86_64`) and the default data directory live in one place, so macOS can be added later without restructuring. Nothing else claims more than Linux x86_64.
8. **Baseline crates:** `clap` (derive), `serde`, `serde_json`, `quick-xml` with serde, `zip`, `sha2`, `directories`, `thiserror`, `libloading`, `ureq`, `schemars`. Smaller choices are left to the build. Every dependency must be AGPL-3.0-compatible (`cargo deny`).
9. **Testability:** `main` is a thin shell around `run(args, env, &dyn Eric) -> (stdout, stderr, exit)`. The release binary has no fake mode and no hidden switch.

## 3. ERiC installation and versions

Source: [ERiC for the Kontoabfrage](issues/07-eric-decision.md), amended by [Distribution](issues/16-distribution.md).

1. `kiecli eric install [--version X] --accept-eric-terms [--sha256 H]`:
   - Anonymous download from `https://download.elster.de/download/eric/eric_<major>/ERiC-<version>-Linux-x86_64.jar`; redirects to other hosts are refused.
   - Extracts only the current platform into a versioned directory under the ERiC directory (`eric_dir`, default the OS data directory) and switches over only after a complete extract.
   - Never runs implicitly. A hand-unpacked ERiC directory also works.
   - The ERiC directory is tooling, not state.
2. **Licence duties (§5):** `--accept-eric-terms` is mandatory. The output links `lizenz.pdf` and the data-protection notice. A marker in the ERiC directory records acceptance, and every command refuses to call ERiC without it (exit 4).
3. **Integrity:** a SHA-256 per tested ERiC version is pinned in the CLI (today `ERiC-44.3.6.0-Linux-x86_64.jar`, `e36df8b02045ff3449eb23e086a39f5455b709c967e708eae44f27099ce4de95`). `--sha256` admits an untested patch version within a supported major.
4. **Versions:** only majors ELSTER currently permits and we have tested: today 44, and 45 once 45.2 ships. Anything else is exit 4, naming the version found and the supported ones. The 44.3.6.0 package's `libericapi.so` reports 44.3.4.0 in `EricVersion`; the version check reads the library, not the package name.
5. **Test-load:** `eric install` and `eric status` load ERiC. A load failure caused by a missing `libgcc_s.so.1` gets its own error code under exit 4, with a `help` naming the package (`libgcc-s1` on Debian/Ubuntu, `libgcc` on Fedora/Arch).
6. Without a usable ERiC, every command that needs it fails with exit 4 and an error naming `eric install`.

## 4. Configuration and secrets

Source: [Configuration and secrets](issues/10-config-and-secrets.md), amended by [Kontoabfrage requests and responses](issues/13-kontoabfrage-requests.md).

### Layers

Precedence: flag > environment (`KIECLI_` + upper-cased key) > config file > built-in default. No OS keychain backend.

| Setting | Flag | Env | Config key | Scope |
|---|---|---|---|---|
| Certificate path | `--certificate` | `KIECLI_CERTIFICATE` | `certificate` | profile |
| PIN | `--pin-file`, `--pin-command` | `KIECLI_PIN`, `KIECLI_PIN_FILE`, `KIECLI_PIN_COMMAND` | `pin_file`, `pin_command` | profile |
| Testmerker | `--test` / `--no-test` | `KIECLI_TEST=0\|1` | `test` | profile |
| Steuernummer (ELSTER or Bescheid format) | `--steuernummer` | `KIECLI_STEUERNUMMER` | `steuernummer` | profile |
| Bundesland (only for a Bescheid-format Steuernummer) | `--bundesland` | `KIECLI_BUNDESLAND` | `bundesland` | profile |
| HerstellerID | `--hersteller-id` | `KIECLI_HERSTELLER_ID` | `hersteller_id` | top-level |
| ERiC directory | `--eric-dir` | `KIECLI_ERIC_DIR` | `eric_dir` | top-level |
| Profile | `--profile` | `KIECLI_PROFILE` | `default_profile` | top-level |
| Config file | `--config` | `KIECLI_CONFIG` | | |

- Flag-only: `--format`, `-v`, `--eric-log`.
- Live by default; `test` puts the Testmerker on every request. The Testmerker values differ by procedure: Kontoabfrage `230000001`, Postfach `700000001` ⏳ (confirm which Postfach Testmerker the test system expects through ERiC).
- The HerstellerID is validated as five digits; it is never in code or the repository.

### PIN

1. Only via `KIECLI_PIN`, a PIN file or a PIN command. Never a `--pin` flag, never a literal PIN in the config file, never a prompt, even on a TTY. No PIN available: exit 3, with an error listing the three ways.
2. **PIN file:** contains only the PIN; one trailing newline is stripped. If group or others can read it: warning `pin_file_permissions`, not a refusal.
3. **PIN command:** run via `/bin/sh -c` with stdin `/dev/null`; the PIN is stdout minus one trailing newline. Empty output, a non-zero exit or a 30 s timeout is exit 3, with the captured stderr in `cause`. It runs only for commands that open the Certificate (not `eric install|status`, `describe`, `skill`, `config show`).
4. The PIN is held in a zeroizing buffer and dropped after ERiC and Otto have used it; it is never echoed in any output.

### Config file

1. TOML at `$XDG_CONFIG_HOME/kiecli/config.toml` (default `~/.config/kiecli/config.toml`).
   - A missing default file is fine. A missing file named by `--config` or `KIECLI_CONFIG` is exit 3.
   - Unknown keys are exit 3, with a did-you-mean `help`.
   - `~` is expanded. Relative paths resolve against the config file's directory; paths from flags and env resolve against the working directory.
2. `[profile.NAME]` holds the per-Taxpayer keys. `hersteller_id`, `eric_dir` and `default_profile` are top-level, and per-Taxpayer keys are not allowed at top level.

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

3. **Profile selection:** `--profile` > `KIECLI_PROFILE` > `default_profile` > the only profile in the file. With several profiles and none chosen: exit 3, listing the names. Flags and env override single fields of the selected profile; when an env var overrides a field the profile sets, warning `env_overrides_profile` names the field.
4. A missing required setting is exit 3, and the error's `notes` name its flag, env var and config key. A Certificate path that does not exist or cannot be read is exit 3 before ERiC starts.

## 5. Commands

Sources: [Agent interface](issues/09-agent-interface.md), [Kontoabfrage requests and responses](issues/13-kontoabfrage-requests.md), [Certificate inspection](issues/14-certificate-inspection.md), [Configuration and secrets](issues/10-config-and-secrets.md).

```
kiecli postfach status        [--datenart X…] [--neue|--alle] [--von D] [--bis D]
kiecli postfach anfrage       [--datenart X…] [--neue|--alle] [--von D] [--bis D]
kiecli postfach abholung      (--id ID… | --neue) --out DIR (--bestaetigen | --ohne-bestaetigung)
                                  [--datenart X…] [--mit-meta]
kiecli postfach bestaetigung  --id ID…
kiecli kontoabfrage offene-betraege
kiecli kontoabfrage zahlungen      --ab D [--ein-jahr] [--steuerart X…]
kiecli kontoabfrage sollstellungen --steuerart X… --jahr YYYY…
kiecli zertifikat status
kiecli eric install [--version X] --accept-eric-terms [--sha256 H]
kiecli eric status
kiecli config show [--profile X]
kiecli describe [command]
kiecli skill
```

**Global flags:** `--format json|text` (default `json`), `-v`, `--eric-log DIR`, plus the settings flags in §4. No command ever reads stdin interactively. Commands are named after ELSTER's lifecycle steps in German, ASCII-folded.

| Command | Opens Certificate (PIN) | Contacts ELSTER | Needs HerstellerID | Needs ERiC |
|---|---|---|---|---|
| `postfach *`, `kontoabfrage *` | yes | yes | yes | yes |
| `zertifikat status` | yes | no | no | yes |
| `eric install`, `eric status` | no | download only (`install`) | no | `status` reports it |
| `config show`, `describe`, `skill` | no | no | no | no |

### 5.1 Postfach

**Bereitstellungsdatenart selection.** A built-in list of every name in SSb table 9.4 goes into one PostfachAnfrage, and `--datenart` narrows it. Names are case-sensitive; an unknown name is exit 2 (`datenart_unknown`, with a did-you-mean `help` and a corrected `suggestion`). ⏳ If the test system rejects kinds the account never receives, `anfrage` sends a PostfachStatus first and asks only for kinds with hits.

**Defaults:** `--neue`. All pages are fetched internally (`paginierungLetzteBestId`, up to 10,000 per page) and merged; no cursor is exposed. `--von` and `--bis` are ISO dates.

**`postfach status`:** PostfachStatus, the hit count per Bereitstellungsdatenart without Abholung.

**`postfach anfrage`:** PostfachAnfrage, the metadata only. It never creates the Bestätigung obligation.

**`postfach abholung`:**

1. Runs its own PostfachAnfrage to resolve object IDs, sizes and names, so the caller passes only ids or `--neue`.
2. Requires exactly one of `--bestaetigen` or `--ohne-bestaetigung`; with neither it is exit 2. Nothing is confirmed implicitly, and nothing is fetched until the caller has decided.
3. `--out DIR` is mandatory; no content goes to stdout. Files are laid out as in §7.
4. Fetches every object of each Postfach-Nachricht through Otto ⏳ (whether Otto returns plain, encrypted or gzipped data, and whether `EricDekodiereDaten` is needed; how Abholung works on the test system, which has no documented Objektspeicher host).
5. `--bestaetigen` confirms only Nachrichten whose every file was written and fsynced; the others are reported unconfirmed.
6. `--ohne-bestaetigung` gives each Nachricht `bestaetigung_faellig_bis` (Abholung time + 24 h, RFC 3339 UTC) and warning `bestaetigung_pending`. The caller confirms later with `postfach bestaetigung`.
7. A **partial Abholung** (some objects fetched, one failed) counts as started: the Nachricht's status is `teilweise`, it is not confirmed, it carries `bestaetigung_faellig_bis`, and the run exits 7.
8. ⏳ Whether `--neue` still lists a Nachricht that was fetched but not confirmed (state 20 ABGEHOLT), and whether a Nachricht stays listed and fetchable after Bestätigung.

**`postfach bestaetigung`:** PostfachBestaetigung, with ids sent in blocks of 100.

### 5.2 Kontoabfrage

1. One Abfrageart per call: `offene-betraege` (`O`), `zahlungen` (`I`), `sollstellungen` (`ZS`). Mixing Abfragearten in one request is not offered.
2. `zahlungen`:
   - `--ab` (ISO date, sent as `TTMMJJJJ`) is required;
   - mode `V` (until today) by default, `--ein-jahr` sends `J`;
   - `--steuerart` defaults to `alle`.
3. `sollstellungen`: `--steuerart` and `--jahr` are both required, and `alle` is not allowed. Repeated values fan out into the cartesian product of `kontoabfrage-teil` in one request; more than 75 Teile is exit 2.
4. Steuerart is matched case-insensitively and sent in ELSTER spelling: `ESt`, `KSt`, `USt`, `LSt`, `GewSt`, `ZaSt`, `KapESt`. GewSt outside the Stadtstaaten is left to ELSTER to reject.
5. **Steuernummer:** exactly one per call, from the `steuernummer` setting.
   - A 13-digit ELSTER-format number is used as is.
   - A Bescheid-format number is converted with `bundesland` through `EricMakeElsterStnr` (exported by ERiC 44, see the spike's pre-findings). For Bavarian and Berlin `BBB/UUUUP` numbers the Bundesfinanzamtsnummer is mandatory.
   - A Bescheid-format number without `bundesland` is exit 2, with a `help` naming the flag.
   - The Empfänger (Bundesfinanzamtsnummer, the first four digits) is always derived from the Steuernummer, never a setting.
6. `--mit-xml` adds the raw `kontoabfrage` Nutzdaten as one string (§6.3).
7. ⏳ Which test Steuernummern the test system accepts (examples use Finanzamt 9238), which response shape the test Finanzamt returns, and whether the test Organisationszertifikat gets a result or `230077012`.

### 5.3 `zertifikat status`

1. Opens the Certificate with the PIN, calls `EricHoleZertifikatEigenschaften`, and sends nothing to ELSTER. It needs no HerstellerID.
2. It is the credentials preflight: `SKILL.md` names it as the first call of a session.
3. An expired or not-yet-valid Certificate still exits 0, with `abgelaufen: true` and warning `certificate_expired`; reporting is the command's job.
4. Output in §6.3.

### 5.4 `eric status`

Reports the ERiC directory, the version found, whether it is supported (with the supported majors), whether the terms are accepted, the library paths, and the result of the test-load (§3.5). No network, no Certificate.

### 5.5 `config show`

Prints each resolved setting with its value and its source (`flag`, `env`, `file:<path>`, `default`). The PIN shows only its channel. It never runs the PIN command and never touches ERiC or the Certificate.

### 5.6 `describe` and `skill`

- `describe [command]` prints JSON generated from the Rust types:
  - commands, flags, types, defaults and exclusive groups;
  - settings, each with its flag, env var, config key, scope and default;
  - exit codes, and every error and warning code with its meaning;
  - the built-in Bereitstellungsdatenart list, the Steuerart values and the Kontoabfrage result codes;
  - JSON Schemas for stdout and error output.

  `--help` stays clap's text.
- `skill` prints the embedded `SKILL.md` (§11).

## 6. Output (stdout)

### 6.1 Conventions

1. One flat JSON object per command, no `data` wrapper. Every output carries:
   - `warnings: []`;
   - `test` (bool);
   - `profile` (name, or null when no profile is selected).
2. JSON keys: glossary and ELSTER terms in German snake_case (`bereitstellungsdatenart`, `verfuegbar_bis`), generic fields in English (`id`, `path`, `warnings`). ELSTER `meta` keys are passed through raw.
3. Dates are ISO 8601, timestamps RFC 3339 UTC. Amounts are decimal strings with a dot and no thousands separator (`"3782.00"`), never JSON numbers.
4. `-v` adds a `trace` array (ERiC calls, durations, pages).
5. `--format text` renders a human view. Its layout is left to the build and is not covered by semver; error rendering is fixed (§8).
6. **Schema stability:** the output is versioned with the CLI's semver. A removed or renamed property, a changed type, a removed enum value, or a removed error or warning code is a major change; additions, including new enum values and codes, are minor. Before 1.0.0 a minor bump may break output (§13.4). Agents must tolerate unknown codes and enum values.

### 6.2 Warnings

Each entry is an object `{"code", "message", "help"?}`, mirroring the error object; every code is listed in `describe`. Codes defined so far:

| Code | When |
|---|---|
| `certificate_expires_soon` | `gueltig_bis` is less than 60 days away (fixed threshold); added by every command that opens the Certificate |
| `certificate_expired` | `zertifikat status` only, on an expired or not-yet-valid Certificate |
| `pin_file_permissions` | the PIN file is readable by group or others |
| `env_overrides_profile` | an env var overrides a field set by the selected profile |
| `bestaetigung_pending` | an Abholung left Nachrichten unconfirmed |

### 6.3 Per command

- **`postfach status`:** `bereitstellungsdatenarten: [{name, anzahl}]`, `einschraenkung`.
- **`postfach anfrage`:** `postfach_nachrichten`, sorted by `id`. Each entry has:
  - `id`, `bereitstellungsdatenart`, `bereitstellungsticket`, `verfuegbar_bis`, `groesse`;
  - `meta` (raw name/value);
  - `anschreiben` (`sendername`, `bezeichnung`, `text` decoded from base64, or null);
  - `meta_maps` (other MetaInfoMaps unchanged);
  - `datenpaket` (`objektid`, `encoding`, `groesse`);
  - `anhaenge`, each with `position`, `dateiname`, `dateibezeichnung`, `dateityp`, `objektid`, `groesse`, `meta`.
- **`postfach abholung`:** one entry per Nachricht with:
  - `id`;
  - `status` (`vollstaendig` | `teilweise` | `fehlgeschlagen`);
  - `abgeholt_am`;
  - `files`, each with `path`, `groesse`, `sha256`, `role` (`datenpaket` | `anhang`) and `position`;
  - `bestaetigt`;
  - `bestaetigung_faellig_bis` (while unconfirmed);
  - `fehler` (the error shape of §8).

  `--mit-meta` adds the `anfrage` fields.
- **`postfach bestaetigung`:** per id, `bestaetigt` and ELSTER's `rueckgabe {code, text}`.
- **`kontoabfrage *`:**
  - Top level: `steuernummer` (ELSTER format), `bundesfinanzamtsnummer`, `abfrageart`, `teile[…]`.
  - Each `teil`: its echoed input (`steuerart`, `ab` + `ein_jahr`, or `jahr`), `bearbeitungshinweis {code, text}`, and `antwort_format: "biene" | "alt"`. Teile are matched to the request by their echoed input, not by position, because ELSTER may reorder them.
  - Each response shape is mapped faithfully, in delivered order, with no merged model:
    - `alt` (`kontoabfrage-output`): `abfrage_tagesdatum`, `steuerarten[{klartext, erlaeuterung, teilbetraege[{zeitraum, faelligkeit, wert, erlaeuterung}], gesamtbetrag}]`, `gesamtsumme`, `erlaeuterung`.
    - `biene` (`kontoabfrage_antwort`): `stpfl {finanzamt, name, steuernummer, abfragedatum, kein_leistungsgebot}`, `zeilen[…]` and `summen[…]` with the fields of `steuerkontoOffeneBetraege` / `steuerkontoTilgungen` / `steuerkontoauszug` (`text`, `faelligkeit_wertstellung`, `betrag`, `zufluss`, `abfluss`, `offen_forderung`, `offen_verbindlichkeit`, `erlaeuterungen`). Text-only heading rows stay rows.
  - Parseable dates become ISO 8601. The BIENE `abfragedatum` has no timezone and stays local time without `Z`. Free-text periods (`4.Vj.2019`) stay text.
  - An empty account is success with empty lists.
  - `--mit-xml` adds `xml`.
- **`zertifikat status`:**
  - Top level: `gueltig_ab`, `gueltig_bis` (the earlier of the two blocks), `tage_verbleibend`, `abgelaufen`, `testzertifikat`, `registrierertyp`, `organisationszertifikat` (`Registrierertyp == Organisation`), `token_typ`, `konto_id` (the subject CN).
  - `verschluesselungszertifikat` and `signaturzertifikat` (null when ERiC omits it), each with `gueltig_ab`, `gueltig_bis`, `signaturalgorithmus`, `schluessellaenge`, `public_key_sha1`, `issuer` and `subjekt` (name/value lists), `identifikationsmerkmaltyp`, `verifikationsart`.
  - ERiC's UTCTime is converted to RFC 3339 UTC. The MD5 fingerprint is not reported.
- **`eric install`:** the installed version and directory, and links to `lizenz.pdf` and the data-protection notice.
- **`eric status`, `config show`:** as in §5.4 and §5.5.

## 7. Files on disk

Source: [Agent interface](issues/09-agent-interface.md).

```
<out>/<nachricht-id>/datenpaket.<xml|txt>
<out>/<nachricht-id>/anhaenge/<NN>-<sanitised DateinameMitErweiterung>
```

- `NN` is the Anhang position, because ELSTER file names are not unique.
- Each file is written to a temp file in the same directory, fsynced, then renamed; existing files are replaced.
- The Datenpaket is written byte for byte as delivered (ISO-8859-15 or UTF-8), and its `encoding` is reported in JSON.
- No metadata file is written to disk.

## 8. Errors and exit codes

Sources: [Agent interface](issues/09-agent-interface.md), [Kontoabfrage requests and responses](issues/13-kontoabfrage-requests.md), [Certificate inspection](issues/14-certificate-inspection.md).

stderr carries exactly one JSON error object on failure, and never progress or log lines:

```json
{"error": {"code": "datenart_unknown", "message": "unknown Bereitstellungsdatenart `DivaBescheidUst`",
  "notes": ["Bereitstellungsdatenart names are case-sensitive"],
  "help": "did you mean `DivaBescheidUSt`?",
  "suggestion": "kiecli postfach anfrage --datenart DivaBescheidUSt",
  "cause": [{"source": "eric|elster|otto", "code": 610301202, "text": "…"}],
  "retry": false}}
```

- `suggestion` is a corrected full command, when one exists.
- `cause` passes ERiC, ELSTER and Otto codes and texts verbatim.
- When ERiC fails, `notes` mention `--eric-log`.
- `--format text` renders the error rustc-style (`error[code]:`, `= note:`, `= help:`).
- All messages are English.

| Exit | Meaning |
|---|---|
| 0 | success |
| 1 | internal error (bug) |
| 2 | usage error |
| 3 | configuration or credentials |
| 4 | ERiC missing, unsupported version, terms not accepted, or not loadable |
| 5 | ELSTER rejected the request |
| 6 | transient (network, OTTER 500/503); a retry makes sense |
| 7 | partial; stdout carries the full result |

### 8.1 Certificate checks

Every command that opens the Certificate calls `EricHoleZertifikatEigenschaften` once after opening it. Networked commands then check locally, before any request:

| Condition | Code | Exit |
|---|---|---|
| `ERIC_CRYPT_E_PIN_WRONG` (610201106) | `pin_wrong` | 3 |
| `ERIC_CRYPT_E_PIN_LOCKED` (610201107) | `pin_locked` | 3 |
| other `ERIC_CRYPT_*` (610201101–610201212) | `certificate_unreadable` | 3 |
| `gueltig_bis` in the past | `certificate_expired`, `help`: renew in Mein ELSTER | 3 |
| `gueltig_ab` in the future | `certificate_not_yet_valid` | 3 |
| `testzertifikat` true and the call runs live | `test_certificate_requires_testmerker`, `suggestion` adds `--test` | 3 |

All have `retry: false`. KIECLI never switches to test mode silently; a real Certificate with `--test` is allowed. ERiC has no error code for an expired Certificate, so the local check also spares the caller a server-side rejection.

### 8.2 Kontoabfrage result codes

| ELSTER code (Fehlerliste 2.1.1) | Exit |
|---|---|
| `0` on every Teil | 0 |
| `950013001` (some Teile failed) | 7; stdout lists each Teil with its code |
| `950013002`, `230077012`, `950013003`, `950017004`, `950017005`–`950017007`, `950017011` | 5 |
| `950017009`, `950017010`, `950019999` | 5, `help`: contact the Finanzamt |
| `950019011`, `950017008` | 6, `retry: true` |

For `230077012`, `notes` explain that the Kontoabfrage presupposes an authorisation (Vollmacht or Freischaltung in Mein ELSTER) ⏳ (how a real account clears `230077012`/`950017011`).

### 8.3 Other mappings

- Missing settings, PIN channel failures and unreadable config: exit 3 (§4).
- A missing ERiC, an unsupported version, terms not accepted, or a missing `libgcc_s.so.1`: exit 4 (§3).
- ⏳ The ERiC and Otto codes for a missing or blocked HerstellerID (610301202 locally for `74931`), a wrong PIN on the wire, and an OTTER 503, and the remaining ERiC/ELSTER codes of the Postfach procedures, mapped to 5 or 6.
- **Error-code names:** codes the tickets name are used as written: `datenart_unknown`, the certificate codes in §8.1, and `test_certificate_requires_testmerker`. Codes for the other conditions in §3–§5 are named in the build, in the same style (snake_case, glossary terms in German, otherwise English), and are all listed in `describe`.

## 9. Diagnostics

ERiC logs to a temporary directory that is deleted at exit, because the log may contain tax data. `--eric-log DIR` keeps it.

## 10. Privacy and the public repository

- Nothing from the ELSTER downloads goes into the repository, the crate or the release artifacts: no ERiC, no schemas, no `Beispiele`, no test certificates, no `extern-offen.p12`.
- No real HerstellerID, Certificate, Konto-ID, Steuernummer or tax data in code, fixtures, tickets or docs.
- Each user supplies their own HerstellerID and owns the obligations attached to it, including the 24 h Bestätigung (a missed Bestätigung risks a block of the HerstellerID).

## 11. `SKILL.md`

Embedded in the binary, printed by `kiecli skill`, with the same file in the repo. English only. Short and task-shaped:

1. The non-affiliation notice (§13.6). What the tool does and does not do.
2. **Setup:** `eric install --accept-eric-terms`, then `eric status`. The agent stops and hands two steps back to the human, because they need a person: obtaining a HerstellerID, and setting up the Certificate and PIN source. Link `docs/getting-started.md` and the config docs.
3. **Preflight:** `zertifikat status` as the first call of a session.
4. **The Postfach loop:** `anfrage` → `abholung --out … --bestaetigen`; or `--ohne-bestaetigung` → store the files → `bestaetigung` within 24 h. State the 24 h rule and the HerstellerID block plainly.
5. **Reacting to exit codes:**
   - retry on 6, never retry on 5 unchanged;
   - on 7, confirm only what was written;
   - retry a `teilweise` Abholung for that id, then confirm; if retries keep failing, confirm before the deadline anyway.
6. Tolerate unknown warning and error codes and enum values.
7. The Kontoabfrage, including what `230077012` means ⏳.
8. `describe` for reference.

## 12. Testing

Source: [Testing strategy](issues/15-testing-strategy.md), amended by [Distribution](issues/16-distribution.md).

### 12.1 Layers

1. **Command logic against an in-process fake** of the `eric` trait. The fake is scriptable per test (a response or an ERiC/Otto code per call) and records every call, so tests assert the request XML, Testmerker and HerstellerID sent. It simulates:
   - ERiC and Otto return codes;
   - Otto streaming in chunks, and a failure after N bytes on object K (partial Abholung, exit 7);
   - PIN and certificate errors, and Certificate properties;
   - Kontoabfrage answers in biene and alt.
2. **`assert_cmd` on the real binary** covers only what needs no ERiC: usage errors, `describe`, `skill`, `config show`, `eric status` without ERiC (exit 4).
3. **The binding against real ERiC, offline:** `EricVersion`, init and teardown, `EricHoleZertifikatEigenschaften` with the test certificates (PIN 123456), `EricMakeElsterStnr`, error texts, and `ERIC_VALIDIERE` of golden requests. There are no C mocks. ERiC validates locally with any five-digit HerstellerID except the blocked `74931`, so these tests use the placeholder `12345` and run on fork PRs. Otto has no offline path and is covered only by test-system runs.
4. **The ELSTER test system** behind a gated job (§12.4).

### 12.2 Fixtures and downloads

1. `tests/fixtures/`:
   - `elster/`: PostfachStatus, PostfachAnfrage and Bestätigung answers, Kontoabfrage biene and alt, Fehlerliste-shaped error answers;
   - `eric/`: `EricHoleZertifikatEigenschaften`, `EricVersion`;
   - `requests/`: golden request XML that the builders must produce byte for byte.
2. Fixtures are hand-written with invented data: test Finanzamt 9198/9238 Steuernummern, invented tickets and objektids, Konto-IDs from a reserved invented range (prefix `TEST`). Test-system captures may shape fixtures but are never committed verbatim: identifiers are rewritten and the result reviewed.
3. Response fixtures are validated against ELSTER's XSDs with `uppsala`. Golden requests are validated by real ERiC (`ERIC_VALIDIERE`, nothing sent).
4. The ERiC jar, `Test_Zertifikate.zip` and the schema zips come from `download.elster.de` at test time, each pinned by SHA-256 in `tests/downloads.toml`; a mismatch fails loudly.
   - Cache: `$XDG_CACHE_HOME/kiecli-test/<sha256>/`. ERiC is installed there with `kiecli eric install --eric-dir …`.
   - Tests needing a download skip when offline; `KIECLI_TEST_REQUIRE_DOWNLOADS=1` (set in CI) turns a skip into a failure.

### 12.3 Contract tests

1. `describe` output and the stdout and stderr JSON Schemas are generated with `schemars` and committed as `schemas/*.json`; a test fails when the generated files differ.
2. A semver test compares `schemas/` with the last release tag and requires a major bump for the breaking changes in §6.1.
3. Every command output from the fake-driven tests is validated against its schema.

### 12.4 CI (GitHub Actions, `its-a-unixsystem/kiecli`)

| Job | When | Secret | Runs |
|---|---|---|---|
| `checks` | every push and PR, forks included | none | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`, `cargo deny check` (AGPL-compatible licence allowlist, RustSec), DCO `Signed-off-by` on every commit |
| `eric-local` | every push and PR, forks included | none | real ERiC from the cache: binding tests, golden requests with placeholder `12345`, fixtures against the XSDs, a canary asserting `74931` → 610301202 |
| `test-system` | manual, release gate, weekly | `KIECLI_HERSTELLER_ID` | `cargo test --features test-system` and the leak guard |

- `ubuntu-latest` only, toolchain from `rust-toolchain.toml`, no MSRV matrix, no coverage gate.
- `KIECLI_HERSTELLER_ID` exists only as an environment secret:
  - `elster-test`: `main` and release tags, maintainer as required reviewer;
  - `elster-test-scheduled`: `main` only, no reviewer, for the weekly run.

  Never `pull_request_target`, never PR code checked out with the secret.
- **Test-system runs** (Testmerker on every request):
  - Always: `zertifikat status`, `postfach status`, `postfach anfrage`, all three `kontoabfrage` kinds, and the live-call refusal.
  - ⏳ `postfach abholung --bestaetigen` and `postfach bestaetigung` join only if the test Postfach is safe for them (shared by every Hersteller? does it count toward our 24 h obligation?). Never `--ohne-bestaetigung` without a Bestätigung in the same run.
  - ⏳ If GitHub's runners cannot reach the test system, the same command runs on the maintainer's machine before each release, and the CI triggers are dropped.
- **Leak guard:** whenever `KIECLI_HERSTELLER_ID` is set, a test scans every tracked file for that value and for the subject CN of the Certificate in `KIECLI_CERTIFICATE`, and fails on a hit.

### 12.5 Final live check

1. The maintainer runs it with their own Certificate, before 1.0.0 and before any release that changes the ERiC major or the request XML.
2. It follows a checked-in `docs/live-check.md`: commands and the expected shape of each result, no values. It includes one real `postfach abholung --bestaetigen` of an already-known Nachricht, with `--out` outside the repo.
3. The only record is a ticked checklist in the release PR: CLI and ERiC versions, and pass or fail per step.

## 13. Build, release and distribution

Sources: [Distribution](issues/16-distribution.md), ADR [0003](../../docs/adr/0003-agpl-with-eric-exception.md).

1. **Build:** `cargo-zigbuild --target x86_64-unknown-linux-gnu.2.25` on `ubuntu-latest`. A release step runs `objdump -T` and fails on any `GLIBC_` symbol newer than 2.25 (ERiC 44.3.6.0's own baseline). `libgcc_s.so.1` is a documented system requirement and is never shipped.
2. **Release workflow** (hand-written, triggered by a `vX.Y.Z` tag), in order:
   1. `checks` + `eric-local`;
   2. `test-system` (environment `elster-test`, required);
   3. zigbuild + glibc guard;
   4. draft GitHub Release, then publish;
   5. `cargo publish`.
3. **Artifacts:** `kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz` (binary, `LICENSE`, `NOTICE`, `README.md`, `README.de.md`), plus a `SHA256SUMS` file and GitHub artifact attestations (Sigstore build provenance). No long-lived signing key.
   - crates.io: crate `kiecli`, with an explicit `include` allowlist in `Cargo.toml`.
   - `[package.metadata.binstall]` points at the tarball.
   - No `curl | sh` script and no distro packages.
4. **Versioning:** `0.y.z` until the live check passes; 1.0.0 is the first release with the JSON stability promise. `CHANGELOG.md` follows Keep a Changelog, and every JSON Schema change needs an entry.
5. **Licence:** `AGPL-3.0-only`, plus a GPLv3 §7 additional permission to combine the program with ERiC and Otto as published by ELSTER and convey the result. The text is in `LICENSE` and `NOTICE`. Contributions come in with a DCO sign-off and no CLA.
6. **Non-affiliation notice**, fixed text, at the top of both READMEs, in `NOTICE`, in `SKILL.md` and in the crates.io `description`; never in `--version` or JSON. No logo or ELSTER imagery. The wording is checked before the first release:
   > KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.
7. **Docs:**
   - Bilingual, with English canonical: `README.md`/`README.de.md` and `docs/getting-started.md`/`docs/getting-started.de.md`, each with 🇬🇧 | 🇩🇪 links at the top; the German files say that English wins on disagreement. Sync is asked for in `CONTRIBUTING.md` and by a PR-template checkbox.
   - English only: `SKILL.md`, `--help`, `describe`, JSON messages, ADRs, `CHANGELOG.md`, `CONTRIBUTING.md`, `docs/live-check.md`.
8. **`docs/getting-started.md`**, in order:
   1. System requirements: Linux x86_64, glibc ≥ 2.25, `libgcc_s.so.1`.
   2. Install: tarball (verify `SHA256SUMS` or the attestation), `cargo binstall` or `cargo install`.
   3. `eric install --accept-eric-terms`, then `eric status`.
   4. Own HerstellerID: Entwicklerbereich registration (login after a few days), then the HerstellerID application in the developer portal naming the product and interfaces (ERiC, Sonstige); the ID can take a week. A product name containing "elster" may be refused ([Obtain a HerstellerID](issues/06-obtain-herstellerid.md)).
   5. Certificate and PIN setup (§4).
   6. `zertifikat status`.
   7. `--test` against the test system.

## 14. Repository layout

Fixed by the decisions above; the build is free to add more.

```
Cargo.toml  rust-toolchain.toml  deny.toml
LICENSE  NOTICE  README.md  README.de.md  CHANGELOG.md  CONTRIBUTING.md  SKILL.md
docs/getting-started.md  docs/getting-started.de.md  docs/live-check.md  docs/adr/
schemas/*.json
src/            # eric module (all unsafe), commands, XML builders and parsers
tests/fixtures/{elster,eric,requests}/  tests/downloads.toml
.github/workflows/  .github/pull_request_template.md
```

## 15. Pending the spike

Each item names where its answer lands in this spec.

| Question | Lands in |
|---|---|
| Whether a PostfachAnfrage naming every Bereitstellungsdatenart from SSb table 9.4 is accepted; otherwise PostfachStatus first | §5.1 |
| Whether PostfachStatus behaves as documented through ERiC | §5.1 |
| What Otto returns (plain, encrypted, gzip); whether `EricDekodiereDaten` is needed; how Abholung works on the test system | §2.3, §5.1 |
| Whether `--neue` lists state 20 ABGEHOLT; whether Nachrichten stay listed and fetchable after Bestätigung | §5.1 |
| The Postfach Testmerker value through ERiC (`700000001` or another) | §4 |
| ERiC/Otto/ELSTER codes for a missing HerstellerID, a wrong PIN, an OTTER 503 and other Postfach failures | §8.3 |
| An `O` Kontoabfrage on the test system: accepted test Steuernummern, the response shape, `230077012` for the test Organisationszertifikat | §5.2, §6.3 |
| How a real account clears `230077012`/`950017011` | §8.2, §11 |
| `EricHoleZertifikatEigenschaften` on the test certificates (both blocks, `Testzertifikat`, `Registrierertyp`, CN shape) | §6.3, fixtures |
| That a test certificate sent live is rejected | §8.1 |
| Whether the test Postfach is shared and safe for Abholung and Bestätigung in recurring runs | §12.4 |
| Whether GitHub runners reach the test system | §12.4 |

## 16. Sources

| Ticket | Sections |
|---|---|
| [Postfach lifecycle and attachments](issues/02-postfach-lifecycle.md) | §5.1 |
| [ELSTER terms for an open-source client](issues/04-open-source-terms.md), [Ask ELSTER the open terms questions](issues/12-ask-elster.md) | §10, §13.6 |
| [ERiC for the Kontoabfrage](issues/07-eric-decision.md) | §2, §3 |
| [Go or Rust](issues/08-language.md) | §2 |
| [Agent interface](issues/09-agent-interface.md) | §5–§9, §11 |
| [Configuration and secrets](issues/10-config-and-secrets.md) | §4, §5.5 |
| [Obtain a HerstellerID](issues/06-obtain-herstellerid.md) | §13.8 |
| [Kontoabfrage requests and responses](issues/13-kontoabfrage-requests.md) | §5.2, §6.3, §8.2 |
| [Certificate inspection](issues/14-certificate-inspection.md) | §5.3, §6.2, §8.1 |
| [Testing strategy](issues/15-testing-strategy.md) | §2.4, §12 |
| [Distribution](issues/16-distribution.md) | §3.5, §13 |
| [Open-interface transport, end to end](issues/01-open-interface-transport.md), [ERiC and the Kontoabfrage](issues/03-eric-kontoabfrage.md), [Go vs Rust library support](issues/05-go-vs-rust-libraries.md) | background only (superseded by the ERiC decision) |
