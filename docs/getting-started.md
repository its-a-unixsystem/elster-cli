# Getting started

🇬🇧 English | [🇩🇪 Deutsch](getting-started.de.md)

## Install KIECLI

KIECLI targets Linux x86_64. The release binary needs glibc 2.25 or newer. Install `libgcc_s.so.1` (`libgcc-s1` on Debian and Ubuntu, `libgcc` on Fedora and Arch); the archive does not include it.

Download the `kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz` archive and `SHA256SUMS` from the same [GitHub Release](https://github.com/its-a-unixsystem/kiecli/releases). In the download directory, set `archive` to the downloaded archive's name. Verify its checksum or build attestation before extracting it:

```sh
archive=kiecli-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz
read -r digest _ < SHA256SUMS
printf '%s  %s\n' "$digest" "$archive" | sha256sum --check --strict
```

Or verify the build attestation with the GitHub CLI:

```sh
gh attestation verify "$archive" --repo its-a-unixsystem/kiecli
```

After verification succeeds, install the binary somewhere on your `PATH`:

```sh
tar -xzf "$archive"
install -D -m 755 kiecli "$HOME/.local/bin/kiecli"
```

With [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) you can run `cargo binstall kiecli`. To compile against your system's glibc, use `cargo install --locked kiecli` or, from a checkout, `cargo install --locked --path .`.

## Install ERiC

Review `lizenz.pdf` inside the [official ERiC package](https://download.elster.de/download/eric/eric_44/ERiC-44.3.6.0-Linux-x86_64.jar) and the [tax administration's data-protection notice](https://www.elster.de/eportal/helpGlobal?themaGlobal=datenschutz_public#ds_steuerverw). If you accept the terms, run:

```sh
kiecli eric install --accept-eric-terms
kiecli eric status
```

`eric status` should report `test_load: true`. Both steps can be done without a Certificate or HerstellerID.

The installer verifies the package's pinned SHA-256 and test-loads ERiC and Otto before activating it. Existing installations under `versions/` are kept, and a failed install leaves the active version alone. To select an unpinned patch in a supported major, supply both `--version` and a trusted `--sha256`.

If you unpack the official package yourself, review the terms and data-protection notice, then create `.kiecli-eric-terms-accepted` next to `lizenz.pdf`. Point KIECLI at that directory with `--eric-dir` or `KIECLI_ERIC_DIR`.

## Configure a Taxpayer

Postfach and Kontoabfrage calls require your own HerstellerID. [Register for the ELSTER Entwicklerbereich](https://www.elster.de/elsterweb/registrierung-entwickler/form). Once you have a login, [apply for a HerstellerID](https://www.elster.de/elsterweb/entwickler/antrag-hersteller-id/form). Name your product and its interfaces (ERiC, Sonstige). Allow a few days for the login and up to a week or more for the HerstellerID. ELSTER may reject a product name containing “Elster”.

Set up the Taxpayer's `.pfx` Certificate and a non-interactive PIN source. For example, in `$XDG_CONFIG_HOME/kiecli/config.toml` (default `~/.config/kiecli/config.toml`):

```toml
[profile.firma]
certificate = "~/elster/firma.pfx"
pin_file = "~/elster/firma.pin"
```

Put your five-digit `hersteller_id` above the profile table, or supply it at runtime through `KIECLI_HERSTELLER_ID`. Keep the PIN file readable only by you. See the [configuration reference](../.scratch/kiecli/spec.md#4-configuration-and-secrets) for other PIN sources and setting precedence.

## Check the Certificate

```sh
kiecli zertifikat status --profile firma
```

This opens the Certificate locally without sending an ELSTER request. Check `abgelaufen` and `testzertifikat` in the result.

## Try the ELSTER test system

Use a separate profile with an official ELSTER test Certificate and its PIN source. Use your own HerstellerID. For example, to get counts without collecting or confirming anything:

```sh
kiecli postfach status --profile test --test --von "$(date -u -d yesterday +%F)"
```

Check that the response contains `"test": true`. The test Postfach is shared; keep the date range narrow and leave other users' Nachrichten unconfirmed. Remote commands use live ELSTER unless you pass `--test`. Run `kiecli skill` for the Postfach and Kontoabfrage workflows.
