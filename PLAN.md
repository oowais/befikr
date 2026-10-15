# Befikr — Plan

Oct 14, 2026 · @Wingiee

## Goal and principles

A private, local-first Todoist replacement: type a task in plain English, get a reliable phone reminder. No server, no third party ever holds the data.

- **Android first**, then Windows and Linux from the same codebase.
- **One main list**, minimal concepts. No projects, labels or priorities.
- **UX is the product.** If it isn't a joy to use, it won't get used.
- **Show it only when it makes sense.** Nothing visible that isn't needed right now; nothing removed either. Extra controls appear when context calls for them, or behind one subtle expand.
- **Reminders never get lost.** Unacted reminders keep coming back.
- **Design for P2P sync from day one**, build it later.

## Tech stack

| Layer | Choice | Why |
| --- | --- | --- |
| Shell | Tauri 2 | One codebase for Android, Windows, Linux |
| Frontend | Nuxt (`ssr: false`, SPA) | Known stack; file routing, auto-imports, modules |
| Database | SQLite via `rusqlite` (bundled) | Proven, no system dependency on Android. Turso considered and dropped: concurrent writes add nothing for one user, and its sync targets Turso Cloud |
| DB access | Small Rust repository layer (`add_task`, `list_tasks`, …) exposed as Tauri commands | Keeps storage swappable and sync logic in one place |
| Date parsing | `chrono-node` | Handles "tomorrow evening", "next Fri 3pm" |
| Recurrence | `rrule` (RRULE strings) + `RRule.fromText` | Standard format, English text parsing |
| Notifications | Tauri notification plugin, scheduled | Fires with app closed |
| Voice input | Phone keyboard dictation | No model needed |

## MVP features

### Main list (single view)

- App opens on the main list; a big add button opens quick add.
- Sort: overdue (oldest first) → upcoming by due date/time → no date at bottom.
- Color by urgency:

| Color | Meaning |
| --- | --- |
| Red | Overdue, or due within 1 hour |
| Yellow | Due today |
| Green | Due after today |
| Gray | No date |

- Overdue tasks stay red at the top until completed or snoozed.
- Other views may come later; none in MVP.

### Task

- **Quick add = one text field.** No buttons for date, repeat, etc. by default. Optional faint hint text (e.g. "buy milk tomorrow evening").
- Parsed parts show up only once detected: highlighted in the text, small chip below ("Tomorrow 18:00", "Every 3 days").
- **Adaptive** toggle appears only when a recurrence is detected.
- One subtle expand affordance reveals the full controls (date/time picker, recurrence, subtasks, description) for when typing isn't enough.
- List row shows: title, due date/time, recurrence icon, subtask count (e.g. "2/5") — each only if present.
- Tap → full task view with description, subtasks, all fields. Same rule: empty fields stay out of sight until you add them.
- Edit = same text field (re-parses natural language) + chips; full controls behind the same expand.

### Subtasks

- Collapsed by default with count; expand on tap.
- No own due dates or notifications — they follow the parent.
- Completing a parent auto-checks open subtasks.
- Recurring parent completes → subtasks reset to unchecked for the next occurrence.

### Dates and times

- Every dated task gets a time. Date without time ("buy milk tomorrow") → prompt with 09:00 prefilled; tap OK to accept.
- Time-word defaults, editable in settings:

| Word | Time |
| --- | --- |
| morning | 09:00 |
| noon | 12:00 |
| afternoon | 14:00 |
| evening | 18:00 |
| night | 21:00 |

- **Custom time words**: add your own in settings (e.g. "dawn" → 05:30); parser picks them up.

### Recurrence

- Natural language in quick add: "every day", "every Monday", "every start of the month".
- **Adaptive toggle** (TimeJot-style):
  - Off = fixed: next due counts from the schedule (rent, bins).
  - On = adaptive: next due counts from completion (plants, haircut).

### Notifications

- Fire at due time. One notification per task, never grouped (Android may visually stack 4+, each keeps its own actions).
- Actions: **Complete** (no app open) and **Snooze**.
- Snooze opens the app straight to a snooze bottom sheet: quick picks (tonight, tomorrow morning, …), custom date/time, or typed natural language. Sheet closes after picking. No fixed snooze duration.
- **Re-remind until acted on**: escalating 5 → 15 → 30 min, then hourly. Swipe-dismiss triggers the next re-fire. Intervals editable.
- No quiet hours.

