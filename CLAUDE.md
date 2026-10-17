# CLAUDE.md

Befikr: private, local-first Todoist replacement. Type a task in plain English, get a reliable phone reminder. No server; no third party holds data. Android first, then Windows/Linux.

## Start every session here

1. Read `PLAN.md` → **Build order and status** (table + **Next:** line). That is the source of truth for what's done and what's next. The history is in `docs/SESSIONS.md`; open it only when past decisions matter.
2. Work on the next unfinished step unless the user says otherwise.
3. Before ending: update the status table, step checklist and **Next:** line in `PLAN.md`, and add one line at the top of `docs/SESSIONS.md` (no need to read the rest). Commit them with the work.

## Stack

Tauri 2 · Nuxt (`ssr: false`, SPA) · SQLite via `rusqlite` (bundled) · `chrono-node` + `rrule` for parsing (TS) · `rrule` crate for recurrence evaluation (Rust) · native Kotlin for Android alarms/notification actions where Tauri plugins fall short.

## Layout (target)

- `app/` — Nuxt SPA. Design system: Glass (`app/assets/main.css`, adapted from Solid Glass, MIT) — use its building blocks (`.pane`, `.popover`, `.btn`, `.chip`, `.field`, `.checkbox`, `.icon-btn`, `.ink`) and Lucide icons (`lucide-vue-next`), no emoji/unicode glyphs as icons
- `crates/core/` — `befikr-core`: storage, recurrence, reminders (pure Rust, no Tauri)
- `src-tauri/` — Tauri app: commands wrapping core, JNI entry points, Android project
- `packages/parser/` — pure TS NL parser, vitest table tests
- `plugins/tauri-plugin-reminders/` — Kotlin alarms/notifications; receivers call core via JNI (`src-tauri/src/jni.rs`)

## Non-negotiable rules

- All DB writes go through the Rust repository layer (single place for sync change events).
- UUIDv7 primary keys; `updated_at` on every row; soft delete (`deleted_at`), never hard delete.
- Instants stored in UTC; RRULEs carry `DTSTART;TZID`.
- Recurrence/next-occurrence + scheduling logic in Rust (must run with no webview, e.g. "Complete" from a notification).
- Parser is a pure function, tested with an input → expected-output table. English only.
- UX: show controls only when context needs them; every action optimistic/instant.
- Dates/times are picked only with our own `DateTimeSheet` (`mode="time"` for time only). Never the native Android pickers: no `<input type="date|time|datetime-local">`, no `DatePickerDialog`/`TimePickerDialog`.
- Vikunja Quick Add Magic is AGPL — read for ideas, never copy code.

## Environment notes

- Cloud sessions have no Android SDK (dl.google.com blocked). Android builds run in CI (`.github/workflows/android.yml`); Kotlin compile errors only show there. Pushes to `main` publish release `v0.x.y` (latest); stable link: `releases/latest/download/befikr.apk`. Discoverium tracks these releases. The APK is built only on `main` (or a manual `workflow_dispatch` run, which uploads an artifact without releasing).
- Desktop packages: `android.yml` also builds the Linux `.deb` + Arch `PKGBUILD` (from `packaging/arch/PKGBUILD.in`) and the Windows NSIS installer, and attaches them to the same release. Arch/CachyOS install/update: `scripts/install-arch.sh` (also the user's Vicinae script, copied to `~/.local/share/vicinae/scripts/befikr-install.sh`; re-copy when it changes). AUR `befikr-bin` later (AUR registration was closed on Oct 15, 2026).
- APK is signed with the committed sideload key (`src-tauri/android/`). Never change it, or updates stop installing over the old app.
- Windows in-app updates are signed with the updater key (minisign, no password). Private key: `~/.tauri/befikr-updater.key` on the user's machine (backup kept by the user) and repo secret `TAURI_SIGNING_PRIVATE_KEY` (`gh secret set TAURI_SIGNING_PRIVATE_KEY < ~/.tauri/befikr-updater.key`; no password secret). Public key: `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. A new PC needs nothing (the secret lives on GitHub); only re-set the secret if it's lost or the repo moves. Never generate a new key: installed Windows apps would reject every update. Without the secret, CI warns and releases skip `latest.json`.
- CI: PRs run `core.yml` (cargo clippy + tests, whole workspace) and `web.yml` (parser tests, Nuxt build + typecheck); `android.yml` (APK + release) runs only on `main`. Kotlin is therefore first compiled after merge — or trigger `android` by hand on the branch to check it early.
- Quick UI check without a device: `pnpm generate`, then render `.output/public` in Playwright with a mocked `window.__TAURI_INTERNALS__.invoke` (Chromium at `/opt/pw-browsers/chromium`).
- Linux desktop build works after `apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`. Local desktop binary with the bundled UI: `pnpm exec tauri build --debug --no-bundle` → `target/debug/befikr` (`--hidden` = tray only). Test with `XDG_DATA_HOME`/`XDG_CONFIG_HOME` pointed at a scratch folder to keep real data out. Desktop code lives in `src-tauri/src/desktop/` (`#[cfg(desktop)]`); Windows parts are only compiled in CI (`core.yml` → `windows`).
- Windows dev machine (user's, Oct 16, 2026): VS Build Tools (C++ tools + Win11 SDK), rustup `stable-x86_64-pc-windows-msvc`, pnpm. `pnpm tauri dev`, `pnpm test`, `cargo test --workspace` all work. Dev data lives in `%APPDATA%\com.befikr.app` (shared with an installed Befikr). Keep scripts and tests OS-neutral: no `VAR=x cmd` in `package.json`, close files before deleting them.

## Git

- Default branch: `main`. **Commit straight to local `main`** after checks pass; no branches or PRs. **Never push** — the user pushes `main` themselves (a push releases; Discoverium picks it up).
- **Conventional Commits** (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `refactor:`, `test:`, `perf:`; `!` or `BREAKING CHANGE` for breaking). They drive the version.
- **Versioning:** semver, always `0.x.y` (`.github/scripts/next-version.sh`): `feat` or breaking → minor, anything else → patch. Tags `v0.x.y` are created by the release step; don't tag by hand or edit `version` in `tauri.conf.json` (CI sets it).
- Don't commit build outputs (`target/`, `.output/`, `node_modules/`, APKs).
