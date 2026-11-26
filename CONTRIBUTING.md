# Contributing

Sign off each commit with `git commit --signoff` to certify the [Developer Certificate of Origin](https://developercertificate.org/). Use the name and email of the commit author. Contributions use the repository's licence; there is no separate contributor licence agreement.

Run `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked`. When you change the English README or getting-started guide, update the German counterpart in the same PR or note why it needs no change. The English docs are canonical.

Keep ELSTER downloads, credentials, and real Postfach and tax-account data out of commits, logs, and PRs. Use invented values in fixtures. Record JSON Schema changes in `CHANGELOG.md`.

## Release

1. [Run the maintainer live check](docs/live-check.md) before 1.0.0 and before any release that changes the ERiC major or request XML. Keep versions below 1.0 until the check passes.
2. Confirm the `elster-test` GitHub environment allows `main` and release tags, requires a maintainer reviewer, and holds `KIECLI_HERSTELLER_ID`. The weekly job uses the separate `elster-test-scheduled` environment on `main`. Keep both secrets at environment scope.
3. Restrict the `crates-io` GitHub environment to release tags. Configure [crates.io trusted publishing](https://crates.io/docs/trusted-publishing) for crate `kiecli`, repository `its-a-unixsystem/kiecli`, workflow `release.yml`, and environment `crates-io`. If the crate does not exist yet, use a short-lived crates.io API token with permission for the initial publish. Set it as `CRATES_IO_BOOTSTRAP_TOKEN` in that environment and set its `CRATES_IO_BOOTSTRAP` variable to `true`. After the first publish, revoke the token, remove both settings, and configure trusted publishing for subsequent releases.
4. Review the non-affiliation notice before the first release. Update `CHANGELOG.md` and the Cargo version in the release PR. After merging, push a new `vX.Y.Z` tag on that `main` commit matching the Cargo version. Limit tag creation and updates to maintainers with a ruleset.
5. Approve the `elster-test` deployment after CI passes. The workflow builds and checks the glibc 2.25 binary, attests the archive, publishes the GitHub Release, then publishes the source package to crates.io. Check both release pages before announcing it.
