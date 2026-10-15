# CLAUDE.md

Befikr: private, local-first Todoist replacement. Type a task in plain English, get a reliable phone reminder. No server; no third party holds data. Android first, then Windows/Linux.

## Start every session here

1. Read `PLAN.md` → **Build order and status** + **Session log**. That is the source of truth for what's done and what's next.
2. Work on the next unfinished step unless the user says otherwise.
3. Before ending: update the status table, step checklist, and add one session-log line in `PLAN.md`. Commit it with the work.

## Stack

Tauri 2 · Nuxt (`ssr: false`, SPA) · SQLite via `rusqlite` (bundled) · `chrono-node` + `rrule` for parsing (TS) · `rrule` crate for recurrence evaluation (Rust) · native Kotlin for Android alarms/notification actions where Tauri plugins fall short.

## Layout (target)

- `app/` — Nuxt SPA
- `src-tauri/` — Rust core + Android project
- `packages/parser/` — pure TS NL parser, vitest table tests
- `spike/` — step 0 throwaway; delete when step 0 is done

## Non-negotiable rules

- All DB writes go through the Rust repository layer (single place for sync change events).
- UUIDv7 primary keys; `updated_at` on every row; soft delete (`deleted_at`), never hard delete.
- Instants stored in UTC; RRULEs carry `DTSTART;TZID`.
- Recurrence/next-occurrence + scheduling logic in Rust (must run with no webview, e.g. "Complete" from a notification).
- Parser is a pure function, tested with an input → expected-output table. English only.
- UX: show controls only when context needs them; every action optimistic/instant.
- Vikunja Quick Add Magic is AGPL — read for ideas, never copy code.

## Git

- Default branch: `main`. Feature work on branches, merged via PR.
- Don't commit build outputs (`target/`, `.output/`, `node_modules/`, APKs).
