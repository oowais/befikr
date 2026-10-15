# Step 0 spike — Android reminders

Throwaway. Proves the reminder pipeline before the real app is built. Delete once the verdict is recorded in `PLAN.md`.

## What's here

- `plugins/tauri-plugin-reminders/` — custom Tauri plugin. Kotlin owns AlarmManager + notifications; broadcast receivers call Rust via JNI (`Native.kt`), so Complete/dismiss/re-fire work with the app closed.
- `src-tauri/` — Rust core: `db.rs` (SQLite, re-remind escalation, pending alarms), `jni.rs` (entry points for Kotlin), `lib.rs` (Tauri commands).
- `app/app.vue` — test UI: permissions, schedule (custom `exact` / `alarmClock`, or official plugin for comparison), tasks, event log.

Spike re-remind intervals are 1 → 2 → 5 min (plan: 5 → 15 → 30 → 60).

## Get the APK

Easiest (on phone): GitHub repo → **Releases** → "Spike APK (latest)" → `befikr-spike.apk`. Also attached to each CI run as artifact `befikr-spike-apk` (zip; logged-in web only). Debug-signed, arm64. Install with `adb install -r <apk>` or open on the phone.

Local (needs Android SDK + NDK, `ANDROID_HOME`, `NDK_HOME`):

```sh
cd spike
pnpm install
pnpm tauri android init      # once; generates src-tauri/gen/android (gitignored)
pnpm tauri android build --debug --apk --target aarch64
# or live: pnpm tauri android dev
```

Logs: `adb logcat -s BefikrReminders`

## Test protocol

Record results in `PLAN.md` → Step 0 checklist.

1. Grant notifications, exact alarms, battery unrestricted (Status buttons). All ✅.
2. Schedule 2 min (custom, exact). Swipe app away from recents. Fires on time?
3. Doze: schedule 5 min, then `adb shell dumpsys deviceidle force-idle` (screen off). Fires on time? Repeat with `alarmClock` mode. Then two tasks due the same minute under Doze — do both fire? (`exact` is throttled to ~1 per 9 min in Doze.)
4. Tap **Complete** on the notification with the app closed. Open the app: task done, event log shows `completed · native`, no more re-fires.
5. Ignore a notification: re-fires at +1, +2, +5, +5 min (event log `fired · native`).
6. Swipe-dismiss: log shows `dismissed · native`; next re-fire still comes.
7. Tap **Snooze**: app opens with the snooze sheet for that task (cold start and while running). Pick +1 min → fires again.
8. Reboot with a pending reminder: still fires (`adb reboot`).
9. Exact alarms permission revoked (Settings): falls back to inexact; logcat warns.
10. Four+ reminders at once: shown separately or bundled?
11. Official plugin button: fires? Its actions always open the app (by design — `PendingIntent.getActivity`), and dismiss has no hook.
