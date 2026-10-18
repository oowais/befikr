package com.befikr.reminders

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.speech.RecognizerIntent
import android.webkit.WebView
import androidx.activity.result.ActivityResult
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationManagerCompat
import app.tauri.annotation.ActivityCallback
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin
import java.util.Locale

@InvokeArg
class ScheduleArgs {
    lateinit var dbPath: String
    lateinit var id: String
    lateinit var title: String
    var atMs: Long = 0
    var keepShown: Boolean = false
}

@InvokeArg
class CancelArgs {
    lateinit var id: String
}

@InvokeArg
class InstallUpdateArgs {
    lateinit var url: String
}

@InvokeArg
class ConfigureArgs {
    lateinit var dbPath: String
}

@TauriPlugin
class RemindersPlugin(private val activity: Activity) : Plugin(activity) {
    private var launchRoute: String? = null

    override fun load(webView: WebView) {
        // Page background until the splash paints (no white flash on cold start).
        webView.setBackgroundColor(activity.getColor(R.color.befikr_window))
        Notifier.ensureChannel(activity)
        launchRoute = routeFor(activity.intent)
    }

    override fun onNewIntent(intent: Intent) {
        val route = routeFor(intent) ?: return
        launchRoute = route
        trigger("launchRoute", JSObject().apply { put("route", route) })
    }

    /** Sets the task's next reminder; clears any notification currently shown for it unless `keepShown` (re-arm). */
    @Command
    fun schedule(invoke: Invoke) {
        val args = invoke.parseArgs(ScheduleArgs::class.java)
        Prefs.setDbPath(activity, args.dbPath)
        if (!args.keepShown) Notifier.cancel(activity, args.id)
        Scheduler.schedule(activity, args.id, args.title, args.atMs)
        TasksWidget.refresh(activity)
        invoke.resolve()
    }

    /** Back on the first page: to the background like other apps (the process stays, so it reopens warm). */
    @Command
    fun moveToBack(invoke: Invoke) {
        activity.moveTaskToBack(true)
        invoke.resolve()
    }

    @Command
    fun cancel(invoke: Invoke) {
        val args = invoke.parseArgs(CancelArgs::class.java)
        Scheduler.cancel(activity, args.id)
        Notifier.cancel(activity, args.id)
        TasksWidget.refresh(activity)
        invoke.resolve()
    }

    @Command
    fun status(invoke: Invoke) {
        val am = activity.getSystemService(AlarmManager::class.java)
        val pm = activity.getSystemService(PowerManager::class.java)
        invoke.resolve(JSObject().apply {
            put("notifications", NotificationManagerCompat.from(activity).areNotificationsEnabled())
            put("exactAlarms", Build.VERSION.SDK_INT < Build.VERSION_CODES.S || am.canScheduleExactAlarms())
            put("batteryUnrestricted", pm.isIgnoringBatteryOptimizations(activity.packageName))
            put("sdk", Build.VERSION.SDK_INT)
            put("device", "${Build.MANUFACTURER} ${Build.MODEL}")
        })
    }

