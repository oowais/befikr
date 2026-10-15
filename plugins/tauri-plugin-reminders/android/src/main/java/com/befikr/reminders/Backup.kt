package com.befikr.reminders

import android.app.AlarmManager
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.util.Log
import androidx.documentfile.provider.DocumentFile
import org.json.JSONObject
import java.io.File
import java.text.SimpleDateFormat
import java.util.Calendar
import java.util.Date
import java.util.Locale

/**
 * Database backups. Rust makes a consistent snapshot (VACUUM INTO, via JNI);
 * this writes it through the Storage Access Framework:
 *  - daily into a folder the user picked once (e.g. a Syncthing folder),
 *    keeping the newest [KEEP];
 *  - one-off export to a chosen file, and import from one.
 * State (folder, last run) is device-local SharedPreferences.
 */
object Backup {
    private const val KEEP = 7
    private const val DAY_MS = 86_400_000L
    private const val PREFS = "befikr.backup"
    private const val PREF_TREE = "tree"
    private const val PREF_LAST = "lastAt"
    private const val PREF_ERROR = "lastError"
    private val NAME = Regex("""befikr-\d{4}-\d{2}-\d{2}-\d{4}\.db""")

    private fun prefs(ctx: Context) = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE)

    fun tree(ctx: Context): Uri? = prefs(ctx).getString(PREF_TREE, null)?.let(Uri::parse)

    fun setTree(ctx: Context, uri: Uri?) {
        prefs(ctx).edit().putString(PREF_TREE, uri?.toString()).remove(PREF_ERROR).apply()
        if (uri != null) scheduleDaily(ctx) else cancelDaily(ctx)
    }

    fun fileName(at: Long = System.currentTimeMillis()): String =
        "befikr-" + SimpleDateFormat("yyyy-MM-dd-HHmm", Locale.US).format(Date(at)) + ".db"

    fun status(ctx: Context): JSONObject {
        val p = prefs(ctx)
        val folder = tree(ctx)?.let { runCatching { DocumentFile.fromTreeUri(ctx, it)?.name }.getOrNull() ?: "Chosen folder" }
        return JSONObject().apply {
            put("folder", folder ?: JSONObject.NULL)
            put("lastAt", if (p.contains(PREF_LAST)) p.getLong(PREF_LAST, 0) else JSONObject.NULL)
            put("lastError", p.getString(PREF_ERROR, null) ?: JSONObject.NULL)
        }
    }

    /** Snapshot of the live database into the app cache. */
    private fun snapshot(ctx: Context): File {
        val db = Prefs.dbPath(ctx) ?: error("Database not ready yet; open the app once")
        val out = File(ctx.cacheDir, "befikr-snapshot.db")
        val res = JSONObject(Native.exportTo(db, out.absolutePath))
        if (res.has("error")) error(res.getString("error"))
        return out
    }

    private fun copy(ctx: Context, from: File, to: Uri) {
        val out = ctx.contentResolver.openOutputStream(to, "wt") ?: error("Cannot write to that location")
        out.use { o -> from.inputStream().use { it.copyTo(o) } }
    }

    /** Daily backup now: snapshot into the folder, prune to [KEEP]. Returns the file name. */
    @Synchronized
    fun runNow(ctx: Context): String {
        try {
            val tree = tree(ctx) ?: error("No backup folder chosen")
            val dir = DocumentFile.fromTreeUri(ctx, tree)?.takeIf { it.canWrite() }
                ?: error("Backup folder is no longer accessible; choose it again")
            val snap = snapshot(ctx)
            val name = fileName()
            dir.findFile(name)?.delete()
            val doc = dir.createFile("application/octet-stream", name) ?: error("Could not create $name")
            copy(ctx, snap, doc.uri)
            snap.delete()
            dir.listFiles()
                .filter { it.name?.matches(NAME) == true }
                .sortedByDescending { it.name }
                .drop(KEEP)
                .forEach { it.delete() }
            prefs(ctx).edit().putLong(PREF_LAST, System.currentTimeMillis()).remove(PREF_ERROR).apply()
            Log.i(TAG, "backup written: $name")
            return name
        } catch (e: Exception) {
            Log.e(TAG, "backup failed", e)
            prefs(ctx).edit().putString(PREF_ERROR, e.message ?: e.toString()).apply()
            throw e
        }
    }

    /** Back up if a folder is set and the last backup is about a day old. */
    fun runIfDue(ctx: Context) {
        if (tree(ctx) == null) return
        val last = prefs(ctx).getLong(PREF_LAST, 0)
        if (System.currentTimeMillis() - last < DAY_MS - 3_600_000L) return
        runCatching { runNow(ctx) }
    }

    /** One-off export to a file the user created. */
    fun exportTo(ctx: Context, uri: Uri) {
        val snap = snapshot(ctx)
        copy(ctx, snap, uri)
        snap.delete()
    }

    /** Copy a picked file into the cache so Rust can read it by path. */
    fun importToCache(ctx: Context, uri: Uri): File {
        val out = File(ctx.cacheDir, "befikr-import.db")
        val input = ctx.contentResolver.openInputStream(uri) ?: error("Cannot read that file")
        input.use { i -> out.outputStream().use { i.copyTo(it) } }
        return out
    }

    // ---- Daily alarm (inexact: battery-friendly, ~03:00) ----

    private fun dailyIntent(ctx: Context) = PendingIntent.getBroadcast(
        ctx, 0, Intent(ctx, BackupReceiver::class.java).setAction("com.befikr.reminders.BACKUP"),
        PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
    )

    fun scheduleDaily(ctx: Context) {
        if (tree(ctx) == null) return
        val next = Calendar.getInstance().apply {
            set(Calendar.HOUR_OF_DAY, 3); set(Calendar.MINUTE, 0); set(Calendar.SECOND, 0)
            if (timeInMillis <= System.currentTimeMillis()) add(Calendar.DAY_OF_MONTH, 1)
        }
        ctx.getSystemService(AlarmManager::class.java)
            .setInexactRepeating(AlarmManager.RTC_WAKEUP, next.timeInMillis, AlarmManager.INTERVAL_DAY, dailyIntent(ctx))
    }

    private fun cancelDaily(ctx: Context) {
        ctx.getSystemService(AlarmManager::class.java).cancel(dailyIntent(ctx))
    }
}

/** Daily backup alarm. */
class BackupReceiver : BroadcastReceiver() {
    override fun onReceive(ctx: Context, intent: Intent) {
        val pending = goAsync()
        Thread {
            try {
                Backup.runIfDue(ctx.applicationContext)
            } finally {
                pending.finish()
            }
        }.start()
    }
}
