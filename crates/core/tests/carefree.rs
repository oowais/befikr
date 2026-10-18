use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";

/// October 2026, Berlin time.
fn at(d: u32, h: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, 0, 0).unwrap())
}

fn setup(start: i64) -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(start));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

fn add(s: &mut Store, title: &str, due: i64) -> String {
    s.add_task(NewTask { title: title.into(), due_at: Some(due), tz: Some(TZ.into()), ..Default::default() })
        .unwrap()
        .task
        .id
}

fn done_at(s: &mut Store, clock: &AtomicI64, id: &str, when: i64) {
    clock.store(when, Ordering::SeqCst);
    s.complete_task(id, Source::App).unwrap();
}

#[test]
fn empty_and_quiet_days_count() {
    let (mut s, clock) = setup(at(1, 8));
    assert_eq!(s.carefree(TZ).unwrap(), Carefree::default());
    add(&mut s, "later", at(30, 9));
    // Oct 1..=4 finished (5th not yet: it ends at noon on the 6th).
    clock.store(at(6, 11), Ordering::SeqCst);
    assert_eq!(s.carefree(TZ).unwrap().streak, 4);
    clock.store(at(6, 13), Ordering::SeqCst);
    assert_eq!(s.carefree(TZ).unwrap().streak, 5);
}

#[test]
fn done_by_noon_next_day_is_fine_later_is_missed() {
    let (mut s, clock) = setup(at(1, 8));
    let a = add(&mut s, "a", at(2, 20));
    done_at(&mut s, &clock, &a, at(3, 11)); // next morning: fine
    let b = add(&mut s, "b", at(4, 9));
    done_at(&mut s, &clock, &b, at(5, 15)); // next afternoon: the 4th is missed (forgiven)
    clock.store(at(8, 13), Ordering::SeqCst);
    let c = s.carefree(TZ).unwrap();
    // 1,2,3 count; 4 forgiven; 5,6,7 count.
    assert_eq!((c.streak, c.best), (6, 6));
    assert_eq!(c.on_time_total, 0, "both done a day late");
}

#[test]
fn second_miss_within_a_week_breaks_the_run() {
    let (mut s, clock) = setup(at(1, 8));
    let a = add(&mut s, "a", at(3, 9));
    done_at(&mut s, &clock, &a, at(4, 18)); // misses the 3rd (forgiven)
    let b = add(&mut s, "b", at(6, 9));
    done_at(&mut s, &clock, &b, at(7, 18)); // misses the 6th: breaks
    clock.store(at(10, 13), Ordering::SeqCst);
    let c = s.carefree(TZ).unwrap();
    assert_eq!(c.best, 4, "1, 2, (3), 4, 5");
    assert_eq!(c.streak, 3, "7, 8, 9");
}

#[test]
fn open_overdue_task_misses_every_day_until_done() {
    let (mut s, clock) = setup(at(1, 8));
    add(&mut s, "x", at(2, 9));
    clock.store(at(6, 13), Ordering::SeqCst);
    let c = s.carefree(TZ).unwrap();
    // 1 counts; 2 forgiven; 3 breaks; 4, 5 broken.
    assert_eq!((c.streak, c.best), (0, 1));
}

#[test]
fn moved_or_deleted_tasks_dont_count_and_on_time_counts() {
    let (mut s, clock) = setup(at(12, 8)); // Monday the 12th
    let moved = add(&mut s, "moved", at(12, 9));
    let gone = add(&mut s, "gone", at(12, 9));
    clock.store(at(13, 15), Ordering::SeqCst);
    s.update_task(&moved, TaskPatch { due_at: Some(Some(at(20, 9))), ..Default::default() }).unwrap();
    s.delete_task(&gone).unwrap();
    let ok = add(&mut s, "ok", at(13, 18));
    done_at(&mut s, &clock, &ok, at(13, 16));
    clock.store(at(15, 13), Ordering::SeqCst);
    let c = s.carefree(TZ).unwrap();
    assert_eq!(c.streak, 3, "12th to 14th");
    assert_eq!((c.on_time_total, c.on_time_week), (1, 1));
}

fn daily(s: &mut Store, title: &str, first: i64) -> String {
    s.add_task(NewTask {
        title: title.into(),
        due_at: Some(first),
        tz: Some(TZ.into()),
        rrule: Some("FREQ=DAILY".into()),
        ..Default::default()
    })
    .unwrap()
    .task
    .id
}

