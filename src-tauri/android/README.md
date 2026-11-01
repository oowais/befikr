# Sideload signing key

The release APK is signed with one key, every build, so updates install over
each other (Android rejects an update signed with a different key — you'd
have to uninstall and lose data).

The repo is public, so the key is **not** committed. It lives on the user's
machine next to the updater key, and in two repo secrets that CI reads
(`.github/workflows/android.yml`):

| File | Secret |
| --- | --- |
| `~/.tauri/befikr-sideload.p12` (PKCS12, alias `befikr`) | `ANDROID_KEYSTORE_B64` (base64 of the file) |
| `~/.tauri/befikr-sideload.pass` | `ANDROID_KEYSTORE_PASSWORD` |

```bash
base64 -w0 ~/.tauri/befikr-sideload.p12 | gh secret set ANDROID_KEYSTORE_B64 -R oowais/befikr
gh secret set ANDROID_KEYSTORE_PASSWORD -R oowais/befikr < ~/.tauri/befikr-sideload.pass
```

Never generate a new key once an APK is installed: updates would stop
installing. Keep a backup of both files.

Until Nov 1, 2026 a keystore was committed here (`sideload.jks`, password
in the workflow). It is still in git history, so it was replaced: APKs from
0.x releases after that date don't install over older ones (uninstall once).
