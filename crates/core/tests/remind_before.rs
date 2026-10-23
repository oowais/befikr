use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

/// Clock starts Thu 2026-10-15 10:00 Berlin.
fn store() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(at(15, 10, 0)));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

fn task(due: i64, before: &[u32]) -> NewTask {
    NewTask {
        title: "dentist".into(),
        due_at: Some(due),
        tz: Some(TZ.into()),
        remind_before: before.to_vec(),
        ..Default::default()
    }
}

fn armed(c: &Change) -> i64 {
    match &c.alarm {
        AlarmChange::Schedule(a) => a.at_ms,
        other => panic!("not scheduled: {other:?}"),
    }
}

#[test]
fn early_reminders_fire_in_turn_then_the_due_one() {
    let (mut s, clock) = store();
    let c = s.add_task(task(at(16, 18, 0), &[30, 1440])).unwrap();
    assert_eq!(c.task.remind_before, [1440, 30], "largest first");
    assert_eq!(armed(&c), at(15, 18, 0), "a day before");

    clock.store(at(15, 18, 0), Ordering::SeqCst);
    let o = s.on_fired(&c.task.id, Source::Native).unwrap();
    assert_eq!(o, FireOutcome { show: true, next_at_ms: Some(at(16, 17, 30)), repeats: false, due_at_ms: Some(at(16, 18, 0)) });

    clock.store(at(16, 17, 30), Ordering::SeqCst);
    let o = s.on_fired(&c.task.id, Source::Native).unwrap();
    assert_eq!(o.next_at_ms, Some(at(16, 18, 0)));
    assert_eq!(o.due_at_ms, Some(at(16, 18, 0)));

    clock.store(at(16, 18, 0), Ordering::SeqCst);
    let o = s.on_fired(&c.task.id, Source::Native).unwrap();
    assert_eq!(o, FireOutcome { show: true, next_at_ms: None, repeats: false, due_at_ms: None }, "the due reminder");
}

#[test]
fn reminders_already_past_are_left_out() {
    let (mut s, _) = store();
    // Due in 20 min: "30 min before" has passed, "10 min before" hasn't.
    let c = s.add_task(task(at(15, 10, 20), &[30, 10])).unwrap();
    assert_eq!(armed(&c), at(15, 10, 10));
    // Due in 20 min with only "1 hour before": the due time.
    let c = s.add_task(task(at(15, 10, 20), &[60])).unwrap();
    assert_eq!(armed(&c), at(15, 10, 20));
}

#[test]
fn a_late_early_alarm_past_the_due_time_is_the_due_reminder() {
    let (mut s, clock) = store();
    let c = s.add_task(task(at(15, 18, 0), &[30])).unwrap();
    // Phone off until after the due time: the catch-up alarm is the real one.
    clock.store(at(15, 18, 5), Ordering::SeqCst);
    let o = s.on_fired(&c.task.id, Source::Native).unwrap();
    assert!(o.show);
    assert_eq!((o.next_at_ms, o.due_at_ms), (None, None));
}

#[test]
fn snoozed_reminds_at_the_snooze_time_only() {
    let (mut s, clock) = store();
    let id = s.add_task(task(at(15, 12, 0), &[30])).unwrap().task.id;
    let c = s.snooze_task(&id, at(15, 20, 0), Source::App).unwrap();
    assert_eq!(armed(&c), at(15, 20, 0));
    clock.store(at(15, 20, 0), Ordering::SeqCst);
    assert_eq!(s.on_fired(&id, Source::Native).unwrap().due_at_ms, None);
}

#[test]
fn editing_reminders_rearms_and_keeps_a_snooze() {
    let (mut s, _) = store();
    let id = s.add_task(task(at(16, 18, 0), &[])).unwrap().task.id;
    let c = s.update_task(&id, TaskPatch { remind_before: Some(vec![60]), ..Default::default() }).unwrap();
    assert_eq!(armed(&c), at(16, 17, 0));
    s.snooze_task(&id, at(17, 9, 0), Source::App).unwrap();
    let c = s.update_task(&id, TaskPatch { remind_before: Some(vec![120]), ..Default::default() }).unwrap();
    assert_eq!(c.task.snoozed_until, Some(at(17, 9, 0)), "snooze kept");
    assert_eq!(armed(&c), at(17, 9, 0));
    let c = s.update_task(&id, TaskPatch { remind_before: Some(vec![]), ..Default::default() }).unwrap();
    assert!(c.task.remind_before.is_empty());
}

#[test]
fn repeating_task_reminds_early_for_each_occurrence() {
    let (mut s, clock) = store();
    let mut t = task(at(15, 18, 0), &[60]);
    t.rrule = Some("FREQ=DAILY".into());
    let id = s.add_task(t).unwrap().task.id;
    clock.store(at(15, 18, 30), Ordering::SeqCst);
    let c = s.complete_task(&id, Source::App).unwrap();
    assert_eq!(c.task.due_at, Some(at(16, 18, 0)));
    assert_eq!(armed(&c), at(16, 17, 0));
}

#[test]
fn limits() {
    let (mut s, _) = store();
    assert!(s.add_task(task(at(16, 18, 0), &[1, 2, 3, 4, 5])).is_err(), "at most 4");
    assert!(s.add_task(task(at(16, 18, 0), &[5 * 7 * 24 * 60])).is_err(), "at most 4 weeks");
    let c = s.add_task(task(at(16, 18, 0), &[30, 30, 0])).unwrap();
    assert_eq!(c.task.remind_before, [30], "zero and doubles dropped");
    let parent = c.task.id;
    let sub = NewTask { title: "sub".into(), parent_id: Some(parent), remind_before: vec![30], ..Default::default() };
    assert!(s.add_task(sub).is_err(), "subtasks follow the parent");
}

#[test]
fn survives_a_restart_and_syncs() {
    let dir = std::env::temp_dir().join(format!("befikr-remind-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("a.db");
    let path = path.to_str().unwrap();
    let id = {
        let mut s = Store::open(path).unwrap();
        s.add_task(task(at(30, 18, 0), &[45])).unwrap().task.id
    };
    let s = Store::open(path).unwrap();
    assert_eq!(s.get_task(&id).unwrap().remind_before, [45]);
    let mut other = Store::open_in_memory().unwrap();
    other.merge(&s.changes_since(0, 1000, None).unwrap()).unwrap();
    assert_eq!(other.get_task(&id).unwrap().remind_before, [45]);
    drop(s);
    let _ = std::fs::remove_dir_all(&dir);
}
