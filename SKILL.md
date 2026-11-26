---
name: kiecli
description: Use KIECLI to check an ELSTER Certificate, read and collect Postfach-Nachrichten, or run a Kontoabfrage for one Taxpayer.
---

# KIECLI

> KIECLI is an independent open-source project. It is not affiliated with, endorsed by or supported by ELSTER, the Bayerisches Landesamt für Steuern or any tax authority. ELSTER® is a registered trademark of the Freistaat Bayern.

KIECLI reads the Postfach and tax account for one Taxpayer at a time. It can confirm receipt explicitly; it cannot file returns, reply to Nachrichten, or maintain an archive. Keep collected files in a private directory the user controls.

## Set up and check credentials

1. Ask the user to review the ERiC terms and data-protection notice in [Getting started](https://github.com/its-a-unixsystem/kiecli/blob/main/docs/getting-started.md). With their approval, run `kiecli eric install --accept-eric-terms`, then `kiecli eric status`. Continue when `test_load` is true.
2. Hand these steps back to the user: obtain their own HerstellerID and configure the Certificate and PIN source. Point them to [Getting started](https://github.com/its-a-unixsystem/kiecli/blob/main/docs/getting-started.md#configure-a-taxpayer) and the [configuration reference](https://github.com/its-a-unixsystem/kiecli/blob/main/.scratch/kiecli/spec.md#4-configuration-and-secrets). Use a profile and a private PIN file or non-interactive PIN command; keep the PIN out of chat, arguments, and logs.
3. Start each session with `kiecli zertifikat status --profile NAME` for the intended Taxpayer. Check `abgelaufen` and `testzertifikat` before contacting ELSTER. Use `--test` with a test Certificate. Without a Testmerker, commands use live ELSTER.

## Read the Postfach

1. Run `kiecli postfach anfrage --profile NAME --von YYYY-MM-DD` to get IDs without collecting anything. Use `--alle` to include confirmed Nachrichten. `postfach status` is only a hint: it can omit kinds that `anfrage` returns.
2. If authorized to collect and confirm, run `kiecli postfach abholung --profile NAME --id ID --out DIR --bestaetigen`. Confirmations apply only to Nachrichten whose files were all safely written. Check each entry's `status`, `files`, and `bestaetigt`.
3. If the user wants to store or inspect files before confirming, choose `--ohne-bestaetigung` instead. Store the files, then run `kiecli postfach bestaetigung --profile NAME --id ID` before `bestaetigung_faellig_bis`. **Confirm within 24 hours of Abholung, even if only some files were collected. Missing the deadline risks ELSTER blocking the HerstellerID.** Check the per-ID result; reporting a deadline is not confirmation.

## Run a Kontoabfrage

Use `kiecli kontoabfrage offene-betraege` for open amounts, `zahlungen --ab YYYY-MM-DD` for payments, or `sollstellungen --steuerart TYPE --jahr YYYY` for amounts assessed. Supply the Steuernummer through the profile or `--steuernummer`; add `--bundesland` for Bescheid format.

ELSTER code `230077012` means the Certificate is not authorized for the Steuerkontoabfrage. The public test Certificate cannot get a tax-account result. For a live account, ask the user to activate the Steuerkontoabfrage in Mein ELSTER. For a Vollmachtgeber, the Steuerkontoabfrage indication must be active for that Steuernummer. First activation varies by Bundesland; ask the Finanzamt about the local procedure.

## Handle results

- Exit 6: retry the transient failure, observing any open Bestätigung deadline.
- Exit 5: read the error and correct the request or prerequisites before retrying.
- Exit 7: read stdout as well as stderr. Confirm only fully written Nachrichten that are still unconfirmed. For `teilweise`, retry Abholung for that ID and then confirm. If retries keep failing, confirm before the deadline anyway and tell the user which files are missing.

Read JSON from stdout and the error object from stderr. Tolerate unknown warning codes, error codes, and enum values. Run `kiecli describe` for arguments, settings, diagnostics and schemas, or `kiecli describe postfach abholung` for one command.
