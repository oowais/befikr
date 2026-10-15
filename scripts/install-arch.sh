#!/usr/bin/env bash
# @vicinae.schemaVersion 1
# @vicinae.title Install or Update Befikr
# @vicinae.mode fullOutput
#
# Install or update Befikr on Arch / CachyOS as a pacman package (befikr-bin),
# the way the AUR would: fetch the latest release's PKGBUILD, build it with
# makepkg, install it with pacman. Run it again to update; it does nothing
# when the latest is installed. Remove with: sudo pacman -R befikr-bin
#
# Works from a terminal (sudo) and from Vicinae, which has no terminal for a
# password prompt (pkexec: the desktop's polkit password dialog).
# Copy for Vicinae: ~/.local/share/vicinae/scripts/befikr-install.sh
set -euo pipefail

repo=oowais/befikr

latest=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
  sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p' | head -1)
if [ -z "$latest" ]; then
  echo "Couldn't find the latest Befikr release." >&2
  exit 1
fi

installed=$(pacman -Q befikr-bin 2>/dev/null | awk '{print $2}' | sed 's/-[0-9]*$//' || true)
if [ "$installed" = "$latest" ]; then
  echo "Befikr $latest is installed and up to date."
  exit 0
fi
echo "Befikr ${installed:-not installed} -> $latest"

dir=$(mktemp -d)
trap 'rm -rf "$dir"' EXIT
if ! curl -fsSL -o "$dir/PKGBUILD" "https://github.com/$repo/releases/download/v$latest/PKGBUILD"; then
  echo "Release v$latest has no desktop package (its desktop build may have failed)." >&2
  exit 1
fi
cd "$dir"

# Repackaging the release .deb needs no build dependencies; pacman pulls in
# the runtime ones when installing.
makepkg -f --nodeps
pkg=$(ls ./*.pkg.tar.zst)
if [ -t 0 ]; then
  sudo pacman -U "$pkg"
else
  pkexec pacman -U --noconfirm "$dir/${pkg#./}"
fi
echo "Befikr $latest installed."
