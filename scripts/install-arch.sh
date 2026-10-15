#!/usr/bin/env bash
# @vicinae.schemaVersion 1
# @vicinae.title Install or Update Befikr
# @vicinae.mode compact
#
# Install or update Befikr on Arch / CachyOS as a pacman package (befikr-bin),
# the way the AUR would: fetch the latest release's PKGBUILD, build it with
# makepkg, install it with pacman. Run it again to update; it does nothing
# when the latest is installed. Remove with: sudo pacman -R befikr-bin
#
# From Vicinae (no terminal) it opens one for the install, so sudo can ask
# for the password. Copy for Vicinae: ~/.local/share/vicinae/scripts/befikr-install.sh
set -euo pipefail

repo=oowais/befikr

latest=$(curl -fsSL "https://api.github.com/repos/$repo/releases/latest" |
  sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p' | head -1)
if [ -z "$latest" ]; then
  echo "Couldn't find the latest Befikr release."
  exit 1
fi

installed=$(pacman -Q befikr-bin 2>/dev/null | awk '{print $2}' | sed 's/-[0-9]*$//' || true)
if [ "$installed" = "$latest" ]; then
  echo "Befikr $latest is installed and up to date."
  exit 0
fi

# No terminal (Vicinae): reopen in one, where sudo can ask for the password.
if [ ! -t 0 ]; then
  for t in "${TERMINAL:-}" ghostty kitty foot alacritty xterm; do
    if [ -n "$t" ] && command -v "$t" >/dev/null; then
      BEFIKR_PAUSE=1 setsid "$t" -e bash "$0" >/dev/null 2>&1 &
      echo "Installing Befikr $latest in $t…"
      exit 0
    fi
  done
  echo "No terminal found: run $0 in one."
  exit 1
fi

dir=$(mktemp -d)
finish() {
  rm -rf "$dir"
  # Opened by this script: keep the window until the result is read.
  if [ -n "${BEFIKR_PAUSE:-}" ]; then read -rp "Press Enter to close." _; fi
}
trap finish EXIT

echo "Befikr ${installed:-not installed} -> $latest"
if ! curl -fsSL -o "$dir/PKGBUILD" "https://github.com/$repo/releases/download/v$latest/PKGBUILD"; then
  echo "Release v$latest has no desktop package (its desktop build may have failed)." >&2
  exit 1
fi
cd "$dir"

# Repackaging the release .deb needs no build dependencies; pacman pulls in
# the runtime ones when installing.
makepkg -f --nodeps
sudo pacman -U ./*.pkg.tar.zst
echo "Befikr $latest installed."
