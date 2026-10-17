package com.befikr.reminders

import android.app.PendingIntent
import android.appwidget.AppWidgetManager
import android.appwidget.AppWidgetProvider
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Bundle
import android.util.Log
import android.view.View
import android.widget.RemoteViews
import org.json.JSONArray
import org.json.JSONObject
import java.util.TimeZone

/**
 * Home-screen widget: today's open tasks (overdue first) with a tick box
 * each, and New task. Ticking completes through the same path as the
 * notification's Complete (ActionReceiver → Rust), with the app closed.
 * Rows are fixed views rather than a list adapter, so each can carry its own
 * Complete and Open intents. Refreshed after every change ([refresh]) and
 * every 30 minutes (labels like "Overdue" move with the clock).
 */
class TasksWidget : AppWidgetProvider() {
    override fun onUpdate(ctx: Context, mgr: AppWidgetManager, ids: IntArray) {
        val tasks = load(ctx)
        ids.forEach { update(ctx, mgr, it, tasks) }
    }

    override fun onAppWidgetOptionsChanged(ctx: Context, mgr: AppWidgetManager, id: Int, options: Bundle) =
        update(ctx, mgr, id, load(ctx))

    companion object {
        private val ROWS = arrayOf(
            intArrayOf(R.id.row0, R.id.check0, R.id.title0, R.id.when0),
            intArrayOf(R.id.row1, R.id.check1, R.id.title1, R.id.when1),
            intArrayOf(R.id.row2, R.id.check2, R.id.title2, R.id.when2),
            intArrayOf(R.id.row3, R.id.check3, R.id.title3, R.id.when3),
            intArrayOf(R.id.row4, R.id.check4, R.id.title4, R.id.when4),
            intArrayOf(R.id.row5, R.id.check5, R.id.title5, R.id.when5),
        )
        /** Header and padding, then 40dp per row (see widget_today.xml). */
        private const val HEADER_DP = 64
        private const val ROW_DP = 40

        /** Redraw every placed widget (after a task changed anywhere). */
        fun refresh(ctx: Context) {
            try {
                val mgr = AppWidgetManager.getInstance(ctx) ?: return
                val ids = mgr.getAppWidgetIds(ComponentName(ctx, TasksWidget::class.java))
                if (ids.isEmpty()) return
                val tasks = load(ctx)
                ids.forEach { update(ctx, mgr, it, tasks) }
            } catch (e: Exception) {
                Log.w(TAG, "widget refresh failed", e)
            }
        }

        /** Today's tasks from Rust; null when the database isn't known yet. */
        private fun load(ctx: Context): JSONArray? {
            val dbPath = Prefs.dbPath(ctx) ?: return null
            val out = parseOutcome(Native.today(dbPath, TimeZone.getDefault().id)) ?: return null
            return out.optJSONArray("tasks")
        }

        private fun update(ctx: Context, mgr: AppWidgetManager, widgetId: Int, tasks: JSONArray?) {
            val views = RemoteViews(ctx.packageName, R.layout.widget_today)
            views.setOnClickPendingIntent(R.id.widget_header, launchPendingIntent(ctx, "widget", "/"))
            views.setOnClickPendingIntent(R.id.widget_add, launchPendingIntent(ctx, "widget", "/?add=1"))

            val n = tasks?.length() ?: 0
            val height = mgr.getAppWidgetOptions(widgetId).getInt(AppWidgetManager.OPTION_APPWIDGET_MAX_HEIGHT, 180)
            val fit = ((height - HEADER_DP) / ROW_DP).coerceIn(1, ROWS.size)
            // When some don't fit, one row's space goes to "+N more".
            val shown = if (n > fit) (fit - 1).coerceAtLeast(1) else n

            views.setTextViewText(R.id.widget_header, if (n == 0) "Today" else "Today · $n")
            views.setViewVisibility(R.id.widget_empty, if (tasks != null && n == 0) View.VISIBLE else View.GONE)
            for ((i, ids) in ROWS.withIndex()) {
                val task = if (i < shown) tasks?.optJSONObject(i) else null
                if (task == null) {
                    views.setViewVisibility(ids[0], View.GONE)
                    continue
                }
                row(ctx, views, ids, task)
            }
            if (n > shown) {
                views.setTextViewText(R.id.widget_more, "+${n - shown} more")
                views.setOnClickPendingIntent(R.id.widget_more, launchPendingIntent(ctx, "widget", "/"))
                views.setViewVisibility(R.id.widget_more, View.VISIBLE)
            } else {
                views.setViewVisibility(R.id.widget_more, View.GONE)
            }
            mgr.updateAppWidget(widgetId, views)
        }

        private fun row(ctx: Context, views: RemoteViews, ids: IntArray, task: JSONObject) {
            val id = task.getString("id")
            views.setViewVisibility(ids[0], View.VISIBLE)
            views.setTextViewText(ids[2], task.getString("title"))
            views.setTextViewText(ids[3], task.getString("when"))
            val color = if (task.optBoolean("overdue")) R.color.widget_overdue else R.color.widget_muted
            views.setTextColor(ids[3], ctx.getColor(color))
            views.setOnClickPendingIntent(ids[1], completeIntent(ctx, id))
            views.setOnClickPendingIntent(ids[2], launchPendingIntent(ctx, id, "/?task=$id"))
        }

        /** Same broadcast as the notification's Complete; one per task (data URI). */
        private fun completeIntent(ctx: Context, id: String): PendingIntent = PendingIntent.getBroadcast(
            ctx, 0,
            Intent(ctx, ActionReceiver::class.java).setAction(ACTION_COMPLETE)
                .setData(Uri.parse("befikr://widget-complete/$id")).putExtra(EXTRA_ID, id),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )
    }
}
