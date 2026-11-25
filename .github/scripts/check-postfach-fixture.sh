#!/usr/bin/env bash
set -euo pipefail
umask 077

work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
archives=${1:-}

schema_archive() {
    local key=$1 url sha256 file
    read -r url sha256 <<< "$(python3 -c 'import sys, tomllib; c = tomllib.load(open("tests/downloads.toml", "rb"))[sys.argv[1]]; print(c["url"], c["sha256"])' "$key")"
    if [[ -n "$archives" ]]; then
        file="$archives/${url##*/}"
    else
        file="$work/${url##*/}"
        curl --proto '=https' --fail --silent --show-error --max-time 60 --output "$file" -- "$url"
    fi
    if ! printf '%s  %s\n' "$sha256" "$file" | sha256sum --check --status; then
        printf 'Checksum mismatch for %s\n' "${url##*/}" >&2
        return 1
    fi
    printf '%s\n' "$file"
}

datenabholung=$(schema_archive datenabholung_schema)
header=$(schema_archive elster_v11_schema)
unzip -qq "$datenabholung" 'Schemata/*' -d "$work"
mkdir "$work/Schemata/header"
unzip -jqq "$header" 'v11/Schemata/header/*.xsd' -d "$work/Schemata/header"
xmllint --nonet --noout --schema "$work/Schemata/elster11_datenabholung_32.xsd" tests/fixtures/elster/postfach-status.xml
