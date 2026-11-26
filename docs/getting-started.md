# Getting started

## Install KIECLI and ERiC

KIECLI targets Linux x86_64. Install `libgcc_s.so.1` (`libgcc-s1` on Debian and Ubuntu, `libgcc` on Fedora and Arch). To install KIECLI from a checkout with the Rust toolchain in `rust-toolchain.toml`, run `cargo install --path . --locked`.

Review `lizenz.pdf` inside the [official ERiC package](https://download.elster.de/download/eric/eric_44/ERiC-44.3.6.0-Linux-x86_64.jar) and the [tax administration's data-protection notice](https://www.elster.de/eportal/helpGlobal?themaGlobal=datenschutz_public#ds_steuerverw). If you accept the terms, run:

```sh
kiecli eric install --accept-eric-terms
kiecli eric status
```

`eric status` should report `test_load: true`. Both steps can be done without a Certificate or HerstellerID.

## Configure a Taxpayer

Postfach and Kontoabfrage calls require your own HerstellerID. Register for the [ELSTER Entwicklerbereich](https://www.elster.de/elsterweb/infoseite/entwickler), then apply for a HerstellerID in the portal. Name your product and the interfaces it uses (ERiC, Sonstige). Allow time for both approvals.

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

This opens the Certificate locally without sending an ELSTER request. Check `abgelaufen` and `testzertifikat` in the result. For remote commands use `--test` with a test Certificate; without a Testmerker, remote commands use live ELSTER. Run `kiecli skill` for the Postfach and Kontoabfrage workflows.
