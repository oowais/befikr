package com.befikr.reminders

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.util.Log

/** Alarm went off: Rust decides whether to show it and when to re-remind. */
class AlarmReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val title = intent.getStringExtra(EXTRA_TITLE) ?: ""
        val dbPath = Prefs.dbPath(ctx) ?: return
        val outcome = parseOutcome(Native.onFired(dbPath, id)) ?: return
        if (outcome.optBoolean("show")) Notifier.show(ctx, id, title, outcome.optBoolean("repeats"))
        if (!outcome.isNull("nextAtMs")) Scheduler.schedule(ctx, id, title, outcome.getLong("nextAtMs"))
    }
}

/** Notification buttons and swipe-dismiss. Runs without opening the app. */
class ActionReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val id = intent.getStringExtra(EXTRA_ID) ?: return
        val dbPath = Prefs.dbPath(ctx) ?: return
        when (intent.action) {
            ACTION_COMPLETE, ACTION_SKIP -> {
                // Recurring tasks come back with the next occurrence to schedule.
                val op = if (intent.action == ACTION_SKIP) "skip" else "complete"
                val change = parseOutcome(Native.onAction(dbPath, id, op))
                Notifier.cancel(ctx, id)
                if (change != null) Scheduler.apply(ctx, change) else Scheduler.cancel(ctx, id)
                TasksWidget.refresh(ctx)
            }
            ACTION_DISMISS -> parseOutcome(Native.onAction(dbPath, id, "dismiss"))
            else -> Log.w(TAG, "unknown action ${intent.action}")
        }
    }
}

/** Alarms are wiped on reboot / app update; time changes need re-arming too. */
class RestoreReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        Log.i(TAG, "restore on ${intent.action}")
        Scheduler.restoreAll(ctx)
        Digest.scheduleNext(ctx)
        Backup.scheduleDaily(ctx)
    }
}
