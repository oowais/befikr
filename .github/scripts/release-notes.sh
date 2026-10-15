#!/usr/bin/env bash
# Markdown release notes from Conventional Commits since the last v0.* tag.
# Reads squash-merge bodies too ("* feat: ..."), so each branch commit counts.
# Skips merge commits, docs/chore/ci/test noise and co-author trailers.
set -euo pipefail

last=$(git describe --tags --abbrev=0 --match 'v0.*' 2>/dev/null || true)
range=${last:+$last..}HEAD

lines=$(git log --no-merges --format='%s%n%b' "$range" \
  | sed -E 's/^\* //' \
  | grep -E '^[a-z]+(\([^)]*\))?!?: ' || true)

section() { # title, type regex
  local items
  items=$(grep -E "^($2)(\([^)]*\))?!?: " <<<"$lines" | sed -E 's/^[a-z]+(\([^)]*\))?!?: /- /' | sed -E 's/^- (.)/- \U\1/' | awk '!seen[$0]++' || true)
  [ -n "$items" ] && printf '### %s\n%s\n\n' "$1" "$items"
  return 0
}

{
  section "New" "feat"
  section "Fixes" "fix"
  section "Improvements" "perf|refactor"
  echo "Arm64, Android 8+. Installs over the previous version."
  [ -n "$last" ] && echo "Changes since ${last}."
}
