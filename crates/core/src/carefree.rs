//! Carefree days (gamification): worked out from completions, skips and open
//! tasks, nothing stored. A day is missed when something due on or before it
//! was still undone at noon the next day; rescheduled or deleted tasks don't
//! count against it. Days are local to the caller's time zone.

use std::collections::BTreeSet;

use chrono::{Datelike, Duration, NaiveDate, NaiveTime};

use crate::model::Carefree;
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
        let mut st = self.conn.prepare(
            "SELECT c.due_at_then, c.completed_at FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.deleted_at IS NULL AND t.deleted_at IS NULL AND c.due_at_then IS NOT NULL
             UNION ALL
             SELECT e.due_at_then, e.at FROM task_events e JOIN tasks t ON t.id = e.task_id
             WHERE e.kind = 'skipped' AND t.deleted_at IS NULL AND e.due_at_then IS NOT NULL
             UNION ALL
             SELECT due_at, ?1 FROM tasks
             WHERE completed_at IS NULL AND deleted_at IS NULL AND due_at IS NOT NULL",
        )?;
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
             WHERE c.deleted_at IS NULL AND t.deleted_at IS NULL AND c.due_at_then IS NOT NULL",
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

fn noon() -> NaiveTime {
    NaiveTime::from_hms_opt(12, 0, 0).unwrap()
}

