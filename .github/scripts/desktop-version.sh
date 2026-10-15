#!/usr/bin/env bash
# Version for a desktop build: the same as the APK job's in this run (the next
# 0.x.y, or the new tag if that job already released). Writes VERSION to
# $GITHUB_ENV and into tauri.conf.json.
set -euo pipefail
V=$(.github/scripts/next-version.sh)
if [ "$V" = none ]; then
  V=$(git describe --tags --abbrev=0 --match 'v0.*')
  V=${V#v}
fi
echo "VERSION=$V" >> "$GITHUB_ENV"
node -e 'const fs = require("fs"), f = "src-tauri/tauri.conf.json", c = JSON.parse(fs.readFileSync(f)); c.version = process.argv[1]; fs.writeFileSync(f, JSON.stringify(c, null, 2))' "$V"
echo "Version $V"
