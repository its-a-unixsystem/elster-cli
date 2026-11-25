# Certificate inspection

Type: grilling
Status: resolved
Blocked by:

## Question

How does KIECLI show what Certificate it is using and warn before it expires? ELSTER certificates last about three years, and an expired one fails every call. ERiC offers `EricHoleZertifikatEigenschaften`; [Configuration and secrets](10-config-and-secrets.md) keeps `config show` from opening the Certificate, leaving this open.

Settle:

- the command: a new `zertifikat` command (e.g. `zertifikat zeigen`), part of `eric status`, or a section added to `config show` behind a flag;
- which properties it reports (subject, issuer, validity, Organisationszertifikat or personal, serial) and which it must not, given the public-repo and privacy rules;
- whether opening the Certificate needs the PIN (and so runs the PIN command), and what that means for agents calling it often;
- the expiry warning: threshold, and whether every command that opens the Certificate adds it to `warnings` or only the inspection command does;
- the error when the Certificate has already expired or the PIN is wrong, mapped to the existing exit codes from [Agent interface](09-agent-interface.md).

Whether `EricHoleZertifikatEigenschaften` needs the PIN and which fields it returns are facts for [Test-system spike: ERiC end to end](11-test-system-spike.md) or the ERiC documentation, not decisions; resolve them before or during the grilling.

## Answer

**A new local `zertifikat status` command, an expiry warning on every command that opens the Certificate, and local checks that fail before ELSTER is contacted.** Term Konto-ID added to `GLOSSARY.md`.

### `zertifikat status`

1. New command `kiecli zertifikat status`, honouring `--profile`, `--certificate` and the PIN settings like every other command. Not part of `eric status` (no Certificate there) or `config show` (no PIN command there).
2. Local only: opens the Certificate with the PIN, calls `EricHoleZertifikatEigenschaften`, sends nothing to ELSTER, needs no HerstellerID. It doubles as the credentials preflight; `SKILL.md` names it as the first step of a session.
3. Output, plus `test`, `profile` and `warnings` as on every command:
   - top level: `gueltig_ab`, `gueltig_bis` (the earlier of the two blocks), `tage_verbleibend`, `abgelaufen`, `testzertifikat`, `registrierertyp`, `organisationszertifikat` (`Registrierertyp == Organisation`), `token_typ`, `konto_id`;
   - `verschluesselungszertifikat` and `signaturzertifikat` (null when ERiC omits it), each with `gueltig_ab`, `gueltig_bis`, `signaturalgorithmus`, `schluessellaenge`, `public_key_sha1`, `issuer` and `subjekt` (name/value lists), `identifikationsmerkmaltyp`, `verifikationsart`.
   ERiC's UTCTime becomes RFC 3339 UTC. Not reported: the MD5 fingerprint, the file's bytes.
4. **Konto-ID** (subject CN of an ELSTER software Certificate) is shown by default as `konto_id`: the user's own data about their own file, and the key for matching a renewed Certificate to the old one. Fixtures use invented IDs only.
5. Expired or not yet valid: `zertifikat status` still exits 0, with `abgelaufen: true` and a warning; reporting is its job.

### Expiry warning

6. Every command that opens the Certificate (`postfach anfrage|abholung|bestaetigung|status`, `kontoabfrage *`, `zertifikat status`) calls `EricHoleZertifikatEigenschaften` once after opening it and adds warning `certificate_expires_soon` when `gueltig_bis` is less than **60 days** away. Fixed threshold, not a setting; callers wanting another read `tage_verbleibend`. No new output field on the other commands.
7. **`warnings` entries are objects** `{"code", "message", "help"?}`, mirroring the error object; every warning code is listed in `describe`. Known codes so far: `certificate_expires_soon`, `certificate_expired` (status only), `pin_file_permissions`, `env_overrides_profile`, `bestaetigung_pending`. Amends [Agent interface](09-agent-interface.md).

### Errors

All exit 3 (configuration or credentials), `retry: false`, ERiC code in `cause`:

| Condition | Code | Where |
|---|---|---|
| `ERIC_CRYPT_E_PIN_WRONG` (610201106) | `pin_wrong` | any command opening the Certificate |
| `ERIC_CRYPT_E_PIN_LOCKED` (610201107) | `pin_locked` | same |
| other `ERIC_CRYPT_*` (610201101–610201212) | `certificate_unreadable` | same |
| `gueltig_bis` in the past | `certificate_expired`, `help` points to renewal in Mein ELSTER | networked commands, checked locally before any request |
| `gueltig_ab` in the future | `certificate_not_yet_valid` | same |
| `testzertifikat` true and the call runs live | `test_certificate_requires_testmerker`, `suggestion` adds `--test` | networked commands; never switches to test mode silently. A real Certificate with `--test` stays allowed |

ERiC has no error code for an expired Certificate, so the local check also spares the caller ELSTER's server-side rejection.

Spike checks added to [Test-system spike: ERiC end to end](11-test-system-spike.md).

No ADR: nothing here is hard to reverse.

## Comments

### Facts from the ERiC 44.3.6.0 headers (2026-11-19)

- `EricHoleZertifikatEigenschaften(hToken, pin, rueckgabeXmlPuffer)` **needs the PIN** for software certificates (`.pfx`), so inspection runs the PIN command. `EricGetPinStatus` and `EricPruefeZertifikatPin` exist as well.
- Returned XML (schema `EricHoleZertifikatEigenschaften.xsd`, namespace `http://www.elster.de/EricXML/2.0/EricHoleZertifikatEigenschaften`): separate signature and encryption certificate blocks, each with `AusgestelltAm`, `GueltigBis` (UTCTime `YYMMDDhhmmssZ`), `Signaturalgorithmus`, public-key hashes and bit length, `Issuer` and `Subjekt` as name/value lists, `Identifikationsmerkmaltyp`, `Registrierertyp` (e.g. `Person`), `Verifikationsart`, `TokenTyp`, `Testzertifikat`.
- Privacy: for an ELSTER software certificate the subject CN is the **ELSTER account ID**. Decide whether to show it, and never put a real one in fixtures.
