package com.befikr.reminders

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Bundle
import android.text.TextUtils
import android.text.format.DateFormat
import android.text.format.DateUtils
import android.util.Log
import android.util.TypedValue
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.TextView
import android.widget.Toast
import org.json.JSONArray
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

internal const val EXTRA_REPEATS = "befikr.repeats"

/**
 * Notification "Snooze": a bottom sheet over whatever is on screen, without
 * opening the app. Quick picks come from core (the same as the app's snooze
 * sheet); "Pick a time" opens the app's sheet for the date & time picker and
 * typing. Closes on a choice, a tap outside, or Back.
 */
class SnoozeActivity : Activity() {
    private lateinit var taskId: String
    private lateinit var dim: View
    private lateinit var sheet: LinearLayout
    private var closing = false

    private val dark get() =
        resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    private val sheetColor get() = if (dark) 0xFF1C1C1E.toInt() else 0xFFF4F5F8.toInt()
    private val pickColor get() = if (dark) 0xFF2C2C2F.toInt() else Color.WHITE
    private val textColor get() = if (dark) 0xFFFAFAFA.toInt() else 0xFF161616.toInt()
    private val mutedColor get() = if (dark) 0xFFABABAB.toInt() else 0xFF626262.toInt()

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        taskId = intent.getStringExtra(EXTRA_ID) ?: return finish()
        val title = intent.getStringExtra(EXTRA_TITLE) ?: ""
        val repeats = intent.getBooleanExtra(EXTRA_REPEATS, false)

        // Draw under the navigation bar; the sheet pads itself above it.
        window.addFlags(WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS)
        window.navigationBarColor = Color.TRANSPARENT

        val root = FrameLayout(this)
        dim = View(this).apply {
            setBackgroundColor(0x66000000)
            alpha = 0f
            setOnClickListener { close() }
        }
        root.addView(dim, FrameLayout.LayoutParams(MATCH_PARENT, MATCH_PARENT))