### Completion and history

- Completing hides the task with an Undo toast.
- All completions are kept. A subtle button opens a Completed history page.
- Later: analyze history, suggest/reuse past tasks when adding new ones.

### Backup

- Manual export/import of the SQLite database.
- Automatic daily export to a user-picked folder (Android folder picker, permission persisted) — e.g. the Syncthing folder, which syncs to PC and Proton Drive.
- Export via SQLite `VACUUM INTO` for a consistent copy. Keep the last 7 snapshots, delete older ones.

## Natural language parsing

Reuse libraries, don't hardcode. English only.

1. **Recurrence**: detect "every …" phrases → `RRule.fromText` → RRULE string. Custom rules for gaps (e.g. "every start of the month").
2. **Date/time**: `chrono-node` with a custom refiner applying the time-word map, including user-defined words.
3. **Title**: strip matched spans from input; remainder is the title.
4. **Live highlighting**: highlight parsed parts in the input as you type (Todoist-style). Tap a highlight to un-parse it (e.g. "Friday" meant literally).
5. **No time given** → prompt with 09:00 prefilled.

**Prior art**: Vikunja's "Quick Add Magic" (Vue/TS) is the closest reference. AGPL — read for ideas, don't copy code.

Parser lives in TS, pure function, heavily unit-tested with a table of sample inputs → expected output.

## Data model and sync-ready rules

Draft tables:

| Table | Key fields |
| --- | --- |
| `tasks` | id, parent\_id (subtasks), title, description, due\_at, rrule, adaptive, completed\_at, snoozed\_until, sort hint |
| `completions` | id, task\_id, completed\_at, due\_at\_then (history for recurring tasks) |
| `task_events` | id, task\_id, type (created, snoozed, completed, reopened), at, due\_at\_then, snoozed\_to — log from MVP so stats work later |
| `settings` | time words (incl. custom), re-remind intervals, backup folder |
| `conflicts` | id, task\_id, field, losing value, winning value, device, timestamp (post-MVP) |

Rules from day one, so P2P sync can be added later:

- UUID primary keys, never auto-increment.
- `updated_at` per row; plan for per-field timestamps (needed for per-field last-write-wins).
- Soft deletes (`deleted_at`), never hard delete.
- Store times in UTC, render in local time zone.
- All writes go through the Rust repository layer — one place to later emit change events for sync.

## UX research (before UI build)

Study Apple Reminders and Things 3 (Apple Design Award winner) for feel, then design screens.

- [ ] Quick add: how input, parsing feedback and confirm feel; keystrokes to save
- [ ] List: density, typography, how dates/urgency are shown without clutter
- [ ] Completion: animation, haptics, undo pattern
- [ ] Subtasks: collapse/expand interaction
- [ ] Snooze / reschedule sheets: quick picks and their order
- [ ] Gestures: swipe to complete/snooze, long-press actions
- [ ] Empty states and onboarding (notification + exact-alarm permission prompts)
- [ ] Also skim TickTick and Todoist quick add for parsing UX
- [ ] Output: mockups of main list, quick add, task view, snooze sheet, settings

### UX inspiration — apps to study

Due (nagging reminders) and Things 3 (feel) are the two most relevant. The rest are for specific interactions.

