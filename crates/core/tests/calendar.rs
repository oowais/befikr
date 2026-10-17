use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

/// Thu 2026-10-15 10:00 Berlin; the week shown is Mon 12 – Mon 19.
fn store() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(at(15, 10, 0)));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

fn add(s: &mut Store, title: &str, due: Option<i64>, rrule: Option<&str>) -> String {
    let t = NewTask {
        title: title.into(),
        due_at: due,
        tz: due.map(|_| TZ.into()),
        rrule: rrule.map(Into::into),
        ..Default::default()
    };
    s.add_task(t).unwrap().task.id
}

fn week(s: &Store) -> Vec<(String, i64, bool)> {
    s.calendar(at(12, 0, 0), at(19, 0, 0), TZ).unwrap().into_iter().map(|i| (i.task.title, i.at, i.upcoming)).collect()
}

#[test]
fn one_offs_in_range_only() {
    let (mut s, _) = store();
    add(&mut s, "undated", None, None);
    add(&mut s, "late", Some(at(14, 9, 0)), None);
    add(&mut s, "fri", Some(at(16, 18, 0)), None);
    add(&mut s, "next week", Some(at(20, 9, 0)), None);
    assert_eq!(week(&s), [("late".into(), at(14, 9, 0), false), ("fri".into(), at(16, 18, 0), false)]);
}

#[test]
fn repeats_show_what_is_still_to_come() {
    let (mut s, _) = store();
    add(&mut s, "vitamins", Some(at(15, 8, 0)), Some("FREQ=DAILY"));
    let w = week(&s);
    // Today's (overdue) occurrence is the task; Fri–Sun are upcoming.
    assert_eq!(w[0], ("vitamins".into(), at(15, 8, 0), false));
    let later: Vec<i64> = w[1..].iter().map(|x| x.1).collect();
    assert_eq!(later, [at(16, 8, 0), at(17, 8, 0), at(18, 8, 0)]);
    assert!(w[1..].iter().all(|x| x.2));
}

#[test]
fn overdue_repeat_does_not_show_missed_days() {
    let (mut s, _) = store();
    add(&mut s, "plants", Some(at(12, 9, 0)), Some("FREQ=DAILY"));
    let days: Vec<i64> = week(&s).into_iter().map(|x| x.1).collect();
    assert_eq!(days, [at(12, 9, 0), at(16, 9, 0), at(17, 9, 0), at(18, 9, 0)], "13–15 were missed");
}

#[test]
fn moving_one_occurrence_keeps_the_series() {
    let (mut s, _) = store();
    let id = add(&mut s, "gym", Some(at(15, 18, 0)), Some("FREQ=WEEKLY;BYDAY=TU,TH"));
    s.snooze_task(&id, at(16, 18, 0), Source::App).unwrap();
    let w = week(&s);
    assert_eq!(w, [("gym".into(), at(16, 18, 0), false)], "Thu moved to Fri; next is Tue 20");
    let next = s.calendar(at(19, 0, 0), at(26, 0, 0), TZ).unwrap();
    assert_eq!(next.iter().map(|i| i.at).collect::<Vec<_>>(), [at(20, 18, 0), at(22, 18, 0)]);
}