    @Command
    fun requestNotifications(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) {
            ActivityCompat.requestPermissions(activity, arrayOf(Manifest.permission.POST_NOTIFICATIONS), 4201)
        }
        invoke.resolve()
    }

    @Command
    fun openExactAlarmSettings(invoke: Invoke) {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
            activity.startActivity(
                Intent(Settings.ACTION_REQUEST_SCHEDULE_EXACT_ALARM, Uri.parse("package:${activity.packageName}"))
            )
        }
        invoke.resolve()
    }

    @Command
    fun openBatterySettings(invoke: Invoke) {
        activity.startActivity(
            Intent(Settings.ACTION_REQUEST_IGNORE_BATTERY_OPTIMIZATIONS, Uri.parse("package:${activity.packageName}"))
        )
        invoke.resolve()
    }

    /** Where the database lives (for receivers and backups); runs a due backup. */
    @Command
    fun configure(invoke: Invoke) {
        Prefs.setDbPath(activity, invoke.parseArgs(ConfigureArgs::class.java).dbPath)
        Backup.scheduleDaily(activity)
        SyncWorker.schedule(activity)
        Digest.scheduleNext(activity)
        TasksWidget.refresh(activity)
        Thread { Backup.runIfDue(activity.applicationContext) }.start()
        invoke.resolve()
    }

    /** The digest time changed (or was turned off): re-arm it. */
    @Command
    fun scheduleDigest(invoke: Invoke) {
        Digest.scheduleNext(activity)
        invoke.resolve()
    }

    // ---- Voice quick add ----

    /** The phone's own speech recogniser (whatever app provides it), English for the parser. */
    private fun speechIntent() = Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
        .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
        .putExtra(RecognizerIntent.EXTRA_LANGUAGE, if (Locale.getDefault().language == "en") Locale.getDefault().toLanguageTag() else "en-US")
        .putExtra(RecognizerIntent.EXTRA_PREFER_OFFLINE, true)
        .putExtra(RecognizerIntent.EXTRA_PROMPT, "Say a task")

    /** Whether any app can take speech input; the mic button shows only then. */
    @Command
    fun speechAvailable(invoke: Invoke) {
        val ok = speechIntent().resolveActivity(activity.packageManager) != null
        invoke.resolve(JSObject().apply { put("available", ok) })
    }

    /** Open the recogniser; resolves `{ text }` (null when cancelled). */
    @Command
    fun recognizeSpeech(invoke: Invoke) {
        val intent = speechIntent()
        if (intent.resolveActivity(activity.packageManager) == null) return invoke.reject("No speech input app on this phone")
        startActivityForResult(invoke, intent, "onSpeech")
    }

    @ActivityCallback
    fun onSpeech(invoke: Invoke, result: ActivityResult) {
        val text = result.data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()
            ?.takeIf { result.resultCode == Activity.RESULT_OK }
        invoke.resolve(JSObject().apply { put("text", text) })
    }

    // ---- Update ----

    /**
     * Download the APK at `url` and start installing it (the system asks to
     * confirm). Without "Install unknown apps" for Befikr, opens that setting
     * instead and resolves `{ needsPermission: true }`. Emits `updateProgress`
     * `{ done, total }` while downloading (`total` -1 if unknown).
     */
    @Command
    fun installUpdate(invoke: Invoke) {
        val url = invoke.parseArgs(InstallUpdateArgs::class.java).url
        if (!Updater.canInstall(activity)) {
            Updater.openInstallSettings(activity)
            invoke.resolve(JSObject().apply { put("needsPermission", true) })
            return
        }
        background(invoke) {
            Updater.downloadAndInstall(activity.applicationContext, url) { done, total ->
                trigger("updateProgress", JSObject().apply { put("done", done); put("total", total) })
            }
            JSObject().apply { put("needsPermission", false) }
        }
    }

    // ---- Backups ----

    private fun background(invoke: Invoke, work: () -> JSObject) {
        Thread {
            try {
                invoke.resolve(work())
            } catch (e: Exception) {
                invoke.reject(e.message ?: e.toString())
            }
        }.start()
    }

    private fun status() = JSObject(Backup.status(activity).toString())

    @Command
    fun backupStatus(invoke: Invoke) = invoke.resolve(status())

    @Command
    fun pickBackupFolder(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT_TREE)
            .addFlags(Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION)
            .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION)
        startActivityForResult(invoke, intent, "onBackupFolder")
    }

    @ActivityCallback
    fun onBackupFolder(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(status())
        activity.contentResolver.takePersistableUriPermission(
            uri, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION
        )
        Backup.tree(activity)?.takeIf { it != uri }?.let { old ->
            runCatching { activity.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
        }
        Backup.setTree(activity, uri)
        // First backup right away, so the folder is proven writable.
        background(invoke) {
            runCatching { Backup.runNow(activity) }
            status()
        }
    }

    @Command
    fun clearBackupFolder(invoke: Invoke) {
        Backup.tree(activity)?.let { old ->
            runCatching { activity.contentResolver.releasePersistableUriPermission(old, Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION) }
        }
        Backup.setTree(activity, null)
        invoke.resolve(status())
    }

    @Command
    fun backupNow(invoke: Invoke) = background(invoke) {
        Backup.runNow(activity)
        status()
    }

    /** Save one copy wherever the user chooses. */
    @Command
    fun exportBackup(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT)
            .addCategory(Intent.CATEGORY_OPENABLE)
            .setType("application/octet-stream")
            .putExtra(Intent.EXTRA_TITLE, Backup.fileName())
        startActivityForResult(invoke, intent, "onExportFile")
    }

    @ActivityCallback
    fun onExportFile(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(JSObject().apply { put("saved", false) })
        background(invoke) {
            Backup.exportTo(activity, uri)
            JSObject().apply { put("saved", true) }
        }
    }

    /** Pick a backup file; it is copied into the cache and its path returned for Rust to restore. */
    @Command
    fun pickImportFile(invoke: Invoke) {
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).addCategory(Intent.CATEGORY_OPENABLE).setType("*/*")
        startActivityForResult(invoke, intent, "onImportFile")
    }

    @ActivityCallback
    fun onImportFile(invoke: Invoke, result: ActivityResult) {
        val uri = result.data?.data
        if (result.resultCode != Activity.RESULT_OK || uri == null) return invoke.resolve(JSObject())
        background(invoke) {
            val file = Backup.importToCache(activity, uri)
            JSObject().apply { put("path", file.absolutePath) }
        }
    }

    /**
     * Route for an intent that opened the app: a notification's (e.g. snooze),
     * or "Share → Befikr" from another app, which opens quick add prefilled.
     */
    private fun routeFor(intent: Intent?): String? {
        if (intent == null) return null
        intent.getStringExtra(EXTRA_ROUTE)?.let { return it }
        if (intent.action != Intent.ACTION_SEND || intent.type?.startsWith("text/") != true) return null
        val text = intent.getStringExtra(Intent.EXTRA_TEXT) ?: return null
        val subject = intent.getStringExtra(Intent.EXTRA_SUBJECT) ?: ""
        return "/?shareText=${Uri.encode(text)}&shareSubject=${Uri.encode(subject)}"
    }

    /** Route the app was opened with (notification or share), once. */
    @Command
    fun takeLaunchRoute(invoke: Invoke) {
        val route = launchRoute
        launchRoute = null
        invoke.resolve(JSObject().apply { put("route", route) })
    }
}
