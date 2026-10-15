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
internal const val EXTRA_MODE = "befikr.mode"
internal const val EXTRA_ROUTE = "befikr.route"
internal const val ACTION_FIRE = "com.befikr.reminders.FIRE"
internal const val ACTION_COMPLETE = "com.befikr.reminders.COMPLETE"
internal const val ACTION_DISMISS = "com.befikr.reminders.DISMISS"
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
    /**
     * mode "exact": setExactAndAllowWhileIdle (Doze-throttled to ~1 per 9 min per app).
     * mode "alarmClock": setAlarmClock (never throttled, shows alarm icon in status bar).
     */
    fun schedule(ctx: Context, id: String, title: String, atMs: Long, mode: String) {
        val am = ctx.getSystemService(AlarmManager::class.java)
        val pi = firePendingIntent(ctx, id, title, mode)
        val exactAllowed = Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms()
        when {
            !exactAllowed -> {
                Log.w(TAG, "exact alarms not permitted; falling back to inexact for $id")
                am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
            }
            mode == "alarmClock" -> am.setAlarmClock(AlarmManager.AlarmClockInfo(atMs, launchPendingIntent(ctx, id, "/")), pi)
            else -> am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, atMs, pi)
        }
        Log.i(TAG, "scheduled $id at $atMs mode=$mode exact=$exactAllowed")
    }

    fun cancel(ctx: Context, id: String) {
        ctx.getSystemService(AlarmManager::class.java).cancel(firePendingIntent(ctx, id, "", ""))
    }

    /** Re-arm everything Rust says is pending (boot, time change, permission granted). */
    fun restoreAll(ctx: Context) {
        val dbPath = Prefs.dbPath(ctx) ?: return
        val raw = Native.pendingAlarms(dbPath)
        if (raw.startsWith("{")) {
            Log.e(TAG, "pendingAlarms failed: $raw")
            return
        }
        val alarms = JSONArray(raw)
        for (i in 0 until alarms.length()) {
            val a = alarms.getJSONObject(i)
            schedule(ctx, a.getString("id"), a.getString("title"), a.getLong("atMs"), a.getString("mode"))
        }
        Log.i(TAG, "restored ${alarms.length()} alarms")
    }

    private fun firePendingIntent(ctx: Context, id: String, title: String, mode: String): PendingIntent {
        val intent = Intent(ctx, AlarmReceiver::class.java)
            .setAction(ACTION_FIRE)
            .setData(taskUri(id, "fire"))
            .putExtra(EXTRA_ID, id)
            .putExtra(EXTRA_TITLE, title)
            .putExtra(EXTRA_MODE, mode)
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

    fun show(ctx: Context, id: String, title: String) {
        ensureChannel(ctx)
        val complete = PendingIntent.getBroadcast(
            ctx, 0,
            Intent(ctx, ActionReceiver::class.java).setAction(ACTION_COMPLETE)
                .setData(taskUri(id, "complete")).putExtra(EXTRA_ID, id),
            PI_FLAGS
        )
        val dismiss = PendingIntent.getBroadcast(
            ctx, 0,
            Intent(ctx, ActionReceiver::class.java).setAction(ACTION_DISMISS)
                .setData(taskUri(id, "dismiss")).putExtra(EXTRA_ID, id),
            PI_FLAGS
        )
        val n = NotificationCompat.Builder(ctx, CHANNEL_ID)
            .setSmallIcon(android.R.drawable.ic_popup_reminder)
            .setContentTitle(title)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setPriority(NotificationCompat.PRIORITY_HIGH)
            .setVibrate(VIBRATION)
            // Unique group per task: try to stop Android auto-bundling.
            .setGroup("task-$id")
            .setContentIntent(launchPendingIntent(ctx, id, "/"))
            .addAction(0, "Complete", complete)
            .addAction(0, "Snooze", launchPendingIntent(ctx, id, "/snooze?id=$id"))
            .setDeleteIntent(dismiss)
            .build()
        try {
            NotificationManagerCompat.from(ctx).notify(notificationId(id), n)
        } catch (e: SecurityException) {
            Log.e(TAG, "notification permission missing", e)
        }
    }

    fun cancel(ctx: Context, id: String) = NotificationManagerCompat.from(ctx).cancel(notificationId(id))
}

internal fun parseOutcome(raw: String): JSONObject? {
    val o = JSONObject(raw)
    if (o.has("error")) {
        Log.e(TAG, "rust error: ${o.getString("error")}")
        return null
    }
    return o
}
