//! Carefree days (gamification): worked out from completions, skips and open
//! tasks, nothing stored. A day is missed when something due on or before it
//! was still undone at noon the next day; rescheduled or deleted tasks don't
//! count against it. A snooze moves the due time (no limit), so snoozing
//! never breaks a streak by itself. Days are local to the caller's time zone.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};

use crate::model::{Carefree, HabitStreak};
use crate::recurrence::{from_ms, parse_tz};
use crate::store::Store;
use crate::Result;

/// A missed day is forgiven when the last forgiven one is at least this many days earlier.
const GRACE_DAYS: i64 = 7;

impl Store {
    pub fn carefree(&self, tz: &str) -> Result<Carefree> {
        let tz = parse_tz(tz)?;
        let now = self.now();
        let local_now = from_ms(now, tz);
        let day = |ms: i64| from_ms(ms, tz).date_naive();
        // Last day whose noon-next-day cutoff has passed.
        let last = local_now.date_naive() - Duration::days(if local_now.time() >= noon() { 1 } else { 2 });

        let first: Option<i64> = self.conn.query_row(
            "SELECT MIN(created_at) FROM tasks WHERE parent_id IS NULL AND deleted_at IS NULL",
            [],
            |r| r.get(0),
        )?;
        let Some(first) = first.map(day) else {
            return Ok(Carefree::default());
        };

        // (due, resolved) for every dated occurrence; open ones resolve "now".
        // Due = the snoozed time when snoozed.
        let mut st = self.conn.prepare(&format!(
            "SELECT {EFFECTIVE_DUE}, c.completed_at FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.deleted_at IS NULL AND c.not_done_at IS NULL AND t.deleted_at IS NULL AND c.due_at_then IS NOT NULL
             UNION ALL
             SELECT e.due_at_then, e.at FROM task_events e JOIN tasks t ON t.id = e.task_id
             WHERE e.kind = 'skipped' AND t.deleted_at IS NULL AND e.due_at_then IS NOT NULL
             UNION ALL
             SELECT COALESCE(snoozed_until, due_at), ?1 FROM tasks
             WHERE completed_at IS NULL AND deleted_at IS NULL AND due_at IS NOT NULL"
        ))?;
        let spans = st.query_map([now], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
        let mut missed = BTreeSet::new();
        for s in spans {
            let (due, done) = s?;
            let done_local = from_ms(done, tz);
            // Missed through the day before `done`'s day, or two before when done by noon.
            let back = if done_local.time() > noon() { 1 } else { 2 };
            let mut d = day(due);
            let until = (done_local.date_naive() - Duration::days(back)).min(last);
            while d <= until {
                missed.insert(d);
                d += Duration::days(1);
            }
        }
        // Marked not done afterwards: its due day is missed.
        let mut st = self.conn.prepare(&format!(
            "SELECT {EFFECTIVE_DUE} FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.not_done_at IS NOT NULL AND c.deleted_at IS NULL AND t.deleted_at IS NULL AND c.due_at_then IS NOT NULL"
        ))?;
        for due in st.query_map([], |r| r.get::<_, i64>(0))? {
            let d = day(due?);
            if d <= last {
                missed.insert(d);
            }
        }

        let (mut run, mut best) = (0u32, 0u32);
        let mut forgiven: Option<NaiveDate> = None;
        let mut d = first;
        while d <= last {
            if !missed.contains(&d) {
                run += 1;
            } else if forgiven.is_none_or(|f| (d - f).num_days() >= GRACE_DAYS) && run > 0 {
                forgiven = Some(d);
            } else {
                run = 0;
                forgiven = None;
            }
            best = best.max(run);
            d += Duration::days(1);
        }

        let monday = local_now.date_naive() - Duration::days(local_now.weekday().num_days_from_monday() as i64);
        let (on_time_total, on_time_week) = self.on_time_counts(tz, monday)?;
        Ok(Carefree { streak: run, best, on_time_total, on_time_week })
    }

    /// Completions done on or before their due day: all time, and since `monday`.
    fn on_time_counts(&self, tz: chrono_tz::Tz, monday: NaiveDate) -> Result<(u32, u32)> {
        let mut st = self.conn.prepare(
            "SELECT c.due_at_then, c.completed_at FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.deleted_at IS NULL AND c.not_done_at IS NULL AND t.deleted_at IS NULL AND c.due_at_then IS NOT NULL",
        )?;
        let rows = st.query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?)))?;
        let (mut total, mut week) = (0, 0);
        for r in rows {
            let (due, done) = r?;
            let done_day = from_ms(done, tz).date_naive();
            if done_day <= from_ms(due, tz).date_naive() {
                total += 1;
                if done_day >= monday {
                    week += 1;
                }
            }
        }
        Ok((total, week))
    }
}

