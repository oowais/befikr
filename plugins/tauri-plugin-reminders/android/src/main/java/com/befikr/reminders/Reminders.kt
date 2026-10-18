package com.befikr.reminders

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import org.json.JSONArray
import org.json.JSONObject

internal const val TAG = "BefikrReminders"
/** Channel settings are immutable once created; bump the id to change them. */
internal const val CHANNEL_ID = "reminders-v2"
private val OLD_CHANNEL_IDS = listOf("reminders")
private val VIBRATION = longArrayOf(0, 400, 200, 400)
internal const val EXTRA_ID = "befikr.id"
internal const val EXTRA_TITLE = "befikr.title"
internal const val EXTRA_ROUTE = "befikr.route"
internal const val ACTION_FIRE = "com.befikr.reminders.FIRE"
internal const val ACTION_COMPLETE = "com.befikr.reminders.COMPLETE"
internal const val ACTION_DISMISS = "com.befikr.reminders.DISMISS"
internal const val ACTION_SKIP = "com.befikr.reminders.SKIP"
private const val PREFS = "befikr.reminders"
private const val PREF_DB_PATH = "dbPath"

internal object Prefs {
    fun dbPath(ctx: Context): String? =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(PREF_DB_PATH, null)

    fun setDbPath(ctx: Context, path: String) =
        ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE).edit().putString(PREF_DB_PATH, path).apply()
}

/** One alarm and one notification per task; both keyed by the task id. */
internal fun notificationId(taskId: String) = taskId.hashCode()

private fun taskUri(taskId: String, kind: String) = Uri.parse("befikr://$kind/$taskId")

private const val PI_FLAGS = PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT

internal object Scheduler {
    /** setExactAndAllowWhileIdle (chosen in step 0); inexact fallback without permission. */
    fun schedule(ctx: Context, id: String, title: String, atMs: Long) {
        val am = ctx.getSystemService(AlarmManager::class.java)
        val pi = firePendingIntent(ctx, id, title)
        val exactAllowed = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms()
        if (exactAllowed) {
            am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
        } else {
            Log.w(TAG, "exact alarms not permitted; falling back to inexact for $id")
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
        }
    }

    fun cancel(ctx: Context, id: String) {
        ctx.getSystemService(AlarmManager::class.java).cancel(firePendingIntent(ctx, id, ""))
    }

    /** Apply a core `AlarmChange` (`{"op":"schedule",...}` / `{"op":"cancel",...}`); the widget follows. */
    fun apply(ctx: Context, change: JSONObject) {
        applyAlarm(ctx, change)
        TasksWidget.refresh(ctx)
    }

    private fun applyAlarm(ctx: Context, change: JSONObject) {
        when (change.optString("op")) {
            "schedule" -> {
                val id = change.getString("taskId")
                Notifier.cancel(ctx, id)
                schedule(ctx, id, change.getString("title"), change.getLong("atMs"))
            }
            "cancel" -> {
                val id = change.getString("taskId")
                cancel(ctx, id)
                Notifier.cancel(ctx, id)
            }
        }
    }

    /**
     * Re-arm everything Rust says is pending (boot, update, time change, permission
     * granted), incl. reminders shown but unanswered. One still on screen stays as is.
     */
    fun restoreAll(ctx: Context) {
        val dbPath = Prefs.dbPath(ctx) ?: return
        val raw = Native.pendingAlarms(dbPath)
        if (raw.startsWith("{")) {
            parseOutcome(raw) // logs the error
            return
        }
        val alarms = JSONArray(raw)
        val onScreen = ctx.getSystemService(NotificationManager::class.java).activeNotifications.map { it.id }.toSet()
        for (i in 0 until alarms.length()) {
            val a = alarms.getJSONObject(i)
            val id = a.getString("taskId")
            if (notificationId(id) in onScreen) continue
            schedule(ctx, id, a.getString("title"), a.getLong("atMs"))
        }
        Log.i(TAG, "restored ${alarms.length()} alarms")
        TasksWidget.refresh(ctx)
    }

