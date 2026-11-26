#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

LC_ALL=C objdump -T -- "$1" | awk '
    NF >= 2 && $(NF - 1) ~ /^\(?GLIBC_/ {
        version = $(NF - 1)
        gsub(/[()]/, "", version)
        found = 1
        if (version !~ /^GLIBC_[0-9]+\.[0-9]+(\.[0-9]+)?$/) {
            print "Unsupported glibc symbol version: " version > "/dev/stderr"
            bad = 1
            next
        }
        split(substr(version, 7), v, ".")
        if (v[1] > 2 || (v[1] == 2 && (v[2] > 25 || (v[2] == 25 && v[3] > 0)))) {
            print version " exceeds glibc 2.25: " $NF > "/dev/stderr"
            bad = 1
        }
    }
    END {
        if (!found) print "No glibc dynamic symbol versions found" > "/dev/stderr"
        exit (!found || bad)
    }
'