impl Store {
    /// Streak of a repeating task, `None` for one-off tasks.
    pub fn habit_streak(&self, task_id: &str, tz: &str) -> Result<Option<HabitStreak>> {
        let tz = parse_tz(tz)?;
        let now = self.now();
        let task = self.get_task(task_id)?;
        if task.rrule.is_none() {
            return Ok(None);
        }
        // Done by noon the day after its due day.
        let in_time = |due: i64, at: i64| {
            let cutoff = (from_ms(due, tz).date_naive() + Duration::days(1)).and_time(noon());
            from_ms(at, tz).naive_local() <= cutoff
        };
        // (due, Some(on time)) for completions, (due, Some(false)) for missed
        // occurrences (and completions marked not done), (due, None) for skips.
        let mut st = self.conn.prepare(&format!(
            "SELECT {EFFECTIVE_DUE}, c.completed_at, CASE WHEN c.not_done_at IS NULL THEN 'done' ELSE 'missed' END FROM completions c
             WHERE c.task_id = ?1 AND c.deleted_at IS NULL AND c.due_at_then IS NOT NULL
             UNION ALL
             SELECT e.due_at_then, e.at, e.kind FROM task_events e
             WHERE e.task_id = ?1 AND e.kind IN ('missed', 'skipped') AND e.due_at_then IS NOT NULL
               AND NOT EXISTS (SELECT 1 FROM completions c
                 WHERE c.task_id = e.task_id AND c.completed_at = e.at AND c.deleted_at IS NOT NULL)"
        ))?;
        let mut occ: Vec<(i64, Option<bool>)> = st
            .query_map([task_id], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, String>(2)?)))?
            .map(|r| {
                r.map(|(due, at, kind)| match kind.as_str() {
                    "done" => (due, Some(in_time(due, at))),
                    "missed" => (due, Some(false)),
                    _ => (due, None),
                })
            })
            .collect::<rusqlite::Result<_>>()?;
        // The current one, once its grace has passed undone (counted from a snooze).
        if let (true, Some(due)) = (task.is_open(), task.effective_due()) {
            if !in_time(due, now) {
                occ.push((due, Some(false)));
            }
        }
        occ.sort_by_key(|o| o.0);
        let (mut current, mut best) = (0u32, 0u32);
        for (_, ok) in occ {
            match ok {
                Some(true) => current += 1,
                Some(false) => current = 0,
                None => {}
            }
            best = best.max(current);
        }
        Ok(Some(HabitStreak { current, best }))
    }
}

/// A completion's due time as it was then, snooze included: the "completed"
/// event logs it (`completions.due_at_then` keeps the schedule's, for undo).
const EFFECTIVE_DUE: &str = "COALESCE((SELECT e.due_at_then FROM task_events e
    WHERE e.task_id = c.task_id AND e.kind = 'completed' AND e.at = c.completed_at LIMIT 1), c.due_at_then)";

fn noon() -> NaiveTime {
    NaiveTime::from_hms_opt(12, 0, 0).unwrap()
}

