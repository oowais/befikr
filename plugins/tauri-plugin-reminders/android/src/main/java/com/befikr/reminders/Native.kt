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

    /** Notification action: "complete" | "dismiss". → AlarmChange JSON or `{"op":"none"}` */
    external fun onAction(dbPath: String, taskId: String, action: String): String

    /** Alarms to (re)arm: `[{"taskId","title","atMs"}]` */
    external fun pendingAlarms(dbPath: String): String
}
