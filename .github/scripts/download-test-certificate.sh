#!/usr/bin/env bash
set -euo pipefail
umask 077

output=$1
tmp=$(mktemp -d)
trap 'rm -rf -- "$tmp"' EXIT
read -r url sha256 <<< "$(python3 -c 'import tomllib; c = tomllib.load(open("tests/downloads.toml", "rb"))["test_certificate"]; print(c["url"], c["sha256"])')"
curl --proto '=https' --fail --silent --show-error --max-time 60 --output "$tmp/certs.zip" -- "$url"
printf '%s  %s\n' "$sha256" "$tmp/certs.zip" | sha256sum --check
unzip -p "$tmp/certs.zip" test-softorg-pse.pfx > "$output"
