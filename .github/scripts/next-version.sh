#!/usr/bin/env bash
# Next version from Conventional Commits since the last v0.* tag.
# Stays 0.x.y: breaking changes and feat bump minor, everything else patch.
# Prints "none" when HEAD is already tagged (nothing to release).
set -euo pipefail

last=$(git describe --tags --abbrev=0 --match 'v0.*' 2>/dev/null || true)
if [ -z "$last" ]; then
  echo "0.1.0"
  exit 0
fi
if [ -z "$(git rev-list "$last..HEAD")" ]; then
  echo "none"
  exit 0
fi

IFS=. read -r _ minor patch <<<"${last#v}"
log=$(git log --format='%s%n%b' "$last..HEAD")
if grep -qE '^[a-z]+(\([^)]*\))?!:|BREAKING CHANGE' <<<"$log" || grep -qE '^feat(\([^)]*\))?:' <<<"$log"; then
  echo "0.$((minor + 1)).0"
else
  echo "0.$minor.$((patch + 1))"
fi
