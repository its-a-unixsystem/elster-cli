# Maintainer live check

Run with your own live Certificate before 1.0.0 and before any release that changes the ERiC major or request XML. This uses the live service and includes an explicit Bestätigung. Use a private terminal; do not copy output or screenshots into a release PR. Keep `--eric-log` unset so diagnostic logs remain temporary.

Use the candidate binary and a profile with a live Certificate, PIN source, your HerstellerID, and a Steuernummer authorized for the Kontoabfrage. Set the shell variables below to local values: `profile`, `since` (a narrow `YYYY-MM-DD` bound), `id` (an already known Nachricht with files, preferably an Anhang), `payment_start`, `tax_kind`, and `tax_year`. Set `out` to a fresh private directory outside the repository. Run each step separately; stop and investigate unexpected output.

- [ ] `kiecli --version` and `kiecli eric status`: candidate CLI version and the expected supported ERiC version; `test_load` is true.
- [ ] `kiecli zertifikat status --profile "$profile"`: successful offline inspection; `abgelaufen` and `testzertifikat` are false, and the validity period has started.
- [ ] `kiecli postfach status --profile "$profile" --alle --von "$since"`: `test` is false and `bereitstellungsdatenarten` is an array of counts.
- [ ] `kiecli postfach anfrage --profile "$profile" --alle --von "$since"`: `test` is false; find the known Nachricht in `postfach_nachrichten`. Verify that it has `datenpaket` or `anhaenge` and that its ID is the one you intend to collect and confirm. Prefer a previously confirmed Nachricht.
- [ ] `kiecli postfach abholung --profile "$profile" --id "$id" --von "$since" --out "$out" --bestaetigen`: one matching entry with `status: "vollstaendig"` and `bestaetigt: true`. Check that each listed file exists under `out` and its SHA-256 matches. If the Nachricht had an Anhang, check that it is present and readable. On partial collection, follow the deadline and recovery steps in `kiecli skill` before continuing.
- [ ] `kiecli kontoabfrage offene-betraege --profile "$profile"`: exit 0, `test: false`, and `abfrageart: "O"`.
- [ ] `kiecli kontoabfrage zahlungen --profile "$profile" --ab "$payment_start"`: exit 0, `test: false`, and `abfrageart: "I"`.
- [ ] `kiecli kontoabfrage sollstellungen --profile "$profile" --steuerart "$tax_kind" --jahr "$tax_year"`: exit 0, `test: false`, and `abfrageart: "ZS"`.

For each Kontoabfrage, check `antwort_format` (`biene` or `alt`), the matching Steuernummer, and the response structure and amounts against the corresponding ELSTER data. An empty result is valid when ELSTER reports no entries.

Record only the CLI and ERiC versions and pass or fail per step in the release PR. Keep all identifiers, output, and collected files out of the repo.
