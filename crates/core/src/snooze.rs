//! Snooze quick picks ("In 1 hour", "Tomorrow morning", …), shared by the
//! app's snooze sheet and the Android snooze overlay (no webview there).

use chrono::{DateTime, Datelike, Duration, LocalResult, NaiveTime, TimeZone, Timelike};
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

/// `h:00` local, `days` after `now`'s date (a skipped DST hour moves on an hour).
fn at(now: DateTime<Tz>, days: i64, h: u32) -> i64 {
    let date = now.date_naive() + Duration::days(days);
    let mut local = date.and_time(NaiveTime::from_hms_opt(h, 0, 0).unwrap_or_default());
    loop {
        match now.timezone().from_local_datetime(&local) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => return to_utc_ms(t),
            LocalResult::None => local += Duration::hours(1),
        }
    }
}

/// Up to six picks, in time order, for `now_ms` in time zone `tz`.
pub fn snooze_picks(now_ms: i64, tz: &str) -> Result<Vec<SnoozePick>> {
    let now = from_ms(now_ms, parse_tz(tz)?);
    let pick = |label: &str, at_ms| SnoozePick { label: label.into(), at_ms };
    let mut list = vec![
        pick("In 1 hour", round_up(now_ms + HOUR)),
        pick("In 3 hours", round_up(now_ms + 3 * HOUR)),
    ];
    if now.hour() < 18 {
        list.push(pick("This evening", at(now, 0, 18)));
    }
    if now.hour() < 21 {
        list.push(pick("Tonight", at(now, 0, 21)));
    }
    list.push(pick("Tomorrow morning", at(now, 1, 9)));
    list.push(pick("Tomorrow evening", at(now, 1, 18)));
    // Monday 9:00; on a Monday, the one after.
    let to_monday = 7 - i64::from(now.weekday().num_days_from_monday());
    list.push(pick("Next week", at(now, to_monday, 9)));
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

    #[test]
    fn afternoon_offers_evening_and_tonight() {
        // Thursday 15:07.
        let p = snooze_picks(ms("2026-10-15 15:07"), TZ).unwrap();
        assert_eq!(labels(&p), ["In 1 hour", "In 3 hours", "This evening", "Tonight", "Tomorrow morning", "Tomorrow evening"]);
        assert_eq!(p[0].at_ms, ms("2026-10-15 16:10"));
        assert_eq!(p[2].at_ms, ms("2026-10-15 18:00"));
        assert_eq!(p[4].at_ms, ms("2026-10-16 09:00"));
    }

    #[test]
    fn late_evening_offers_next_week() {
        // Thursday 22:00: no evening or tonight left.
        let p = snooze_picks(ms("2026-10-15 22:00"), TZ).unwrap();
        assert_eq!(labels(&p), ["In 1 hour", "In 3 hours", "Tomorrow morning", "Tomorrow evening", "Next week"]);
        assert_eq!(p[0].at_ms, ms("2026-10-15 23:00"));
        assert_eq!(p[4].at_ms, ms("2026-10-19 09:00"));
    }

    #[test]
    fn next_week_from_monday_is_the_monday_after() {
        let p = snooze_picks(ms("2026-10-19 21:30"), TZ).unwrap();
        assert_eq!(p.last().unwrap().at_ms, ms("2026-10-26 09:00"));
    }

    #[test]
    fn unknown_zone_is_an_error() {
        assert!(snooze_picks(0, "Mars/Base").is_err());
    }
}
