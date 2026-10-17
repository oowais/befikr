package com.befikr.reminders

/**
 * JNI bridge into the app's Rust core (`src-tauri/src/jni.rs` → befikr-core).
 * Runs without the webview, e.g. from a notification action while the app is
 * closed. All functions return JSON; failures are `{"error": "..."}`.
 */
object Native {
    init {
        System.loadLibrary("befikr_lib")
    }

    /** Alarm went off. → `{"show": Boolean, "nextAtMs": Long?}` */
    external fun onFired(dbPath: String, taskId: String): String

    /** Notification action: "complete" | "skip" | "dismiss". → AlarmChange JSON or `{"op":"none"}` */
    external fun onAction(dbPath: String, taskId: String, action: String): String

    /** Snooze overlay quick picks for now in [tz]. → `[{"label","atMs"}]` */
    external fun snoozePicks(tz: String): String

    /** Snooze until [untilMs]. → AlarmChange JSON */
    external fun snooze(dbPath: String, taskId: String, untilMs: Long): String

    /** Alarms to (re)arm: `[{"taskId","title","atMs"}]` */
    external fun pendingAlarms(dbPath: String): String

    /** Morning digest for now in [tz]. → `{"digest": {"title","text","lines"} | null}` */
    external fun digest(dbPath: String, tz: String): String

    /** Next morning digest in [tz]. → `{"atMs": Long | null}` (null = off) */
    external fun nextDigestAt(dbPath: String, tz: String): String

    /** Consistent snapshot of the database to [outPath]. → `{}` */
    external fun exportTo(dbPath: String, outPath: String): String

    /** One background sync round. → `{"alarms": [AlarmChange], "skipped"?, "problem"?}` */
    external fun syncOnce(dbPath: String, keyPath: String): String
}
