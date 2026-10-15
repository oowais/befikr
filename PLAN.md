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
| 0 | **Notification spike** on a real Android phone (throwaway) | 🟡 | See checklist below |
| 1 | **Parser** (`packages/parser`), test table first | ⬜ | Can run in parallel with 0 |
| 2 | **Rust storage layer**: rusqlite + migrations, sync-ready schema, repository commands | ⬜ | |
| 3 | **Ugly working MVP**: list + sort/colors, quick add + chips, complete + undo, notifications. Dogfood daily | ⬜ | |
| 4 | Re-remind escalation, snooze sheet, subtasks, history page, backup (`VACUUM INTO`, keep 7) | ⬜ | |
| 5 | UX polish after UX research (research runs in parallel, never blocks 0–3) | ⬜ | |

### Step 0 — spike checklist

Goal: prove Android reminders are reliable before building anything. Code lives in `spike/` and is thrown away afterward.

- [ ] Tauri 2 + Nuxt SPA builds and runs on device
- [ ] Exact notification scheduled 2 min out fires on time with app closed (also under forced Doze: `adb shell dumpsys deviceidle force-idle`)
- [ ] Exact-alarm permission flow (Android 12+) works
- [ ] **Complete** action writes to DB without opening the app
- [ ] Swipe-dismiss triggers a delete intent → next re-fire scheduled
- [ ] **Snooze** action opens app on the snooze route
- [ ] Alarms survive reboot (`BOOT_COMPLETED` reschedule)
- [ ] Verdict recorded: Tauri notification plugin enough, or custom Kotlin plugin needed

## Session log

Newest first. One or two lines per session: what changed, what's next.

- **Oct 15, 2026** — Plan + CLAUDE.md committed; `main` created as default branch. Started step 0 spike.
