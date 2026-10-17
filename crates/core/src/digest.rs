//! Morning digest: one quiet notification at a chosen local time listing
//! what's overdue and what's due later today. Worked out when it fires,
//! nothing stored; days are local to the caller's time zone.

use chrono::{DateTime, Duration, LocalResult, NaiveTime, TimeZone};
use chrono_tz::Tz;
use serde::Serialize;

use crate::recurrence::{from_ms, parse_tz, to_utc_ms};
use crate::store::Store;
use crate::Result;

/// Alarm id of the digest in the schedulers (no task has it: tasks use UUIDv7).
pub const DIGEST_ID: &str = "digest";
/// Default time, until changed in Settings.
pub const DEFAULT_DIGEST_TIME: &str = "08:00";
/// Lines in the expanded notification; the rest become "+N more".
const MAX_LINES: usize = 6;
/// Titles named in the collapsed one-line text.
const MAX_NAMED: usize = 2;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Digest {
    /// "Today: 4 tasks"
    pub title: String,
    /// One line for the collapsed notification: "1 overdue · Pay bill, Call mum, +2"
    pub text: String,
    /// Expanded: "Overdue · Pay bill", "10:00 · Call mum", …
    pub lines: Vec<String>,
}

impl Store {
    /// When the digest fires next, or `None` when it's off.
    pub fn next_digest_at(&self, tz: &str) -> Result<Option<i64>> {
        let Some(time) = self.settings()?.digest_time else { return Ok(None) };
        Ok(Some(next_at(from_ms(self.now(), parse_tz(tz)?), &time)))
    }

    /// The digest for now; `None` when nothing is overdue or due today.
    pub fn digest(&self, tz: &str) -> Result<Option<Digest>> {
        let tz = parse_tz(tz)?;
        let now = self.now();
        let end_of_day = local_ms(from_ms(now, tz) + Duration::days(1), 0, 0);
        let mut st = self.conn.prepare(
            "SELECT title, COALESCE(snoozed_until, due_at) AS due FROM tasks
             WHERE parent_id IS NULL AND completed_at IS NULL AND deleted_at IS NULL
               AND COALESCE(snoozed_until, due_at) < ?1
             ORDER BY due, id",
        )?;
        let tasks: Vec<(String, i64)> =
            st.query_map([end_of_day], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<rusqlite::Result<_>>()?;
        Ok(summarize(&tasks, now, tz))
    }
}

fn summarize(tasks: &[(String, i64)], now: i64, tz: Tz) -> Option<Digest> {
    if tasks.is_empty() {
        return None;
    }
    let n = tasks.len();
    let overdue = tasks.iter().filter(|(_, due)| *due < now).count();
    let mut lines: Vec<String> = tasks
        .iter()
        .map(|(title, due)| {
            let when = if *due < now { "Overdue".to_string() } else { from_ms(*due, tz).format("%H:%M").to_string() };
            format!("{when} · {title}")
        })
        .collect();
    if lines.len() > MAX_LINES {
        let more = lines.len() - (MAX_LINES - 1);
        lines.truncate(MAX_LINES - 1);
        lines.push(format!("+{more} more"));
    }
    let mut named: Vec<&str> = tasks.iter().take(MAX_NAMED).map(|(t, _)| t.as_str()).collect();
    let rest = format!("+{}", n.saturating_sub(MAX_NAMED));
    if n > MAX_NAMED {
        named.push(&rest);
    }
    let names = named.join(", ");
    Some(Digest {
        title: format!("Today: {n} {}", if n == 1 { "task" } else { "tasks" }),
        text: if overdue > 0 { format!("{overdue} overdue · {names}") } else { names },
        lines,
    })
}

/// Next `HH:MM` local after `now` (today if still ahead, else tomorrow).
fn next_at(now: DateTime<Tz>, time: &str) -> i64 {
    let (h, m) = time.split_once(':').map_or((8, 0), |(h, m)| (h.parse().unwrap_or(8), m.parse().unwrap_or(0)));
    let today = local_ms(now, h, m);
    if today > to_utc_ms(now) {
        today
    } else {
        local_ms(now + Duration::days(1), h, m)
    }
}

/// `h:m` local on `day`'s date; a time skipped by DST moves on an hour.
fn local_ms(day: DateTime<Tz>, h: u32, m: u32) -> i64 {
    let mut local = day.date_naive().and_time(NaiveTime::from_hms_opt(h, m, 0).unwrap_or_default());
    loop {
        match day.timezone().from_local_datetime(&local) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => return to_utc_ms(t),
            LocalResult::None => local += Duration::hours(1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: Tz = chrono_tz::Europe::Berlin;

    fn at(d: u32, h: u32, m: u32) -> i64 {
        to_utc_ms(TZ.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
    }

    #[test]
    fn next_is_today_until_the_time_then_tomorrow() {
        assert_eq!(next_at(from_ms(at(17, 7, 59), TZ), "08:00"), at(17, 8, 0));
        assert_eq!(next_at(from_ms(at(17, 8, 0), TZ), "08:00"), at(18, 8, 0));
        assert_eq!(next_at(from_ms(at(17, 21, 0), TZ), "07:30"), at(18, 7, 30));
    }

    #[test]
    fn next_skips_the_missing_dst_hour() {
        // Berlin springs forward on Mar 29, 2026: 02:30 doesn't exist.
        let before = to_utc_ms(TZ.with_ymd_and_hms(2026, 3, 28, 12, 0, 0).unwrap());
        let expected = to_utc_ms(TZ.with_ymd_and_hms(2026, 3, 29, 3, 30, 0).unwrap());
        assert_eq!(next_at(from_ms(before, TZ), "02:30"), expected);
    }

    #[test]
    fn nothing_due_is_no_digest() {
        assert_eq!(summarize(&[], at(17, 8, 0), TZ), None);
    }

    #[test]
    fn overdue_first_then_times() {
        let now = at(17, 8, 0);
        let tasks = vec![
            ("Pay bill".to_string(), at(16, 18, 0)),
            ("Call mum".to_string(), at(17, 10, 0)),
            ("Dentist".to_string(), at(17, 14, 30)),
        ];
        let d = summarize(&tasks, now, TZ).unwrap();
        assert_eq!(d.title, "Today: 3 tasks");
        assert_eq!(d.text, "1 overdue · Pay bill, Call mum, +1");
        assert_eq!(d.lines, ["Overdue · Pay bill", "10:00 · Call mum", "14:30 · Dentist"]);
    }

    #[test]
    fn one_task_and_long_lists() {
        let now = at(17, 8, 0);
        let d = summarize(&[("Call mum".to_string(), at(17, 10, 0))], now, TZ).unwrap();
        assert_eq!((d.title.as_str(), d.text.as_str()), ("Today: 1 task", "Call mum"));

        let many: Vec<_> = (0..9).map(|i| (format!("T{i}"), at(17, 9 + i, 0))).collect();
        let d = summarize(&many, now, TZ).unwrap();
        assert_eq!(d.lines.len(), MAX_LINES);
        assert_eq!(d.lines.last().unwrap(), "+4 more");
    }
}
