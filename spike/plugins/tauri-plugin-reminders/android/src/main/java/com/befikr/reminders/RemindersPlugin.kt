package com.befikr.reminders

import android.Manifest
import android.app.Activity
import android.app.AlarmManager
import android.content.Intent
import android.net.Uri
import android.os.Build
import android.os.PowerManager
import android.provider.Settings
import android.webkit.WebView
import androidx.core.app.ActivityCompat
import androidx.core.app.NotificationManagerCompat
import app.tauri.annotation.Command
import app.tauri.annotation.InvokeArg
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.JSObject
import app.tauri.plugin.Plugin

@InvokeArg
class ScheduleArgs {
    lateinit var dbPath: String
    lateinit var id: String
    lateinit var title: String
    var atMs: Long = 0
    var mode: String = "exact"
}

@InvokeArg
class CancelArgs {
    lateinit var id: String
}

@TauriPlugin
class RemindersPlugin(private val activity: Activity) : Plugin(activity) {
    private var launchRoute: String? = null

    override fun load(webView: WebView) {
        Notifier.ensureChannel(activity)
        launchRoute = activity.intent?.getStringExtra(EXTRA_ROUTE)
    }

    override fun onNewIntent(intent: Intent) {
        val route = intent.getStringExtra(EXTRA_ROUTE) ?: return
        launchRoute = route
        trigger("launchRoute", JSObject().apply { put("route", route) })
    }

    /** Sets the task's next reminder; clears any notification currently shown for it. */
    @Command
    fun schedule(invoke: Invoke) {
        val args = invoke.parseArgs(ScheduleArgs::class.java)
        Prefs.setDbPath(activity, args.dbPath)
        Notifier.cancel(activity, args.id)
        Scheduler.schedule(activity, args.id, args.title, args.atMs, args.mode)
        invoke.resolve()
    }

    @Command
    fun cancel(invoke: Invoke) {
        val args = invoke.parseArgs(CancelArgs::class.java)
        Scheduler.cancel(activity, args.id)
        Notifier.cancel(activity, args.id)
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

    /** Route the app was opened with from a notification (e.g. snooze), once. */
    @Command
    fun takeLaunchRoute(invoke: Invoke) {
        val route = launchRoute
        launchRoute = null
        invoke.resolve(JSObject().apply { put("route", route) })
    }
}
