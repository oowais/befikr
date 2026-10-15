//! Next-occurrence math. Runs in the task's own time zone so "every day 9am"
//! stays at 9am across DST changes.

use chrono::{DateTime, Duration, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use rrule::RRuleSet;

use crate::{Error, Result};

pub fn parse_tz(tz: &str) -> Result<Tz> {
    tz.parse::<Tz>().map_err(|_| Error::Invalid(format!("unknown time zone: {tz}")))
}

pub fn to_utc_ms(dt: DateTime<Tz>) -> i64 {
    dt.with_timezone(&Utc).timestamp_millis()
}

pub fn from_ms(ms: i64, tz: Tz) -> DateTime<Tz> {
    Utc.timestamp_millis_opt(ms).single().unwrap_or_default().with_timezone(&tz)
}

/// Validate an RRULE body (e.g. `FREQ=WEEKLY;BYDAY=MO`).
pub fn validate(rrule: &str) -> Result<()> {
    set(rrule, Utc.timestamp_millis_opt(0).unwrap().with_timezone(&Tz::UTC)).map(|_| ())
}

fn set(rrule: &str, dtstart: DateTime<Tz>) -> Result<RRuleSet> {
    if rrule.contains('\n') || rrule.to_ascii_uppercase().contains("DTSTART") {
        return Err(Error::Invalid("rrule must be a bare RRULE body".into()));
    }
    let tz_name = dtstart.timezone().name();
    let local = dtstart.format("%Y%m%dT%H%M%S");
    let text = if tz_name == "UTC" {
        format!("DTSTART:{local}Z\nRRULE:{rrule}")
    } else {
        format!("DTSTART;TZID={tz_name}:{local}\nRRULE:{rrule}")
    };
    text.parse::<RRuleSet>().map_err(|e| Error::Invalid(format!("rrule: {e}")))
}

/// First occurrence strictly after `after`.
fn first_after(rrule: &str, dtstart: DateTime<Tz>, after: DateTime<Tz>) -> Result<Option<DateTime<Tz>>> {
    // rrule's `after` filter is inclusive; nudge by 1 s for "strictly after".
    let after = after + Duration::seconds(1);
    let tz: rrule::Tz = dtstart.timezone().into();
    let res = set(rrule, dtstart)?.after(after.with_timezone(&tz)).all(1);
    Ok(res.dates.into_iter().next().map(|d| d.with_timezone(&dtstart.timezone())))
}

/// Next due after completing an occurrence.
///
/// * fixed: next scheduled occurrence after both the current due and now
///   (missed occurrences are skipped, like rent or bins).
/// * adaptive: counted from completion, keeping the time of day
///   (plants, haircut).
pub fn next_due(rrule: &str, tz: Tz, due_at: i64, adaptive: bool, now: i64) -> Result<Option<i64>> {
    let due = from_ms(due_at, tz);
    let now_tz = from_ms(now, tz);
    let next = if adaptive {
        let time: NaiveTime = due.time();
        let start = tz
            .from_local_datetime(&now_tz.date_naive().and_time(time))
            .earliest()
            .unwrap_or(now_tz);
        first_after(rrule, start, start.max(now_tz))?
    } else {
        first_after(rrule, due, due.max(now_tz))?
    };
    Ok(next.map(to_utc_ms))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn berlin(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> i64 {
        to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap())
    }
    const B: Tz = chrono_tz::Europe::Berlin;

    #[test]
    fn fixed_weekly_moves_to_next_monday() {
        // Mon 2026-10-12 09:00, completed on time.
        let due = berlin(2026, 10, 12, 9, 0);
        let next = next_due("FREQ=WEEKLY;BYDAY=MO", B, due, false, due + 60_000).unwrap();
        assert_eq!(next, Some(berlin(2026, 10, 19, 9, 0)));
    }

    #[test]
    fn fixed_skips_missed_occurrences() {
        let due = berlin(2026, 10, 1, 9, 0);
        let now = berlin(2026, 10, 15, 12, 0);
        assert_eq!(next_due("FREQ=DAILY", B, due, false, now).unwrap(), Some(berlin(2026, 10, 16, 9, 0)));
    }

    #[test]
    fn keeps_wall_clock_time_across_dst() {
        // DST ends in Berlin on 2026-10-25.
        let due = berlin(2026, 10, 24, 9, 0);
        assert_eq!(next_due("FREQ=DAILY", B, due, false, due).unwrap(), Some(berlin(2026, 10, 25, 9, 0)));
        assert_eq!(
            berlin(2026, 10, 25, 9, 0) - due,
            25 * 3_600_000,
            "the UTC gap is 25 h, wall clock stays 09:00"
        );
    }

    #[test]
    fn adaptive_counts_from_completion() {
        // Due Oct 1 (every 3 days), completed late on Oct 15 at 20:00.
        let due = berlin(2026, 10, 1, 9, 0);
        let now = berlin(2026, 10, 15, 20, 0);
        assert_eq!(
            next_due("INTERVAL=3;FREQ=DAILY", B, due, true, now).unwrap(),
            Some(berlin(2026, 10, 18, 9, 0))
        );
    }

    #[test]
    fn monthly_end_of_month() {
        let due = berlin(2026, 10, 31, 18, 0);
        assert_eq!(
            next_due("FREQ=MONTHLY;BYMONTHDAY=-1", B, due, false, due).unwrap(),
            Some(berlin(2026, 11, 30, 18, 0))
        );
    }

    #[test]
    fn rejects_bad_rules() {
        assert!(validate("FREQ=DAILY").is_ok());
        assert!(validate("NOPE").is_err());
        assert!(validate("DTSTART:20260101T000000Z\nRRULE:FREQ=DAILY").is_err());
    }
}
