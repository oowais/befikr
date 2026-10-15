# CLAUDE.md

Befikr: private, local-first Todoist replacement. Type a task in plain English, get a reliable phone reminder. No server; no third party holds data. Android first, then Windows/Linux.

## Start every session here

1. Read `PLAN.md` → **Build order and status** + **Session log**. That is the source of truth for what's done and what's next.
2. Work on the next unfinished step unless the user says otherwise.
3. Before ending: update the status table, step checklist, and add one session-log line in `PLAN.md`. Commit it with the work.

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
- Vikunja Quick Add Magic is AGPL — read for ideas, never copy code.

## Environment notes

- Cloud sessions have no Android SDK (dl.google.com blocked). Android builds run in CI (`.github/workflows/android.yml`); Kotlin compile errors only show there. Pushes to `main` publish release `v0.x.y` (latest); stable link: `releases/latest/download/befikr.apk`. Discoverium tracks these releases. The APK is built only on `main` (or a manual `workflow_dispatch` run, which uploads an artifact without releasing).
- APK is signed with the committed sideload key (`src-tauri/android/`). Never change it, or updates stop installing over the old app.
- CI: PRs run `core.yml` (cargo clippy + tests, whole workspace) and `web.yml` (parser tests, Nuxt build + typecheck); `android.yml` (APK + release) runs only on `main`. Kotlin is therefore first compiled after merge — or trigger `android` by hand on the branch to check it early.
- Quick UI check without a device: `pnpm generate`, then render `.output/public` in Playwright with a mocked `window.__TAURI_INTERNALS__.invoke` (Chromium at `/opt/pw-browsers/chromium`).
- Linux desktop build works after `apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev`.

## Git

- Default branch: `main`. **Work goes into one open PR** from the working branch to `main`; keep adding commits to it. **Never merge or push to `main` unless the user says so** — merging releases (Discoverium picks it up).
- **Conventional Commits** (`feat:`, `fix:`, `chore:`, `docs:`, `ci:`, `refactor:`, `test:`, `perf:`; `!` or `BREAKING CHANGE` for breaking). They drive the version.
- **Versioning:** semver, always `0.x.y` (`.github/scripts/next-version.sh`): `feat` or breaking → minor, anything else → patch. Tags `v0.x.y` are created by the release step; don't tag by hand or edit `version` in `tauri.conf.json` (CI sets it).
- Don't commit build outputs (`target/`, `.output/`, `node_modules/`, APKs).
