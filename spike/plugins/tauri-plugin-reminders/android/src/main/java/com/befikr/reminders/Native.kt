package com.befikr.reminders

/**
 * JNI bridge into the app's Rust core. Runs without the webview, e.g. from a
 * notification action while the app is closed.
 *
 * Spike: library name is hardcoded to the app crate's `[lib] name`.
 */
object Native {
    init {
        System.loadLibrary("befikr_spike_lib")
    }

    /**
     * Records [kind] ("fired" | "completed" | "dismissed") for a task.
     * Returns JSON `{"show": Boolean, "nextAtMs": Long?}` or `{"error": String}`.
     */
    external fun onEvent(dbPath: String, taskId: String, kind: String): String

    /** JSON array of alarms to (re)schedule: `[{"id","title","atMs","mode"}]`. */
    external fun pendingAlarms(dbPath: String): String
}
