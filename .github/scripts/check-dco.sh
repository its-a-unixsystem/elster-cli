#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-only
set -euo pipefail

head=$1
base=$2
range=("$head")
if [[ -n "$base" ]]; then
    range+=("^$base")
fi
commits=$(git rev-list "${range[@]}" --)
failed=0
while IFS= read -r commit; do
    [[ -n "$commit" ]] || continue
    author=$(git show -s --format='%an <%ae>' "$commit")
    signoffs=$(git show -s --format='%(trailers:key=Signed-off-by,valueonly,unfold)' "$commit")
    if ! grep -Fxq -- "$author" <<< "$signoffs"; then
        printf 'Commit %s lacks a Signed-off-by trailer matching its author. Use git commit --signoff.\n' "$commit" >&2
        failed=1
    fi
done <<< "$commits"
exit "$failed"