#[test]
fn habit_streak_counts_on_time_skips_are_neutral_late_resets() {
    let (mut s, clock) = setup(at(1, 8));
    let one_off = add(&mut s, "once", at(2, 9));
    assert_eq!(s.habit_streak(&one_off, TZ).unwrap(), None);

    let gym = daily(&mut s, "gym", at(1, 18));
    done_at(&mut s, &clock, &gym, at(1, 19)); // 1st on time
    done_at(&mut s, &clock, &gym, at(3, 10)); // 2nd: next morning, fine
    clock.store(at(3, 12), Ordering::SeqCst);
    s.skip_task(&gym, Source::App).unwrap(); // 3rd skipped
    done_at(&mut s, &clock, &gym, at(4, 18)); // 4th
    assert_eq!(s.habit_streak(&gym, TZ).unwrap(), Some(HabitStreak { current: 3, best: 3 }));

    // 5th done on the 7th: late (5th) and the 6th passed meanwhile.
    done_at(&mut s, &clock, &gym, at(7, 20));
    assert_eq!(s.habit_streak(&gym, TZ).unwrap(), Some(HabitStreak { current: 0, best: 3 }));
    // Now due the 8th; on time again.
    done_at(&mut s, &clock, &gym, at(8, 19));
    assert_eq!(s.habit_streak(&gym, TZ).unwrap().unwrap().current, 1);
    // The 9th goes by undone: broken once its grace (noon on the 10th) passes.
    clock.store(at(10, 11), Ordering::SeqCst);
    assert_eq!(s.habit_streak(&gym, TZ).unwrap().unwrap().current, 1);
    clock.store(at(10, 13), Ordering::SeqCst);
    assert_eq!(s.habit_streak(&gym, TZ).unwrap().unwrap().current, 0);
}

#[test]
fn completion_marked_not_done_counts_as_missed_until_marked_done_again() {
    let (mut s, clock) = setup(at(1, 8));
    let gym = daily(&mut s, "gym", at(1, 18));
    for d in 1..=5 {
        done_at(&mut s, &clock, &gym, at(d, 19));
    }
    clock.store(at(6, 13), Ordering::SeqCst);
    let due = s.get_task(&gym).unwrap().due_at;
    assert_eq!(s.habit_streak(&gym, TZ).unwrap().unwrap().current, 5);
    let history = s.task_history(&gym).unwrap();
    let third = history.iter().find(|c| c.due_at_then == Some(at(3, 18))).unwrap().id.clone();

    s.set_completion_done(&third, false).unwrap();
    let h = s.task_history(&gym).unwrap();
    assert_eq!((h.len(), h.iter().filter(|c| c.not_done).count()), (5, 1), "kept, shown as not done");
    assert_eq!(s.habit_streak(&gym, TZ).unwrap(), Some(HabitStreak { current: 2, best: 2 }));
    assert_eq!(s.get_task(&gym).unwrap().due_at, due, "next due stays");
    let st = s.stats(None, &[]).unwrap();
    assert_eq!((st.completed, st.missed), (4, 1));
    assert_eq!(s.completed_groups().unwrap()[0].count, 4);
    assert_eq!(s.carefree(TZ).unwrap().streak, 4, "1, 2, (3 forgiven), 4, 5");
    assert_eq!(s.carefree(TZ).unwrap().on_time_total, 4);

    // A second one breaks the carefree run.
    let second = h.iter().find(|c| c.due_at_then == Some(at(2, 18))).unwrap().id.clone();
    s.set_completion_done(&second, false).unwrap();
    assert_eq!(s.carefree(TZ).unwrap().streak, 2);

    // Both back: as before.
    s.set_completion_done(&second, true).unwrap();
    s.set_completion_done(&third, true).unwrap();
    s.set_completion_done(&third, true).unwrap(); // already done: no-op
    assert!(s.task_history(&gym).unwrap().iter().all(|c| !c.not_done));
    assert_eq!(s.habit_streak(&gym, TZ).unwrap().unwrap().current, 5);
    assert_eq!(s.stats(None, &[]).unwrap().missed, 0);
    assert!(s.set_completion_done("nope", false).is_err());
}
