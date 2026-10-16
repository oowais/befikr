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