| App | Platform | Why it feels good | Idea to steal |
| --- | --- | --- | --- |
| [Due](https://apps.apple.com/gb/app/due-reminders-timers/id524373870?mt=12) | iOS/Mac | Built around persistent reminders: auto-snooze re-notifies until done or rescheduled; postpone picker with 12 customizable preset times | Re-remind model (validates ours, we go further: escalating + swipe-proof); one-tap preset grid in snooze sheet; today's tasks in larger type than future ones |
| [Superlist](https://efficient.app/apps/superlist) | iOS/Android/Mac/Web | From the Wunderlist team; reviewers single out satisfying tick-off sounds, playful squiggly line details, and keeping quick capture separate from detail editing | Sound + haptic on complete; small playful illustration details; quick add stays minimal, details live in task view. **On Android — install and try it** |
| [Structured](https://screensdesign.com/showcase/structured-daily-planner) | iOS/Android | Whole day as one vertical timeline, shows free gaps between items; goal is the joy of a paper planner; onboarding has you build your first day | Possible future "timeline" view of today; interactive onboarding (add your first task as the tutorial). **On Android — try it** |
| [Flighty](https://cur.at/VcCahLU?m=web) | iOS | Aims to feel "boringly obvious"; one line per flight like airport boards; right info at the right moment (context-aware states) | Each row shows only what matters *now*: overdue → "2h late", soon → "in 25 min", later → date. Relative time near due, absolute time far out |
| [Things 3](https://culturedcode.com/things/features/) | iOS/Mac | Smooth custom animations; a to-do opens by morphing into a clean "paper" card; extra fields hidden until needed | Task-opens-in-place transition; hidden-until-needed fields; "This Evening" split of today; type-ahead date completion ("tom" → tomorrow) |
| Clear | iOS | Gesture-only to-do list: swipe right complete, swipe left delete, pull down to add | Swipe right = complete, swipe left = snooze; pull-down-to-add as alternative to the button |
| Fantastical | iOS/Mac | Natural-language entry with live preview of the parsed result | Live parse preview card under the input |
| Linear | Web/Mobile | Instant, optimistic UI; nothing waits on a spinner | Every action applies instantly, DB write in background |

**Reference — Android motion:** [Material 3 Expressive motion guide](https://skills.cat/skills/shelbeely/shelbeely-agent-skills/material-design-3-motion) — spring physics (stiffness, damping, velocity) instead of easing curves; expressive vs standard motion per scenario; pair haptics with motion; respect reduce-motion. Use as the motion spec for sheets, swipes and completion.

**Principles to adopt**

- Every action instant (optimistic), with spring motion and a light haptic on complete/snooze.
- One satisfying completion moment: check animation + haptic + task slides out; keep it short.
- Hide complexity: list shows only title, time, recurrence icon, subtask count.
- Thumb-first: add button and sheets in the bottom half; swipe gestures on rows.
- Typography carries urgency alongside color (today larger/bolder than later).
- Respect system reduce-motion setting.

Note: Clear, Fantastical and Linear entries are from general knowledge, not re-checked today.

## Post-MVP

1. **Desktop builds** — Windows and Linux from the same codebase.
   - **Start hidden**: `befikr --hidden` starts straight to the tray (no window), for the user's own autostart (Linux: `~/.config/autostart` / systemd user unit / WM config).
   - **Windows**: Settings toggles "Start with Windows" and "Start hidden" (e.g. `tauri-plugin-autostart`, which passes `--hidden`).
   - **System tray** on both: open / quick add / quit; closing the window hides to tray so reminders keep running.
   - **Notifications** on both: native toasts at due time with Complete / Snooze actions where the OS allows; scheduler runs in the Rust process (no AlarmManager on desktop), re-arms from the DB on start.
   - Single instance: a second launch focuses the running one.
2. **P2P sync, no server** — Syncthing-style: each device has its own database; pair devices with codes; sync whenever both are online.
   - Candidates: iroh (P2P transport, NAT traversal); Loro or Automerge only if full CRDT is ever needed.
   - Conflict strategy: per-field last-write-wins.
   - **Conflicts page**: every overwritten value is logged; review and restore from there.
3. **Share-to-app** — share a link from browser → new task (e.g. community `tauri-plugin-sharetarget`).
4. **Native snooze overlay** — true Android bottom sheet without opening the full app (custom Kotlin). Low priority; MVP opens the app on the snooze route.
5. **Completion history analysis** — suggest/reuse past tasks while adding.
6. **Statistics page** — on-time vs late completion rate, how often and how many times tasks get snoozed, how long tasks stay overdue (hours/days), trends over time.
7. **Gamification** (way later) — Todoist Karma-style levels, streaks, goals, built on the same stats.
8. **More views** — only if needed.
9. **Shopping list** — own view behind a dedicated button on the main screen.
   - Top: the current shopping list (tap to tick off; ticked items clear away).
   - Below: chips of things usually bought ("milk", "eggs", …). Tap a chip → added to the list (tap again or already on list → no duplicate).
   - Manage chips in place: add a new chip, long-press/✕ to remove.
   - Design notes (decide when building): separate from the main task list and its reminders; own tables (`staples`, `shopping_items`) following the sync-ready rules; later, suggest chips from what gets added most often.

## Risks and open items

- **Exact alarms**: Android 12+ needs exact-alarm permission, or Doze delays reminders. Verify the Tauri notification plugin schedules exactly; fallback = small Kotlin plugin using `AlarmManager.setExactAndAllowWhileIdle`.
- **Re-remind on swipe**: needs a delete intent on the notification to reschedule. Check plugin support; may need native code.
- **Notification actions**: confirm "Complete" runs without opening the app (background handler + DB write).
- **Battery optimization**: some Android OEMs kill background apps; may need a prompt to exempt the app.
- **Parser edge cases**: `RRule.fromText` coverage is limited; build a test table early.
- Open: exact sort tie-breaker for tasks due at the same time (creation order?).

## Architecture decisions (added Oct 15, 2026)

Resolved during planning; these override anything above that conflicts.

- **Recurrence math lives in Rust**, not TS. Completing from a notification runs with no JS/webview, so Rust must compute the next occurrence and reschedule. TS parses (`RRule.fromText` → RRULE string); Rust evaluates (`rrule` crate).
- **Time zones**: instants (`due_at`, `completed_at`, …) in UTC. RRULEs stored with `DTSTART;TZID=<local zone>` so "every day 9am" doesn't drift across DST.
- **One scheduler**: a single Rust `reschedule_all()` called after every write, on `BOOT_COMPLETED`, and on time-zone change (alarms are wiped on reboot).
- **IDs**: UUIDv7 (time-sortable). Also settles the tie-breaker: same due time → creation order.
- **Settings**: key/value `settings` table + separate `time_words` table.
- **Native Android notifications**: expect a small Kotlin plugin (`AlarmManager.setExactAndAllowWhileIdle`, BroadcastReceiver for Complete/dismiss) rather than relying only on the Tauri notification plugin. Step 0 confirms.
- **Repo layout**: `app/` (Nuxt SPA), `src-tauri/` (Rust + Android), `packages/parser/` (pure TS, vitest).

## Build order and status

Status legend: ✅ done · 🟡 in progress · ⬜ not started. **Update this section at the end of every session.**

| # | Step | Status | Notes |
| --- | --- | --- | --- |
| 0 | **Notification spike** on a real Android phone (throwaway) | ✅ | Verdict: custom Kotlin plugin + `setExactAndAllowWhileIdle`. Doze/reboot/cold-snooze tests skipped by choice; revisit if reminders arrive late |
| 1 | **Parser** (`packages/parser`), test table first | ✅ | 57 tests; day-of-month, monthly nth weekday, yearly dates, fortnight/quarter, repeat adverbs; repeat builder UI for the rest |
| 2 | **Rust storage layer**: rusqlite + migrations, sync-ready schema, repository commands | ✅ | `crates/core` (no Tauri dep), 24 tests. Exposed as Tauri commands + JNI in step 3 |
| 3 | **Ugly working MVP**: list + sort/colors, quick add + chips, complete + undo, notifications. Dogfood daily | 🟡 | Built; APK from CI releases `v0.x.y` (main only). Needs on-device check (see step 3 notes) |
| 4 | Re-remind escalation, snooze sheet, subtasks, history page, backup (`VACUUM INTO`, keep 7) | 🟡 | All built (backup in PR #1). Needs on-device check of backup (folder picker, daily run, restore) |
| 5 | UX polish after UX research (research runs in parallel, never blocks 0–3) | 🟡 | Pass 1: list sections, shorter time labels, snooze/completed labels. Pass 2: quick add + task view tidy-ups |

### Step 0 — spike checklist

Goal: prove Android reminders are reliable before building anything. Code lives in `spike/` (throwaway); test protocol in `spike/README.md`.

Built (not yet run on device): custom plugin `spike/plugins/tauri-plugin-reminders` (Kotlin AlarmManager + notifications, receivers → JNI → Rust), Rust core with escalation logic + tests, Nuxt test UI, CI workflow `spike-android` that builds a debug APK.

- [x] Code scaffolded; Rust tests + clippy pass; JNI symbols exported; Nuxt builds
- [x] CI builds the APK (debug, arm64; minSdk 26)
- [x] App runs on device (Pixel 9, Android SDK 37)
- [x] Exact notification fires on time (2 min, to the second)
- [ ] Same under forced Doze (`adb shell dumpsys deviceidle force-idle`)
- [ ] `exact` vs `alarmClock` mode under Doze, incl. two reminders due the same minute
- [x] Permissions flow: notifications, exact alarms, battery unrestricted all granted
- [x] **Complete** action writes to DB without opening the app (event source `native`)
- [x] Re-remind escalation fires while ignored (+1, +2 min observed)
- [x] Swipe-dismiss logged (`dismissed · native`) and re-fire still comes
- [x] **Snooze** opens app on the snooze sheet; snoozed reminder fired on time and escalation restarted
- [ ] Alarms survive reboot (`BOOT_COMPLETED` restore)
- [x] 8 at once: all fired on time (≤1 s). Android bundles them into one app group (count badge); each child still expandable with its own actions. Unique `setGroup` doesn't prevent it — accepted (plan already expected this)
- [x] Vibration on reminders (new channel `reminders-v2`)
- [x] Verdict recorded: **custom Kotlin plugin, `setExactAndAllowWhileIdle`**. Good enough for current use. Not tested (user's call): forced Doze, two-at-once under Doze, reboot restore, snooze from cold start — retest if reminders are ever late/missing

**Findings so far (from reading `tauri-plugin-notification` 2.7.0 source):**
- Its action buttons are `PendingIntent.getActivity` → always open the app. "Complete without opening" is impossible with it.
- Its dismiss receiver only deletes stored data; no hook to re-fire.
- It does use `setExactAndAllowWhileIdle` when exact-alarm permission is granted.
- ⇒ Custom Kotlin plugin is required (as built in the spike). Official plugin kept in the spike only for comparison.

**Design notes from the spike:**
- Re-fire is scheduled when a reminder *fires* (not on dismiss), so ignoring and swiping both lead to the next re-fire. Dismiss is only logged.
- `setExactAndAllowWhileIdle` is throttled in Doze (~1 alarm / 9 min per app). `setAlarmClock` isn't, but shows an alarm icon. Spike tests both; pick per result.
- Rust ↔ Kotlin: Kotlin `Native` object loads the app `.so` and calls `#[no_mangle]` JNI functions; panics/errors return `{"error": …}` JSON. The DB path is stored in SharedPreferences for cold starts.

### Step 3 — app notes

Pieces: `app/` (Nuxt pages: list, quick add sheet, task view, snooze, completed, settings) → Tauri commands in `src-tauri/src/lib.rs` → `befikr-core`. Kotlin plugin promoted to `plugins/tauri-plugin-reminders`; its receivers call `src-tauri/src/jni.rs` → core (Complete from a notification uses the same `complete_task` as the app, recurring tasks reschedule their next occurrence natively). Spike deleted (git history has it).

- App id `com.befikr.app` (spike was `com.befikr.spike`; uninstall it).
- CI `android.yml` builds a **release** APK signed with the committed sideload key (`src-tauri/android/`), so updates install in place. Download: `https://github.com/oowais/befikr/releases/latest/download/befikr.apk`. Pushes to `main` release `v0.x.y` (semver from Conventional Commits, see CLAUDE.md) so Discoverium auto-updates.
- UI checked in desktop Chromium with mocked commands (light/dark); not yet on device.
- App icon: "breeze checklist" — light = sage paper, dark = charcoal + mint (`values-night`; only some launchers switch), monochrome layer for themed icons, matching notification icon. Generated by `src-tauri/android/icon/gen.py` → `src-tauri/android/res/` (copied into the Android project in CI). App colours match (light sage / dark charcoal + mint).
- Design: **Glass** only (`app/assets/main.css`, adapted from Solid Glass, MIT: smoky glass panes, ink primary, grooves for open items, one pressed state, hue only for urgency). Classic removed after comparing. Lucide icons, 44px icon buttons.
- Permission tour (`/welcome`): shown on launch while a permission is missing (unless "Later" in the last 7 days); steps tick off as Android reports them granted. Reopen from Settings.
- Date & time picker (`DateTimeSheet.vue`): quick days, month calendar, time-word chips, hour/minute wheels; used in quick add, task view, snooze.
- Swipe gestures on list rows: right = complete (ink reveal), left = snooze sheet (amber reveal; dated tasks only). Axis-locked so vertical scroll is untouched; haptic tick at the threshold; a swipe never counts as a tap.
- Known MVP gaps: desktop builds have no reminder scheduler. (Fixed: editing a title offers its date/repeat; parsed words are highlighted inline in quick add.)

Device checklist:
- [ ] Quick add: "call mom tomorrow 6pm", "buy milk friday" (time prompt), "bins every monday evening" (repeat chip + schedule toggle)
- [ ] Reminder fires; Complete from notification with app closed; list updates on reopen
- [ ] Recurring: Complete from notification → next occurrence scheduled and fires
- [ ] Snooze from notification → snooze sheet → reminder at the new time
- [ ] Undo after completing; completed history; subtasks; settings (time word "dawn", re-remind minutes)
- [ ] Update install: next CI build installs over the previous one, data kept

### Step 4 — backup notes

- Snapshot: Rust `Store::export_to` (`VACUUM INTO`), reachable from the app and via JNI (`Native.exportTo`) so it runs with no webview.
- Daily: user picks a folder once (SAF `OPEN_DOCUMENT_TREE`, persisted permission). `BackupReceiver` on an inexact daily alarm (~03:00) + a catch-up on app start if the last backup is >23 h old. Files `befikr-YYYY-MM-DD-HHmm.db`, newest 7 kept. State in Kotlin SharedPreferences (device-local).
- Manual: "Export a copy…" (`CREATE_DOCUMENT`), "Restore from a backup…" (`OPEN_DOCUMENT` → cache → `Store::import_from`, SQLite backup API into the live connection, then migrations; refuses non-Befikr or newer-schema files). Current data is snapshotted to the app cache before a restore; alarms re-armed after.
- On-device checklist: [ ] pick Syncthing folder → file appears; [ ] next day a new file, max 7 kept; [ ] export a copy; [ ] restore (tasks replaced, reminders fire); [ ] folder deleted/permission lost → error shown in Settings.

### Step 2 — core notes

`crates/core` = `befikr-core`: `Store` (repository; only writer), `recurrence` (rrule crate, evaluated in the task's IANA zone), models. Every write returns `Change { task, alarm: Schedule|Cancel }` for the platform scheduler.

Decisions:
- `due_at` UTC ms + `tz` per task; RRULE body stored bare, DTSTART built from current `due_at` in `tz` (keeps wall-clock time across DST).
- Snooze sets `snoozed_until`; `due_at` stays for stats. Effective due = snooze ?? due. Editing the date clears the snooze.
- Fixed recurrence: next occurrence after max(due, now) — missed occurrences skipped. Done late → also skips the rest of today. Each skipped past occurrence is logged as a `missed` task event (`due_at_then` = that occurrence, `at` = the completion) for stats; not shown in the UI. `Store::missed_occurrences` ignores those of undone completions. Adaptive tasks log none. Adaptive: counted from completion day, keeping time of day.
- Recurring completion: `completions` row + due advances + subtasks unchecked. Undo restores previous due.
- Parent completion auto-checks open subtasks with the same timestamp; undo reopens exactly those.
- Re-remind state is device-local (`alarm_state`, not synced). Fires escalate per `reremind_minutes` (default 5/15/30/60). Title edits keep escalation; date/snooze changes restart it.
- Time words live in DB (`time_words`), seeded with defaults; parser gets them from `settings()`.
- Backup primitive: `export_to(path)` = `VACUUM INTO`. Rotation/scheduling in step 4.

### Step 1 — parser notes

API: `parse(text, now, settings?, { ignore? }) → { title, due: {date, time|null} | null, rrule, recurrenceText, spans }`. Local wall-clock out; caller converts to UTC and adds `DTSTART;TZID`. `due.time === null` → UI prompts with 09:00.

How it works: recurrence first (`every …` → grow phrase token by token through `RRule.parseText`, since it ignores trailing words; custom rules for start/end of month, weekend, "every other"; "every <time word>" = daily) → mask it → chrono (custom time-word parser + refiner overriding chrono's defaults) → strip spans + connector words (on/at/by/due/before) for the title.

Fixed Oct 15 (night): "on the 1st", "every year on march 3" / "every 3rd of march", "every last friday" / "first monday" / "second tuesday of the month" (monthly `BYDAY=±nXX`), "every 2nd tuesday" (= every other week), "every 15th", "every last day of the month", "every fortnight/quarter", "daily/weekly/biweekly/monthly/yearly"; "every 2 weeks on monday" starts next Monday. `firstDue(rrule, now, time)` exported for the repeat builder (`app/components/RepeatSheet.vue`), which covers anything typing can't.

Oct 15 (late): bare ordinals "21st", "on 21st", "21st 5pm" → next such day (this month, else next). Spelled-out ordinals ("twenty-first") deliberately not supported. A noun after an ordinal ("3rd floor") is also read as a date; ✕ on the chip keeps it as text.

Known gaps (add test rows when fixing):
- "next fri" = Friday of next week (chrono semantics); plain "fri" = coming Friday.
- Only the first date phrase is used; others stay in the title.
- Tests to add from dogfooding; Vikunja's test suite is a good source of phrasings (read only, AGPL).

## Session log

Newest first. One or two lines per session: what changed, what's next.

- **Oct 15, 2026 (week + settings)** — "This week" now ends on Sunday (Monday–Sunday weeks), not 7 rolling days. Settings: with every permission granted, Reminders collapses to one "All set up" row (tap to see them and the setup guide). **Next:** device checks; polish from dogfooding.
- **Oct 15, 2026 (sections v2)** — v0.6.0 on device showed no headings (all tasks were "Upcoming", and headings hid with one group). List now: Overdue / Today / Tomorrow / This week (to Sunday) / This month (rest of it) / Later / No date, headings always shown; Today and Tomorrow rows show only the time. **Next:** device checks; polish from dogfooding.
- **Oct 15, 2026 (no native pickers)** — Settings time words used `<input type="time">` (native Android picker); now our `DateTimeSheet` in a new time-only mode. Rule added to CLAUDE.md. **Next:** device checks; polish from dogfooding.
- **Oct 15, 2026 (time prompt)** — Quick add: a date typed without a time shows "No time given" with the time words as chips (09:00 picked; Add accepts it, "Other…" opens the picker). Disappears once a time is typed or picked. **Next:** device checks; polish from dogfooding.
- **Oct 15, 2026 (ordinals)** — Parser: bare numeric ordinals ("pay rent 21st", "21st 5pm") set the next such day; 66 tests. Workflow now: commit straight to local `main` after checks; user pushes. **Next:** device checks; polish from dogfooding.
- **Oct 15, 2026 (UX polish 2)** — Missed occurrences of late fixed recurring tasks logged as `missed` events (stats only). Quick add: dropped the `→ title` preview line (highlights already show it); with More open, date/repeat show only as fields, not also as chips. Task view: notes grow with their text (no fixed box/resize handle); "+ Notes" focuses the field. **Next:** device checks; more polish from dogfooding.
- **Oct 15, 2026 (undo)** — Undo on every complete and delete: delete gets an Undo toast (`restore_task` in core brings back the task and subtasks deleted with it); task-view Complete/Delete undo now refreshes the list. Fixed recurring task done late no longer comes back later the same day (overdue daily done at 08:00 → next is tomorrow 09:00, not today 09:00). **Next:** decide on recording missed occurrences; polish quick add + task view.
- **Oct 15, 2026 (UX polish 1)** — First local setup on CachyOS; lockfile deps stepped back past pnpm's 24 h release age. UX pass 1: main list in Overdue / Today / Upcoming / No date sections (headers only when >1), Today rows show only the time, times without leading zero, compact "2h late"; snooze picks rounded to 5 min with time-only subtitles; completed shows "1h late". Commits stay local (not pushed). **Next:** more polish (quick add, task view), device checks.
- **Oct 15, 2026 (night, 2)** — v0.4.0 merged (PR #3). Parser gaps closed (step 1 ✅), repeat builder sheet (presets, every N unit, weekdays, day-of-month / nth weekday, yearly date) in quick add + task page, inline highlight of parsed words, title edits offer their date. **Next:** user tests; UX polish (TODO #5); reliability tests (#6).
- **Oct 15, 2026 (night)** — v0.3.1 released (should have been 0.4.0: version script missed squash bodies — fixed). PR #3: release notes from commits, quick add updates while the keyboard composes (swipe/predictive typing), one date chip that opens the picker (✕ keeps words as text), date-only text saves, completed list → open/undo/delete. Setup guide never opened: plugin lacked `register_listener` permission, the throw skipped the check — fixed (also fixes snooze-from-notification while the app is open). Splash screen (`app/spa-loading-template.html`). **Next:** user tests on device; then parser gaps (TODO #4).
- **Oct 15, 2026 (swipe)** — Swipe right to complete, left to snooze (in PR #1).
- **Oct 15, 2026 (backup)** — Backup: daily to a picked folder (keep 7), export a copy, restore from file (in PR #1).
- **Oct 15, 2026 (PR flow)** — Switched to PR workflow (no direct merges). Glass-only UI, Lucide icons, bigger icon buttons, permission tour, custom date & time picker. In PR, not released.
- **Oct 15, 2026 (sound)** — Completion sounds (Web Audio, toggle in Settings); keyboard suggestions enabled in text fields; main builds queued.
- **Oct 15, 2026 (glass)** — Added Glass design (Solid Glass-inspired) alongside Classic, switchable in Settings; Geist font bundled.
- **Oct 15, 2026 (later still)** — Versioning: semver 0.x.y from Conventional Commits, releases from `main` only, version shown in Settings.
- **Oct 15, 2026 (late night)** — App icon chosen (breeze checklist, sage / charcoal+mint), wired into Android build + notifications; app theme matched.
- **Oct 15, 2026 (night, later)** — Added post-MVP item 9: shopping list view with staple chips.
- **Oct 15, 2026 (night)** — Step 3 built: Nuxt UI + Tauri commands + JNI on core; plugin promoted, spike removed; signed release APK in CI (`befikr-latest`); CI split into `core`, `web`, `android`. **Next:** user installs and dogfoods; fix what hurts; then step 4.
- **Oct 15, 2026 (evening)** — Step 0 closed (custom plugin verdict). Step 2 done: `crates/core` with schema, recurrence, repository, reminders, settings, backup export; CI `core.yml`. **Next:** step 3 — real app (`app/` + `src-tauri/`), promote reminders plugin out of `spike/`, wire core via Tauri commands + JNI, parser in quick add.
- **Oct 15, 2026 (device test 2)** — vibration, swipe-dismiss re-fire, 8 simultaneous reminders all ✅ (bundled by Android, accepted). **Left:** Doze (both modes), two-at-once under Doze, reboot, snooze from cold start.
- **Oct 15, 2026 (device test 1)** — Pixel 9 / SDK 37: exact fire, escalation, snooze, native Complete all work. Custom plugin clearly better than official. User wants vibration → added. **Next:** Doze, swipe-dismiss, reboot, alarmClock mode, 4+ notifications, vibration.
- **Oct 15, 2026 (later)** — Step 1 parser v1: `packages/parser` (chrono-node + rrule, vitest table, CI `parser.yml`). APK now also published as prerelease "Spike APK (latest)". User testing spike on phone. **Next:** record spike results; parser gaps above; then step 2 (Rust storage).
- **Oct 15, 2026** — Plan + CLAUDE.md added; `main` branch created. Step 0 spike scaffolded (custom Kotlin plugin + JNI + Rust core + test UI + APK CI). CI green after 2 fixes (runner's preinstalled SDK; minSdk 26). **Next:** user installs APK from CI artifact `befikr-spike-apk`, runs `spike/README.md` protocol on phone, record verdict; start step 1 parser in parallel.
