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
- **Reminder stays until acted on**: the notification stays until Complete / Snooze / Skip or a swipe; nothing re-fires while it's on screen (Oct 18). Swiped away, it comes back after 1 h (15 min / 30 min / 1 h / 2 h, or off; Times page). Unanswered reminders are shown again after a reboot / app update (Android) or restart (desktop).
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
| `settings` | time words (incl. custom), re-remind after a swipe, backup folder |
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

1. **Desktop builds** — Windows and Linux from the same codebase. 🟡 Built Oct 15 (scheduler + notifications, tray, single instance, `--hidden`, start at login / start hidden); packaged in CI (Linux .deb + Arch PKGBUILD, Windows NSIS) and attached to each release; Arch install/update via `scripts/install-arch.sh` (Vicinae script too). AUR `befikr-bin` once AUR registration reopens.
   - **Start hidden**: `befikr --hidden` starts straight to the tray (no window), for the user's own autostart (Linux: `~/.config/autostart` / systemd user unit / WM config).
   - **Windows**: Settings toggles "Start with Windows" and "Start hidden" (e.g. `tauri-plugin-autostart`, which passes `--hidden`).
   - **System tray** on both: open / quick add / quit; closing the window hides to tray so reminders keep running.
   - **Notifications** on both: native toasts at due time with Complete / Snooze actions where the OS allows; scheduler runs in the Rust process (no AlarmManager on desktop), re-arms from the DB on start.
   - Single instance: a second launch focuses the running one.
   - **Backup** (Oct 16): same Backup page as Android. Export a copy / restore via native file dialogs (`tauri-plugin-dialog`); optional daily backup into a picked folder, keep newest 7, checked hourly while the app runs (catches up after sleep). `src-tauri/src/desktop/backup.rs`; state in `backup.json` beside `desktop.json`.
2. **P2P sync, no server** — Syncthing-style: each device has its own database; pair devices with codes; sync whenever both are online. 🟡 Built Oct 16 (step 7) — see **P2P sync — plan** below.
   - Conflict strategy: per-field last-write-wins.
   - **Conflicts page**: every overwritten value is logged; review and restore from there. ✅ Oct 16 (side-by-side cards, tap the version to keep).
