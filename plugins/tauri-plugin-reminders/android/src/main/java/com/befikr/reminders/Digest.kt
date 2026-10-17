package com.befikr.reminders

import android.app.AlarmManager
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.os.Build
import android.util.Log
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import org.json.JSONObject
import java.util.TimeZone

private const val DIGEST_CHANNEL_ID = "digest"
/** Matches `DIGEST_ID` in befikr-core; never a task id (those are UUIDv7). */
private const val DIGEST_ID = "digest"

/**
 * Morning digest: one quiet notification a day listing what's overdue and
 * due today. Rust decides the time and the text; this only arms the alarm
 * and shows the result.
 */
internal object Digest {
    /** Arm the next digest, or cancel it when it's off. */
    fun scheduleNext(ctx: Context) {
        val dbPath = Prefs.dbPath(ctx) ?: return
        val next = parseOutcome(Native.nextDigestAt(dbPath, TimeZone.getDefault().id)) ?: return
        val am = ctx.getSystemService(AlarmManager::class.java)
        val pi = pendingIntent(ctx)
        if (next.isNull("atMs")) {
            am.cancel(pi)
            return
        }
        val at = next.getLong("atMs")
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms()) {
            am.setExactAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pi)
        } else {
            am.setAndAllowWhileIdle(AlarmManager.RTC_WAKEUP, at, pi)
        }
    }

    fun show(ctx: Context, digest: JSONObject) {
        ensureChannel(ctx)
        val lines = digest.getJSONArray("lines")
        val style = NotificationCompat.InboxStyle().setBigContentTitle(digest.getString("title"))
        for (i in 0 until lines.length()) style.addLine(lines.getString(i))
        val n = NotificationCompat.Builder(ctx, DIGEST_CHANNEL_ID)
            .setSmallIcon(Notifier.smallIcon(ctx))
            .setContentTitle(digest.getString("title"))
            .setContentText(digest.getString("text"))
            .setStyle(style)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setCategory(NotificationCompat.CATEGORY_REMINDER)
            .setGroup("digest")
            .setAutoCancel(true)
            .setContentIntent(launchPendingIntent(ctx, DIGEST_ID, "/"))
            .build()
        try {
            NotificationManagerCompat.from(ctx).notify(notificationId(DIGEST_ID), n)
        } catch (e: SecurityException) {
            Log.e(TAG, "notification permission missing", e)
        }
    }

    /** Low importance: in the shade and on the lock screen, no sound or pop-up. */
    private fun ensureChannel(ctx: Context) {
        val nm = ctx.getSystemService(NotificationManager::class.java)
        if (nm.getNotificationChannel(DIGEST_CHANNEL_ID) == null) {
            nm.createNotificationChannel(
                NotificationChannel(DIGEST_CHANNEL_ID, "Morning digest", NotificationManager.IMPORTANCE_LOW)
            )
        }
    }

    private fun pendingIntent(ctx: Context): PendingIntent =
        PendingIntent.getBroadcast(
            ctx, 0, Intent(ctx, DigestReceiver::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
}

/** Digest time: show today's summary (if anything is due) and arm tomorrow's. */
class DigestReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val dbPath = Prefs.dbPath(ctx) ?: return
        val out = parseOutcome(Native.digest(dbPath, TimeZone.getDefault().id))
        out?.optJSONObject("digest")?.let { Digest.show(ctx, it) }
        Digest.scheduleNext(ctx)
        TasksWidget.refresh(ctx)
    }
}
