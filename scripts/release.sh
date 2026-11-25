#!/usr/bin/env bash
# Release Befikr: picks the next version from the Conventional Commits since
# the last tag (.github/scripts/next-version.sh), shows the release notes
# (the feat:/fix: lines are also the in-app "What's new"), then tags HEAD and
# pushes the tag, which starts the release build (.github/workflows/android.yml).
# Push main first, so the tag points at a commit GitHub already has.
# Usage: scripts/release.sh [--dry-run]
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"
dry=false
[ "${1:-}" = --dry-run ] && dry=true

if [ "$(git branch --show-current)" != main ]; then
  echo "Switch to main first."
  exit 1
fi
if [ -n "$(git status --porcelain)" ]; then
  echo "Commit or stash your changes first."
  exit 1
fi
git fetch -q --tags origin
if [ "$(git rev-parse HEAD)" != "$(git rev-parse origin/main)" ]; then
  echo "Local main and origin/main differ: push (or pull) first."
  exit 1
fi

v=$(.github/scripts/next-version.sh)
if [ "$v" = none ]; then
  echo "Nothing to release: no user-facing commit since $(git describe --tags --abbrev=0 --match 'v0.*')."
  exit 0
fi
echo "Befikr $v"
echo
.github/scripts/release-notes.sh
echo
$dry && exit 0

read -r -p "Tag v$v and push it (starts the release build)? [y/N] " ok
case "$ok" in
  y | Y) ;;
  *) echo "Nothing done."; exit 0 ;;
esac
git tag -a "v$v" -m "Befikr $v"
git push origin "v$v"
echo "Release build: https://github.com/oowais/befikr/actions/workflows/android.yml"