3. **Share-to-app** — share a link from browser → new task. ✅ Oct 15: own intent filter (reminders plugin manifest, merged into MainActivity) → route `/?shareText=…&shareSubject=…` → quick add with title from the page title / text, link in notes.
4. **Native snooze overlay** — true Android bottom sheet without opening the full app (custom Kotlin). 🟡 Built Oct 17, not yet on a device: notification Snooze opens `SnoozeActivity` (translucent, own task, out of recents) — task title, the six quick picks as cards, "Pick a time" (opens the app's sheet at `/?snooze=<id>` for the picker and typing), "Skip this one" for repeating tasks; closes on a choice, tap outside or Back, toast confirms. Quick picks now come from core (`snooze_picks`, 4 tests) for both the overlay (JNI `snoozePicks`) and the app's snooze sheet; JNI `snooze` writes and returns the alarm change.
5. **Completion history analysis** — suggest/reuse past tasks while adding. ✅ Oct 16: quick add shows at most one past-task chip from the 2nd typed letter (title start first, then a later word), only for titles added ≥2 times, within 30 days, or still open (then "On your list" opens that task instead). Tap fills the title, keeps typed date/repeat words, notes into empty notes. A known title pre-selects its usual time in "No time given". `Store::past_tasks` (core, tested; derived from `tasks`, nothing stored).
6. **Statistics page** — on-time vs late completion rate, how often and how many times tasks get snoozed, how long tasks stay overdue (hours/days), trends over time. Late = done on a later day than due (task's zone); same day counts as on time (changed Oct 16 from a 1-minute grace). ✅ Oct 16: Completed page became **Stats** (`/stats`, chart icon): range switch (7 days / 30 days / all time), tiles (done, % on time, median lateness, snoozes, missed repeats), last-8-weeks stacked bars (on time / late / no date), completion list below. Numbers from `Store::stats` in core.
7. **Gamification** — no points/levels or count goals (they reward volume). ✅ Oct 17: **carefree days** — a day counts when nothing due on or before it is still undone at noon the next day (rescheduled/deleted tasks don't count; snoozing doesn't help); one missed day per 7 is forgiven. Streak beside the title (taps to Stats), Stats pane with best. **Growing landscape** in `Doodle.vue` (drawings in `app/utils/garden.ts`): 12 tree kinds in growth order (pine, bush, birch, fir, oak, cypress, apple, big fir, cherry blossom, autumn maple, two pines, grand oak; 3…330 on-time completions ever), 5 flower kinds for this week's on-time ones (up to 14), birds/stars for the streak (3/7/14/30); soft natural fills, drawn stronger than the faint base landscape; nothing is taken away. Seasons (Oct 17, meteorological, flipped for southern time zones): spring fresh green, cherry blossom, apple blossom, butterflies; summer full green, fruit; autumn gold/rust/orange crowns, falling leaves, mushrooms; winter bare branches, snow caps, snowed-over bush, snowy ground, snowflakes, holly/sprigs/snowdrops. Settings → Feedback → Carefree days (device-local). `Store::carefree` (core, 5 tests). ✅ Oct 17: **habit streaks** — repeating tasks show "Done 8× · 6 weeks in a row (best 9)" in the task sheet (`Store::habit_streak`: on time = done by noon the day after; skips neutral; late, missed or overdue past the grace resets); **weekly recap** — `WeekRecap.vue` card on home Sunday 17:00 → Monday noon: done, % on time, change vs last week, carefree streak; ✕ hides it for that week. Both follow the Carefree days switch. ✅ Oct 17: **quiet milestones** (`useMilestones`, device-local, once each; already-reached ones marked seen on first run): "A birch grew in your garden" at each tree threshold, carefree streaks 7/30/100/365, "Backlog cleared" when ≥3 overdue drop to 0; waits for any toast on screen. **All-clear moment**: completing the last task plays a soft arpeggio, a double buzz, and the sun/moon rises over the empty scene. Dropped: encrypted backups (user: not needed, it's just tasks).
8. **More views** — only if needed.
9. **Shopping list** — own view behind a dedicated button on the main screen.
   - Top: the current shopping list (tap to tick off; ticked items clear away).
   - Below: chips of things usually bought ("milk", "eggs", …). Tap a chip → added to the list (tap again or already on list → no duplicate).
   - Manage chips in place: add a new chip, long-press/✕ to remove.
   - Design notes (decide when building): separate from the main task list and its reminders; own tables (`staples`, `shopping_items`) following the sync-ready rules; later, suggest chips from what gets added most often.
   - ✅ Oct 16: `/shopping` (cart button on the main screen). Migration 2 adds `shopping_items` (`bought_at` = ticked) + `staples`; core in `crates/core/src/shopping.rs`. Add field on top, tap to tick (Undo toast), chips already on the list show ✓ and don't duplicate; pencil → ✕ on chips + field for new ones. Suggestions from history + common groceries ✅ Oct 16.

## Ideas (not planned)

Suggestions to pick from later; none is decided. Same rules apply (local only, no third party).

1. ✅ **Home-screen widget** (Android) — built Oct 17: today's tasks (overdue first) with tick boxes, New task, tap a title to open it; refreshed after every change and every 30 min.
2. ~~Location reminders~~ — not doing it: no location permission or geofence checks (Oct 17).
3. ~~Quick add from anywhere~~ — not wanted (Oct 17).
4. ✅ **Voice quick add** (Android) — built Oct 17: mic button in quick add opens the phone's speech input app (`RecognizerIntent`); hidden when none is installed (GrapheneOS has none by default, e.g. FUTO Voice Input provides one, offline).
5. ✅ **Morning digest** — built Oct 17 (see session log).
6. ~~Templates~~ — not wanted (Oct 17).
7. ~~Edit by typing~~ — already there: a date typed into a task's title offers "Set …?" in the task sheet.
8. ✅ **Calendar** (wide windows) — built Oct 17: `/calendar` (button in the home header from 900 px), Week and Month views; drag a task to another day (same time; a repeat moves only this occurrence, via snooze), drag from "No date" to give a date (09:00), tap a day to add a task on it; later repeats shown dashed. ✅ `.ics` export (Oct 17): header button saves open dated tasks as a file (repeats as RRULE series, snoozes as overrides); one-way, no live feed.
9. ✅ **Search** (from Todoist) — built Oct 23: `/search` (magnifier in the home header; `/` or Ctrl+F on computers). Title, notes and subtask titles; every word must appear; open tasks first in list order, then done ones newest first; tap opens the task sheet. `Store::search` (core, 5 tests).
10. ✅ **Earlier reminders** (Todoist Pro) — built Oct 23: up to 4 per task, 5 min … 4 weeks before the due time (task sheet: "Earlier reminder" chip, then Bell row with chips). One alarm per task: core arms the first one still ahead and, when it fires, returns the next (`FireOutcome.next_at_ms`, already honoured by Android and desktop) plus `due_at_ms` for the text "Due tomorrow at 18:00". Not before a snoozed time; each repeat occurrence gets its own; synced (`tasks.remind_before`, migration 9); `.ics` export writes them as VALARMs. 9 core tests. Not yet on a device.

## Risks and open items

- ~~Exact alarms / re-remind on swipe / notification actions~~: settled by the step 0 spike (custom Kotlin plugin, `setExactAndAllowWhileIdle`, Complete runs via JNI without the app).
- **Background sync timing**: WorkManager runs ~every 15 min but Doze can stretch it; a task added elsewhere may ring late if the phone is idle. Watch in daily use.
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
| 1 | **Parser** (`packages/parser`), test table first | ✅ | 78 tests (run in any time zone); day-of-month, ordinals, monthly nth weekday, yearly dates, fortnight/quarter, repeat adverbs; repeat builder UI for the rest |
| 2 | **Rust storage layer**: rusqlite + migrations, sync-ready schema, repository commands | ✅ | `crates/core` (no Tauri dep), 56 tests; migrations 1–6 (6: `changes.prev_hlc`, `conflicts`) |
| 3 | **Ugly working MVP**: list + sort/colors, quick add + chips, complete + undo, notifications. Dogfood daily | ✅ | In daily use on Pixel 9 (GrapheneOS) from CI releases `v0.x.y`. Device checklist in step 3 notes still has unticked items |
| 4 | Re-remind escalation, snooze, subtasks, history, backup (`VACUUM INTO`, keep 7) | 🟡 | All built. Backup still needs an on-device check (folder picker, daily run, restore) |
| 5 | UX polish after UX research (research runs in parallel, never blocks 0–3) | 🟡 | Pass 1: list sections, time labels. Pass 2: quick add + task view. Pass 3 (Oct 16): task view and snooze as bottom sheets (`?task=` / `?snooze=`), home screen pinned between system bars with a "New task" bar and doodle, urgency colours (red overdue, orange within the hour), sync dot |
| 6 | **Desktop** (Windows, Linux) — Post-MVP 1 | 🟡 | Built: scheduler + notifications (+ morning digest), tray (red dot + tooltip when overdue) (dark icon), start at login / hidden, backup (export, restore, daily to a folder), packages in every release, in-app update on Windows (signed Tauri updater, download progress, off when offline; needs the signing key in repo secrets + pubkey in `tauri.conf.json`). Windows dev setup works (`pnpm tauri dev`). AUR later |
| 7 | **P2P sync** — Post-MVP 2 | 🟡 | Built: core change log + per-field LWW (+ restore-safe), iroh with relay fallback, QR/link pairing, Conflicts page, Android background sync. Phone↔phone confirmed on 0.13.1; Android background sync with the CachyOS PC confirmed Oct 17 (phone app closed); conflicts not yet device-tested |

**Next:** on Windows, check that the mouse wheel scrolls the Repeat, date and time chip rows (#14); device check of earlier reminders (Kotlin first compiles in CI: notification text "Due … at …", next one armed, swipe keeps the due alarm), of Complete from a notification refreshing the open app (#12), of (tick one task on both devices offline, then sync: 1×, card on the later device) of the stale-sync dot + hint, the themed scrollbars on Windows/Linux, the friendlier error lines (e.g. sync with the other device off), and the "What's new" sheet after updating to the next release (PC off/asleep, phone app closed), the cold-start reveal (no white flash, splash → list in one move), the morning digest, widget and voice button (Kotlin first compiles in CI). Calendar built; try it on the desktop, and import an exported `.ics` into a calendar app. Ideas list is done. (Arch in-app update dropped: Linux apps update through the package manager; AUR later.) Snooze overlay (Post-MVP 4) built, not yet on a device.

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

### P2P sync — plan

Decided Oct 16, 2026 (after looking at iroh and the user's setup: Proton VPN always on with NetShield).

**Transport: iroh** (Rust library by n0, MIT/Apache; not a service). Each device has a key pair; its public key is its address. QUIC + TLS 1.3, end-to-end encrypted, peers authenticated by key. iroh always tries the direct path first (local discovery on the same network) and only falls back to a relay when direct fails.

**Two modes, one setting** (Settings → Sync) — *superseded Oct 16, 2026: one mode (On = option A below); Same Wi-Fi, mDNS, the fixed port and typed addresses are gone.*
- **Same network only** (default): relays off, local discovery only. No third party at all.
- **Also away from home (via relay)**: same as above, plus n0's public relays as fallback. On the same Wi-Fi traffic still goes direct.
- Public relays: free, on by default in iroh, rate-limited per connection (limits not published, tuned by n0; throttling slows, never drops). n0 calls them "development and hobby use" — fine here: a sync is a few KB. A relay sees IPs, device keys, timing and volume, never content. Options if that ever matters: paid shared relays ($19/month) or self-host `iroh-relay` (no limits by default).

**Proton VPN** (user runs it always):
- Android: Proton → Settings → Connection → Advanced → **LAN connections** must be on (paid plan; on by default). "Allow direct device connections" is not needed (that's for devices that make their own Wi-Fi).
- Linux: LAN is always allowed with a paid plan (no switch in the app). If peers can't see each other, suspect the kill switch / firewall.
- NetShield is DNS blocking; no effect on LAN sync. Through the VPN a direct path between phone and desktop usually fails → relay mode carries it.

**Pairing** (works phone↔desktop and desktop↔desktop, no camera needed):
- Device A: Settings → Sync → "Add device" shows a QR code and a short code (6 digits) for a few minutes.
- Device B, same network: picks A from nearby devices and types the code (or scans the QR). Away from home: scan the QR or paste A's link (it carries A's key).
- The code is checked over the encrypted connection; few attempts, then the pairing window closes. Both save each other's key in a device-local `peers` table. Only paired keys may sync. Unpair from Settings.

**What syncs**
- Synced rows: tasks, completions, task_events, settings (shared ones), time_words, shopping_items, staples. Not synced: `alarm_state`, `peers`, device identity, backup folder, desktop startup prefs.
- Every write already goes through the Rust repository layer; it also appends to a **change log**: (table, row id, field, value, HLC timestamp, origin device). HLC = hybrid logical clock, so phone/desktop clock drift can't reorder edits.
- Each device remembers, per origin device, the newest change it has. On connect both sides send what the other lacks (so A→B→C forwards too).
- Merge: per field, newest HLC wins. Inserts are by UUIDv7 id (append-only tables never conflict). Soft deletes are a field like any other.
- A local edit overwritten by a newer remote one is written to `conflicts` (for the Conflicts page later).
- "Marked completed elsewhere" needs no special message: the completion row + task fields arrive, then the receiving device re-runs its alarm logic for touched tasks (cancel / reschedule notifications, list refresh). Same for snooze, skip, delete.

**When:** only while both are running. Desktop runs in the tray, so it's usually there. Phone: sync on app open/resume and while open; with the app closed, background sync every ~15 min (✅ Oct 16: `SyncWorker` → JNI `Native.syncOnce` → `SyncService::sync_once`, alarms re-armed from the merge). Both devices may ring for the same task until they've synced.

**Build steps**
1. Core: HLC, change log written by every repository write, export changes since X, merge incoming with LWW + conflicts log, alarm re-run for touched tasks. Tested with two in-memory stores, no network. ✅ Oct 16 (conflicts log moved to step 3): migration 3 (`sync_state`, `changes`), `crates/core/src/sync.rs` — triggers generated from one table/field list (re-created on every open; rows that predate them are backfilled once), `Store::changes_since(seq)`, `Store::merge(changes)` → alarm changes. Restoring a backup keeps this device's sync id; restored data is older, so peers' newer edits win on the next sync.
2. iroh endpoint in the app (Rust, `src-tauri`): device key, same-network discovery, sync protocol over one QUIC stream, pairing by code/QR, Settings → Sync UI (devices, last synced, mode switch). ✅ Oct 16 (built; relay mode switch included): new crate `crates/sync` (`befikr-sync`, iroh 1.3 + `iroh-mdns-address-lookup`, service name `befikr`) with `SyncService` + wire protocol (`proto.rs`: Hello/Want/Changes/Done, Pair/Paired/Denied, length-prefixed JSON on one bi stream). Migration 4: `sync_state.mode/name`, `peers`. Device key in `sync.key` next to the DB (not in backups). Sync runs on start, when a paired device appears via mDNS, ~2 s after any local write (polls the change log, so notification actions count) and every minute. Pairing: 6-digit code, 5-minute window, 5 tries; the code shows on one device, the other picks it under "Nearby" and types it (QR / link: ✅ Oct 16 — `befikr://pair?id&code&name&ip&relay`, QR under the code, Android scans in the webview (`QrScanner.vue`: getUserMedia + `jsqr`; `tauri-plugin-barcode-scanner` dropped: it needs Google Play Services ML Kit and crashed without it), desktop pastes the link under “Enter an address or link”). UI: Settings → Devices → Sync (`/sync`). Android: multicast lock while sync runs (`setMulticast`, `CHANGE_WIFI_MULTICAST_STATE`). Tested: two endpoints in-process (pair, wrong code, both directions, unpaired refused) and two processes over real mDNS on Linux (`cargo run -p befikr-sync --example peer`). Not yet: Android/Windows build of iroh in CI, on-device test.
3. Option A built (see **Sync transport — rethink**). ✅ Oct 16: Conflicts page. Rule for "concurrent": every change carries `prev_hlc`, the hlc of the version it replaced (migration 6; triggers park it in `change_prev`, since an upsert would override `INSERT OR REPLACE` inside them). An incoming change that wins over *our own* value and replaced an older version than ours (`prev_hlc < our hlc`) was made without seeing our edit → our value goes to the device-local `conflicts` table (one open per field). Only content fields count (title, notes, date, repeat, repeat mode; done/deleted only when the state differs; time words; settings). `/conflicts`: Keep / Use mine (writes ours back as a new edit, which then wins everywhere without a new conflict); reached from a row on the Sync page that only shows when there are some. Restoring a backup soft-deletes the snapshot's open conflicts. Done twice (Oct 18): an occurrence ticked on two devices before they synced counts once — on merge (and once on open for older doubles) the earliest completion per (task, `due_at_then`) stays, the others are soft-deleted as synced local writes (their missed rows drop out like an undo's), the task's done time follows; the device whose tick lost gets a "Done twice" card (`completions.completed_at`) to pick the time.

**Check per step:** iroh builds for Android (arm64) and Windows in CI before relying on it.

### Sync transport — rethink (decided Oct 16, 2026: option A)

**0.13.1 result (user, Pixel 9 ↔ Pixel 7, both GrapheneOS, same Wi-Fi, no VPN):** QR pairing in **Anywhere works** (pair + sync). **Same Wi-Fi fails** ("device not reachable", both directions, even with the IPs from the QR) → the direct path between the phones is blocked (router client isolation, or Android 17's local-network restriction; not investigated). Anywhere still goes direct when the network allows.

**Done (option A):** one mode, Off / On (`SyncMode::Relay`; a stored `"lan"` now reads as Off, so nobody is silently moved onto a relay). Pairing by QR (phones scan) or pasted link (computers); the 6-digit code only lives inside the link. Removed: mDNS (`iroh-mdns-address-lookup`, Nearby list, Search again), fixed port 47474 + TCP who-is responder, pairing by address, Android multicast lock + `CHANGE_WIFI_MULTICAST_STATE`, the `discover`/`peer` examples. Kept: peers' last direct addresses (tried before the relay). Tests run without relays (`SyncService::without_relays`), so CI needs no internet. Main list top bar: sync dot (green = working, orange = no device synced for 6 h+, red = error / not running, grey = no devices; hidden when off) → Sync page; while orange, a hint line under the bar says how long and which device to open (Oct 18). An unreachable device (app closed there) is not an error. **Next:** device test phone ↔ phone and phone ↔ Windows/Linux on 0.14; then the Conflicts page.

**Problems so far (all in connecting, none in merging):**
- 0.12.0, two Androids: nobody under "Nearby" in either mode. Nearby is mDNS only (also in Anywhere: the relay doesn't help discovery), and only lists devices that are showing a code. Suspects: Proton VPN on the phones, router client isolation, Wi-Fi drivers dropping multicast. Not confirmed (asked: VPN on? any warning on the Sync page? address under the code starting `10.2.` = VPN address).
- Desktop firewall blocks incoming → had to make the computer do all the dialling (0.12.0).
- 0.13.0: scanning the QR crashed (Pixel 9 "tokay", Android 17, likely without/sandboxed Play Services): `tauri-plugin-barcode-scanner` uses Play Services ML Kit → NPE. Fixed in 0.13.1 (in-webview scanner, getUserMedia + jsQR). Not yet tested on a device.
- Every fix is networking code that only fails on real devices, and Kotlin/Android only compiles in CI. The core merge (change log + HLC + per-field LWW, `Store::changes_since` / `Store::merge`) is transport-agnostic and tested.

**Next test (user):** 0.13.1 between two Androids: both on **Anywhere**, A "Show a pairing code", B "Scan a pairing code". Then phone ↔ desktop. Report exact error text.

**Option A — keep iroh, one path only.** Anywhere (relay) as the only mode, pair by QR/link only, drop mDNS, the fixed port and the TCP who-is responder. Relay gets through VPN/NAT/firewalls; same Wi-Fi still goes direct when possible. Cost: n0's relay sees that two devices connect (never data); we still own networking code. Good if 0.13.1 works.

**Option B — Syncthing folder as transport (recommended if 0.13.1 still fails).** Each device appends its changes to its own file in a folder the user picks (like the backup folder), e.g. `Befikr/<device-id>.jsonl`; reads the other devices' files and merges with the existing core (`changes_since` / `merge`, per-peer seq). Syncthing (user already runs it; Syncthing-Fork on Android) does discovery, NAT, VPN, relays, pairing, encryption. Befikr drops all networking: iroh, ports, QR, camera, multicast lock; smaller APK. Cost: changes arrive when Syncthing has synced (seconds); Befikr reads the folder on open/resume, on a timer while running (desktop tray), and in the background on Android (WorkManager, ~15 min) so alarms changed elsewhere get re-armed. Needs Syncthing on every device. Open details: file format + compaction (rewrite own file when large; peers track last seq per file), SAF folder access on Android (reuse backup's tree-URI picker), conflicts page unchanged.

**Option C — both** (folder main, iroh extra): most code to maintain; not recommended.

### Step 3 — app notes

Pieces: `app/` (Nuxt pages: list, stats, shopping, settings, sync, conflicts; sheets: quick add, task, snooze, date/time, repeat) → Tauri commands in `src-tauri/src/lib.rs` → `befikr-core`. Kotlin plugin promoted to `plugins/tauri-plugin-reminders`; its receivers call `src-tauri/src/jni.rs` → core (Complete from a notification uses the same `complete_task` as the app, recurring tasks reschedule their next occurrence natively). Spike deleted (git history has it).

- App id `com.owaisahmed.befikr` since 0.38 (was `com.befikr.app`; spike was `com.befikr.spike`). Desktop copies the old folders once; Android: backup → restore.
- CI `android.yml` builds a **release** APK signed with the committed sideload key (`src-tauri/android/`), so updates install in place. Download: `https://github.com/oowais/befikr/releases/latest/download/befikr.apk`. Pushes to `main` release `v0.x.y` (semver from Conventional Commits, see CLAUDE.md) so Discoverium auto-updates.
- UI checked in desktop Chromium with mocked commands (light/dark); not yet on device.
- App icon: "breeze checklist" — light = sage paper, dark = charcoal + mint (`values-night`; only some launchers switch), monochrome layer for themed icons, matching notification icon. Generated by `src-tauri/android/icon/gen.py` → `src-tauri/android/res/` (copied into the Android project in CI). App colours match (light sage / dark charcoal + mint).
- Design: **Glass** only (`app/assets/main.css`, adapted from Solid Glass, MIT: smoky glass panes, ink primary, grooves for open items, one pressed state, hue only for urgency). Classic removed after comparing. Lucide icons, 44px icon buttons.
- Permission tour (`/welcome`): shown on launch while a permission is missing (unless "Later" in the last 7 days); steps tick off as Android reports them granted. Reopen from Settings.
- Date & time picker (`DateTimeSheet.vue`): quick days, month calendar, time-word chips, hour/minute wheels; used in quick add, task view, snooze.
- Swipe gestures on list rows: right = complete (ink reveal), left = snooze sheet (amber reveal; dated tasks only). Axis-locked so vertical scroll is untouched; haptic tick at the threshold; a swipe never counts as a tap.
- (Fixed: desktop reminder scheduler, see step 6; editing a title offers its date/repeat; parsed words are highlighted inline in quick add.)

Device checklist:
- [ ] Quick add: "call mom tomorrow 6pm", "buy milk friday" (time prompt), "bins every monday evening" (repeat chip + schedule toggle)
- [ ] Reminder fires; Complete from notification with app closed; list updates on reopen
- [ ] Recurring: Complete from notification → next occurrence scheduled and fires
- [ ] Snooze from notification → snooze sheet → reminder at the new time
- [ ] Undo after completing; completed history; subtasks; settings (time word "dawn", re-remind minutes)
- [x] Update install: next CI build installs over the previous one, data kept (reminders re-armed, Oct 16)

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
- Re-remind state is device-local (`alarm_state`, not synced). Fires escalate per `reremind_minutes` (default 15/30/60). Title edits keep escalation; date/snooze changes restart it.
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

Moved to [`docs/SESSIONS.md`](docs/SESSIONS.md).
