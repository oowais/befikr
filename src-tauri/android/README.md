# Sideload signing key

`sideload.jks` signs the CI release APK so updates install over each other
(Android rejects an update signed with a different key — you'd have to
uninstall and lose data).

Password: `befikr-sideload`, alias `befikr`. It is committed on purpose: the
app is sideloaded for personal use, not published. Anyone with repo access
could sign an update that installs over the app. If the repo is shared, move
the keystore to a GitHub secret and decode it in CI instead.