        sheet = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = GradientDrawable().apply {
                setColor(sheetColor)
                val r = dp(24f)
                cornerRadii = floatArrayOf(r, r, r, r, 0f, 0f, 0f, 0f)
            }
            isClickable = true // taps inside don't reach the dim layer
            setPadding(dp(20), dp(10), dp(20), dp(20))
        }
        sheet.addView(handle())
        sheet.addView(text(title, 18f, textColor, bold = true).apply {
            maxLines = 2
            ellipsize = TextUtils.TruncateAt.END
            setPadding(dp(4), dp(10), dp(4), dp(14))
        })
        addPicks()
        sheet.addView(LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            setPadding(0, dp(6), 0, 0)
            addView(link("Pick a time") { openApp() })
            if (repeats) addView(link("Skip this one") { skip() })
        })
        root.addView(sheet, FrameLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT, Gravity.BOTTOM))
        sheet.setOnApplyWindowInsetsListener { v, insets ->
            @Suppress("DEPRECATION")
            v.setPadding(dp(20), dp(10), dp(20), dp(20) + insets.systemWindowInsetBottom)
            insets
        }
        setContentView(root)

        // Rise in.
        sheet.translationY = dp(480f)
        sheet.animate().translationY(0f).setDuration(260).start()
        dim.animate().alpha(1f).setDuration(200).start()
    }

    /** Two picks per row: label, then the time it lands on. */
    private fun addPicks() {
        val picks = try {
            JSONArray(Native.snoozePicks(TimeZone.getDefault().id))
        } catch (e: Exception) {
            Log.e(TAG, "snooze picks failed", e)
            JSONArray()
        }
        var row: LinearLayout? = null
        for (i in 0 until picks.length()) {
            val p = picks.getJSONObject(i)
            if (i % 2 == 0) {
                row = LinearLayout(this).apply { orientation = LinearLayout.HORIZONTAL }
                sheet.addView(row)
            }
            val label = p.getString("label")
            val at = p.getLong("atMs")
            row!!.addView(pick(label, whenLabel(label, at)) { snooze(at) })
        }
        // Odd count: keep the last pick half width.
        if (picks.length() % 2 == 1) row!!.addView(View(this), cell())
    }

    private fun snooze(untilMs: Long) =
        act("snooze", "Snoozed to ${dayTime(untilMs)}") { db -> Native.snooze(db, taskId, untilMs) }

    private fun skip() = act("skip", "Skipped") { db -> Native.onAction(db, taskId, "skip") }

    /** Write via core off the main thread, apply the alarm change, say so, close. */
    private fun act(what: String, done: String, call: (String) -> String) {
        if (closing) return
        val dbPath = Prefs.dbPath(this) ?: return close()
        sheet.performHapticFeedback(
            if (Build.VERSION.SDK_INT >= 30) HapticFeedbackConstants.CONFIRM else HapticFeedbackConstants.VIRTUAL_KEY
        )
        val ctx = applicationContext
        Thread {
            try {
                val change = parseOutcome(call(dbPath))
                if (change != null) {
                    Scheduler.apply(ctx, change)
                    toast(done)
                } else {
                    toast("Couldn't $what: open Befikr")
                }
            } catch (e: Exception) {
                Log.e(TAG, "$what failed", e)
            }
        }.start()
        close()
    }

    /** The app's own snooze sheet (date & time picker, typing). */
    private fun openApp() {
        val route = "/?snooze=$taskId"
        packageManager.getLaunchIntentForPackage(packageName)?.let {
            startActivity(
                it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_SINGLE_TOP or Intent.FLAG_ACTIVITY_CLEAR_TOP)
                    .putExtra(EXTRA_ROUTE, route)
            )
        }
        finish()
        overridePendingTransition(0, 0)
    }

    private fun close() {
        if (closing) return
        closing = true
        dim.animate().alpha(0f).setDuration(180).start()
        sheet.animate().translationY(sheet.height.toFloat()).setDuration(200).withEndAction {
            finish()
            overridePendingTransition(0, 0)
        }.start()
    }

    @Deprecated("Deprecated in Java")
    override fun onBackPressed() = close()

    // The activity may be gone by now: post to the main looper, not runOnUiThread.
    private fun toast(msg: String) {
        val ctx = applicationContext
        android.os.Handler(android.os.Looper.getMainLooper()).post { Toast.makeText(ctx, msg, Toast.LENGTH_SHORT).show() }
    }

    // "Tomorrow morning" already names the day: just the time. Otherwise "Mon 9:00".
    private fun whenLabel(label: String, at: Long) =
        if (DateUtils.isToday(at) || label.startsWith("Tomorrow")) time(at) else dayTime(at)

    private fun time(at: Long) = DateFormat.getTimeFormat(this).format(Date(at))

    private fun dayTime(at: Long) =
        if (DateUtils.isToday(at)) time(at)
        else "${SimpleDateFormat("EEE", Locale.getDefault()).format(Date(at))} ${time(at)}"

    private fun handle() = View(this).apply {
        background = GradientDrawable().apply { setColor(mutedColor); cornerRadius = dp(2f); alpha = 90 }
        layoutParams = LinearLayout.LayoutParams(dp(36), dp(4)).apply { gravity = Gravity.CENTER_HORIZONTAL }
    }

    private fun cell() = LinearLayout.LayoutParams(0, WRAP_CONTENT, 1f).apply { setMargins(dp(4), dp(4), dp(4), dp(4)) }

    private fun pick(label: String, sub: String, onClick: () -> Unit) = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        background = GradientDrawable().apply { setColor(pickColor); cornerRadius = dp(16f) }
        foreground = ripple()
        setPadding(dp(14), dp(12), dp(14), dp(12))
        layoutParams = cell()
        addView(text(label, 15.5f, textColor, bold = true))
        addView(text(sub, 13.5f, mutedColor))
        setOnClickListener { onClick() }
    }

    private fun link(label: String, onClick: () -> Unit) = text(label, 15f, mutedColor, bold = true).apply {
        setPadding(dp(8), dp(12), dp(16), dp(8))
        foreground = ripple()
        setOnClickListener { onClick() }
    }

    private fun ripple() = TypedValue().let {
        theme.resolveAttribute(android.R.attr.selectableItemBackground, it, true)
        getDrawable(it.resourceId)
    }

    private fun text(s: String, sp: Float, color: Int, bold: Boolean = false) = TextView(this).apply {
        text = s
        setTextColor(color)
        setTextSize(TypedValue.COMPLEX_UNIT_SP, sp)
        if (bold) typeface = Typeface.create(Typeface.DEFAULT, Typeface.BOLD)
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()
    private fun dp(v: Float) = v * resources.displayMetrics.density

    companion object {
        fun intent(ctx: Context, id: String, title: String, repeats: Boolean): Intent =
            Intent(ctx, SnoozeActivity::class.java)
                .setData(android.net.Uri.parse("befikr://snooze/$id"))
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK or Intent.FLAG_ACTIVITY_NO_ANIMATION)
                .putExtra(EXTRA_ID, id)
                .putExtra(EXTRA_TITLE, title)
                .putExtra(EXTRA_REPEATS, repeats)
    }
}