    private fun firePendingIntent(ctx: Context, id: String, title: String): PendingIntent {
        val intent = Intent(ctx, AlarmReceiver::class.java)
            .setAction(ACTION_FIRE)
            .setData(taskUri(id, "fire"))
            .putExtra(EXTRA_ID, id)
            .putExtra(EXTRA_TITLE, title)
        return PendingIntent.getBroadcast(ctx, 0, intent, PI_FLAGS)
    }
}

/** Opens the app; the webview reads [EXTRA_ROUTE] via the plugin. */
internal fun launchPendingIntent(ctx: Context, id: String, route: String): PendingIntent {
    val intent = ctx.packageManager.getLaunchIntentForPackage(ctx.packageName)!!
        .setData(taskUri(id, "open$route"))
        .addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
        .putExtra(EXTRA_ROUTE, route)
    return PendingIntent.getActivity(ctx, 0, intent, PI_FLAGS)
}

/** The snooze sheet over whatever is on screen ([SnoozeActivity]). */
internal fun snoozePendingIntent(ctx: Context, id: String, title: String, repeats: Boolean): PendingIntent =
    PendingIntent.getActivity(ctx, 0, SnoozeActivity.intent(ctx, id, title, repeats), PI_FLAGS)

internal object Notifier {
    fun ensureChannel(ctx: Context) {
        val nm = ctx.getSystemService(NotificationManager::class.java)
        OLD_CHANNEL_IDS.forEach { nm.deleteNotificationChannel(it) }
        if (nm.getNotificationChannel(CHANNEL_ID) == null) {
            nm.createNotificationChannel(
                NotificationChannel(CHANNEL_ID, "Reminders", NotificationManager.IMPORTANCE_HIGH).apply {
                    enableVibration(true)
                    vibrationPattern = VIBRATION
                }
            )
        }
    }

    /** [repeats]: also offer Skip (move to the next occurrence, no completion). */
    fun show(ctx: Context, id: String, title: String, repeats: Boolean) {
        ensureChannel(ctx)
        fun action(action: String, kind: String) = PendingIntent.getBroadcast(
            ctx, 0,
            Intent(ctx, ActionReceiver::class.java).setAction(action)
                .setData(taskUri(id, kind)).putExtra(EXTRA_ID, id),
            PI_FLAGS
        )
        val b = NotificationCompat.Builder(ctx, CHANNEL_ID)
            .setSmallIcon(smallIcon(ctx))
            .setContentTitle(title)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setVibrate(VIBRATION)
            // Unique group per task: try to stop Android auto-bundling.
            .setGroup("task-$id")
            .setContentIntent(launchPendingIntent(ctx, id, "/"))
            .addAction(0, "Complete", action(ACTION_COMPLETE, "complete"))
            .addAction(0, "Snooze", snoozePendingIntent(ctx, id, title, repeats))
            .setDeleteIntent(action(ACTION_DISMISS, "dismiss"))
        if (repeats) b.addAction(0, "Skip", action(ACTION_SKIP, "skip"))
        val n = b.build()
        try {
            NotificationManagerCompat.from(ctx).notify(notificationId(id), n)
        } catch (e: SecurityException) {
            Log.e(TAG, "notification permission missing", e)
        }
    }

    fun cancel(ctx: Context, id: String) = NotificationManagerCompat.from(ctx).cancel(notificationId(id))

    /** App-provided `ic_stat_befikr` (src-tauri/android/res), else a system bell. */
    fun smallIcon(ctx: Context): Int {
        val id = ctx.resources.getIdentifier("ic_stat_befikr", "drawable", ctx.packageName)
        return if (id != 0) id else android.R.drawable.ic_popup_reminder
    }
}

internal fun parseOutcome(raw: String): JSONObject? {
    val o = JSONObject(raw)
    if (o.has("error")) {
        Log.e(TAG, "rust error: ${o.getString("error")}")
        return null
    }
    return o
}
