package com.befikr.reminders

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/** Alarm went off: ask Rust whether to show, and when to re-remind. */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val title = intent.getStringExtra(EXTRA_TITLE) ?: ""
        val mode = intent.getStringExtra(EXTRA_MODE) ?: "exact"
        val dbPath = Prefs.dbPath(ctx) ?: return
        val outcome = parseOutcome(Native.onEvent(dbPath, id, "fired")) ?: return
        if (outcome.optBoolean("show")) Notifier.show(ctx, id, title)
        if (!outcome.isNull("nextAtMs")) {
            Scheduler.schedule(ctx, id, title, outcome.getLong("nextAtMs"), mode)
        }
    }
}

/** Notification buttons and swipe-dismiss. Runs without opening the app. */
class ActionReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val dbPath = Prefs.dbPath(ctx) ?: return
        when (intent.action) {
            ACTION_COMPLETE -> {
                parseOutcome(Native.onEvent(dbPath, id, "completed"))
                Scheduler.cancel(ctx, id)
                Notifier.cancel(ctx, id)
            }
            ACTION_DISMISS -> parseOutcome(Native.onEvent(dbPath, id, "dismissed"))
            else -> Log.w(TAG, "unknown action ${intent.action}")
        }
    }
}

/** Alarms are wiped on reboot / app update; time changes need re-arming too. */
class RestoreReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        Log.i(TAG, "restore on ${intent.action}")
        Scheduler.restoreAll(ctx)
    }
}
