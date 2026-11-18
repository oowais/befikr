//! Snooze quick picks ("In 1 hour", "Tomorrow morning", …), shared by the
//! app's snooze sheet and the Android snooze overlay (no webview there).

use chrono::{DateTime, Datelike, Duration, LocalResult, Months, NaiveDate, NaiveTime, TimeZone, Timelike};
use chrono_tz::Tz;
use serde::Serialize;

use crate::recurrence::{from_ms, parse_tz, to_utc_ms};
use crate::Result;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnoozePick {
    pub label: String,
    pub at_ms: i64,
}

const HOUR: i64 = 3_600_000;
/// Relative picks land on the next 5 minutes: 15:10, not 15:07.
const FIVE: i64 = 5 * 60_000;
const MAX: usize = 6;

fn round_up(ms: i64) -> i64 {
    (ms + FIVE - 1).div_euclid(FIVE) * FIVE
}

/// `h:00` local, `days` after `now`'s date.
fn at(now: DateTime<Tz>, days: i64, h: u32) -> i64 {
    at_date(now.timezone(), now.date_naive() + Duration::days(days), h)
}

/// `h:00` local on `date` (a skipped DST hour moves on an hour).
fn at_date(tz: Tz, date: NaiveDate, h: u32) -> i64 {
    let mut local = date.and_time(NaiveTime::from_hms_opt(h, 0, 0).unwrap_or_default());
    loop {
        match tz.from_local_datetime(&local) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => return to_utc_ms(t),
            LocalResult::None => local += Duration::hours(1),
        }
    }
}

/// Up to six picks, in time order, for `now_ms` in time zone `tz`: two
/// relative ones, the next of this evening / tonight, then tomorrow, next
/// week and next month (all 9:00).
pub fn snooze_picks(now_ms: i64, tz: &str) -> Result<Vec<SnoozePick>> {
    let now = from_ms(now_ms, parse_tz(tz)?);
    let pick = |label: &str, at_ms| SnoozePick { label: label.into(), at_ms };
    let mut list = vec![
        pick("In 1 hour", round_up(now_ms + HOUR)),
        pick("In 3 hours", round_up(now_ms + 3 * HOUR)),
    ];
    if now.hour() < 18 {
        list.push(pick("This evening", at(now, 0, 18)));
    } else if now.hour() < 21 {
        list.push(pick("Tonight", at(now, 0, 21)));
    }
    list.push(pick("Tomorrow", at(now, 1, 9)));
    // Monday 9:00; on a Monday, the one after.
    let to_monday = 7 - i64::from(now.weekday().num_days_from_monday());
    list.push(pick("Next week", at(now, to_monday, 9)));
    // Same date next month; Jan 31 → Feb 28.
    let next_month = now.date_naive().checked_add_months(Months::new(1)).unwrap_or(now.date_naive());
    list.push(pick("Next month", at_date(now.timezone(), next_month, 9)));
    list.sort_by_key(|p| p.at_ms);
    list.truncate(MAX);
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: &str = "Europe/Berlin";

    fn ms(s: &str) -> i64 {
        let tz: Tz = TZ.parse().unwrap();
        let local = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M").unwrap();
        to_utc_ms(tz.from_local_datetime(&local).unwrap())
    }

    fn labels(p: &[SnoozePick]) -> Vec<&str> {
        p.iter().map(|p| p.label.as_str()).collect()
    }

    const ALL: [&str; 6] = ["In 1 hour", "In 3 hours", "This evening", "Tomorrow", "Next week", "Next month"];

    #[test]
    fn afternoon_offers_this_evening() {
        // Thursday 14:07.
        let p = snooze_picks(ms("2026-10-15 14:07"), TZ).unwrap();
        assert_eq!(labels(&p), ALL);
        assert_eq!(p[0].at_ms, ms("2026-10-15 15:10"));
        assert_eq!(p[2].at_ms, ms("2026-10-15 18:00"));
        assert_eq!(p[3].at_ms, ms("2026-10-16 09:00"));
        assert_eq!(p[4].at_ms, ms("2026-10-19 09:00"));
        assert_eq!(p[5].at_ms, ms("2026-11-15 09:00"));
    }

    #[test]
    fn evening_offers_tonight() {
        let p = snooze_picks(ms("2026-10-15 19:00"), TZ).unwrap();
        assert_eq!(labels(&p), ["In 1 hour", "Tonight", "In 3 hours", "Tomorrow", "Next week", "Next month"]);
        assert_eq!(p[1].at_ms, ms("2026-10-15 21:00"));
    }

    #[test]
    fn late_evening_has_no_evening_pick() {
        // Thursday 22:00.
        let p = snooze_picks(ms("2026-10-15 22:00"), TZ).unwrap();
        assert_eq!(labels(&p), ["In 1 hour", "In 3 hours", "Tomorrow", "Next week", "Next month"]);
        assert_eq!(p[0].at_ms, ms("2026-10-15 23:00"));
    }

    #[test]
    fn in_3_hours_after_this_evening_keeps_time_order() {
        // 16:30: in 3 hours (19:30) comes after this evening (18:00).
        let p = snooze_picks(ms("2026-10-15 16:30"), TZ).unwrap();
        assert_eq!(labels(&p), ["In 1 hour", "This evening", "In 3 hours", "Tomorrow", "Next week", "Next month"]);
    }

    #[test]
    fn next_month_on_a_short_month_is_its_last_day() {
        let p = snooze_picks(ms("2027-01-31 10:00"), TZ).unwrap();
        assert_eq!(p.last().unwrap().at_ms, ms("2027-02-28 09:00"));
    }

    #[test]
    fn next_week_from_monday_is_the_monday_after() {
        let p = snooze_picks(ms("2026-10-19 21:30"), TZ).unwrap();
        assert_eq!(p[p.len() - 2].at_ms, ms("2026-10-26 09:00"));
    }

    #[test]
    fn unknown_zone_is_an_error() {
        assert!(snooze_picks(0, "Mars/Base").is_err());
    }
}
