use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";
const MIN: i64 = 60_000;

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

/// Store with a controllable clock, starting Thu 2026-10-15 10:00 Berlin.
fn store() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(at(15, 10, 0)));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

fn task(title: &str, due: Option<i64>) -> NewTask {
    NewTask { title: title.into(), due_at: due, tz: due.map(|_| TZ.into()), ..Default::default() }
}

fn titles(s: &Store) -> Vec<String> {
    s.list_open().unwrap().into_iter().map(|i| i.task.title).collect()
}

#[test]
fn list_order_overdue_upcoming_undated() {
    let (mut s, _) = store();
    s.add_task(task("undated", None)).unwrap();
    s.add_task(task("tomorrow", Some(at(16, 9, 0)))).unwrap();
    s.add_task(task("overdue new", Some(at(15, 9, 0)))).unwrap();
    s.add_task(task("overdue old", Some(at(14, 9, 0)))).unwrap();
    s.add_task(task("later today", Some(at(15, 18, 0)))).unwrap();
    s.add_task(task("same time, created second", Some(at(15, 18, 0)))).unwrap();
    assert_eq!(
        titles(&s),
        ["overdue old", "overdue new", "later today", "same time, created second", "tomorrow", "undated"]
    );
}

#[test]
fn ids_are_uuid_v7_and_rows_have_timestamps() {
    let (mut s, _) = store();
    let t = s.add_task(task("x", None)).unwrap().task;
    assert_eq!(uuid::Uuid::parse_str(&t.id).unwrap().get_version_num(), 7);
    assert_eq!((t.created_at, t.updated_at), (at(15, 10, 0), at(15, 10, 0)));
}

#[test]
fn add_returns_alarm_change() {
    let (mut s, _) = store();
    let c = s.add_task(task("x", Some(at(15, 18, 0)))).unwrap();
    assert_eq!(c.alarm, AlarmChange::Schedule(Alarm { task_id: c.task.id.clone(), title: "x".into(), at_ms: at(15, 18, 0) }));
    let c = s.add_task(task("y", None)).unwrap();
    assert_eq!(c.alarm, AlarmChange::Cancel { task_id: c.task.id });
}

#[test]
fn validation() {
    let (mut s, _) = store();
    assert!(s.add_task(task("  ", None)).is_err());
    let mut t = task("r", None);
    t.rrule = Some("FREQ=DAILY".into());
    assert!(s.add_task(t).is_err(), "recurring needs due + tz");
    let mut t = task("r", Some(at(15, 9, 0)));
    t.rrule = Some("NOPE".into());
    assert!(s.add_task(t).is_err());
    let mut t = task("r", Some(at(15, 9, 0)));
    t.tz = Some("Mars/Base".into());
    assert!(s.add_task(t).is_err());
}

#[test]
fn complete_and_undo() {
    let (mut s, clock) = store();
    let id = s.add_task(task("x", Some(at(15, 18, 0)))).unwrap().task.id;
    clock.store(at(15, 11, 0), Ordering::SeqCst);
    let c = s.complete_task(&id, Source::App).unwrap();
    assert_eq!(c.task.completed_at, Some(at(15, 11, 0)));
    assert_eq!(c.alarm, AlarmChange::Cancel { task_id: id.clone() });
    assert!(titles(&s).is_empty());
    assert_eq!(s.list_completed(10).unwrap()[0].due_at_then, Some(at(15, 18, 0)));
    // Double complete (notification + app) is harmless.
    s.complete_task(&id, Source::Native).unwrap();
    assert_eq!(s.list_completed(10).unwrap().len(), 1);

    let c = s.reopen_task(&id).unwrap();
    assert_eq!(c.task.completed_at, None);
    assert!(matches!(c.alarm, AlarmChange::Schedule(_)));
    assert!(s.list_completed(10).unwrap().is_empty());
}

#[test]
fn subtasks_follow_parent() {
    let (mut s, clock) = store();
    let p = s.add_task(task("parent", Some(at(15, 18, 0)))).unwrap().task.id;
    let sub = |title: &str| NewTask { title: title.into(), parent_id: Some(p.clone()), ..Default::default() };
    let a = s.add_task(sub("a")).unwrap().task.id;
    s.add_task(sub("b")).unwrap();
    s.add_task(sub("c")).unwrap();
    // Subtasks: no dates, no alarms, one level only.
    let mut dated = sub("d");
    dated.due_at = Some(at(15, 9, 0));
    assert!(s.add_task(dated).is_err());
    assert!(s.add_task(NewTask { title: "deep".into(), parent_id: Some(a.clone()), ..Default::default() }).is_err());
    assert_eq!(s.complete_task(&a, Source::App).unwrap().alarm, AlarmChange::Cancel { task_id: a.clone() });

    let row = &s.list_open().unwrap()[0];
    assert_eq!((row.subtasks_done, row.subtasks_total), (1, 3));

    // Completing the parent checks the rest; undo unchecks only those
    // (matched by the parent's completion timestamp).
    clock.fetch_add(MIN, Ordering::SeqCst);
    s.complete_task(&p, Source::App).unwrap();
    assert!(s.list_subtasks(&p).unwrap().iter().all(|t| t.completed_at.is_some()));
    s.reopen_task(&p).unwrap();
    let open: Vec<_> = s.list_subtasks(&p).unwrap().into_iter().filter(|t| t.completed_at.is_none()).map(|t| t.title).collect();
    assert_eq!(open, ["b", "c"]);
}

#[test]
fn recurring_fixed_advances_and_resets_subtasks() {
    let (mut s, clock) = store();
    let mut t = task("bins", Some(at(19, 18, 0))); // Mon
    t.rrule = Some("FREQ=WEEKLY;BYDAY=MO".into());
    let id = s.add_task(t).unwrap().task.id;
    let sub = s.add_task(NewTask { title: "sort".into(), parent_id: Some(id.clone()), ..Default::default() }).unwrap().task.id;
    s.complete_task(&sub, Source::App).unwrap();

    clock.store(at(19, 18, 5), Ordering::SeqCst);
    let c = s.complete_task(&id, Source::Native).unwrap();
    assert_eq!(c.task.completed_at, None, "recurring tasks stay open");
    assert_eq!(c.task.due_at, Some(at(26, 18, 0)));
    assert_eq!(c.alarm, AlarmChange::Schedule(Alarm { task_id: id.clone(), title: "bins".into(), at_ms: at(26, 18, 0) }));
    assert_eq!(s.get_task(&sub).unwrap().completed_at, None, "subtasks reset");
    assert_eq!(s.list_completed(10).unwrap()[0].due_at_then, Some(at(19, 18, 0)));

    // Undo restores the occurrence.
    assert_eq!(s.reopen_task(&id).unwrap().task.due_at, Some(at(19, 18, 0)));
}

#[test]
fn recurring_adaptive_counts_from_completion() {
    let (mut s, clock) = store();
    let mut t = task("water plants", Some(at(10, 9, 0)));
    t.rrule = Some("INTERVAL=3;FREQ=DAILY".into());
    t.adaptive = true;
    let id = s.add_task(t).unwrap().task.id;
    clock.store(at(15, 20, 0), Ordering::SeqCst);
    assert_eq!(s.complete_task(&id, Source::App).unwrap().task.due_at, Some(at(18, 9, 0)));
}

#[test]
fn snooze_moves_task_and_alarm() {
    let (mut s, _) = store();
    let a = s.add_task(task("overdue", Some(at(15, 8, 0)))).unwrap().task.id;
    s.add_task(task("noon", Some(at(15, 12, 0)))).unwrap();
    let c = s.snooze_task(&a, at(15, 21, 0), Source::App).unwrap();
    assert_eq!(c.task.due_at, Some(at(15, 8, 0)), "original due kept for stats");
    assert_eq!(c.task.effective_due(), Some(at(15, 21, 0)));
    assert!(matches!(c.alarm, AlarmChange::Schedule(Alarm { at_ms, .. }) if at_ms == at(15, 21, 0)));
    assert_eq!(titles(&s), ["noon", "overdue"]);
    // Editing the date clears the snooze.
    let patch = TaskPatch { due_at: Some(Some(at(16, 9, 0))), ..Default::default() };
    assert_eq!(s.update_task(&a, patch).unwrap().task.snoozed_until, None);
}

#[test]
fn rereminds_escalate_until_acted_on() {
    let (mut s, clock) = store();
    let id = s.add_task(task("x", Some(at(15, 10, 0)))).unwrap().task.id;
    let mut gaps = vec![];
    for _ in 0..6 {
        let now = s.now();
        let o = s.on_fired(&id, Source::Native).unwrap();
        assert!(o.show);
        gaps.push((o.next_at_ms.unwrap() - now) / MIN);
        clock.store(o.next_at_ms.unwrap(), Ordering::SeqCst);
    }
    assert_eq!(gaps, [5, 15, 30, 60, 60, 60]);

    // A title edit doesn't restart escalation; completion stops it.
    s.update_task(&id, TaskPatch { title: Some("y".into()), ..Default::default() }).unwrap();
    assert_eq!(s.pending_alarms().unwrap()[0].title, "y");
    s.complete_task(&id, Source::Native).unwrap();
    assert_eq!(s.on_fired(&id, Source::Native).unwrap(), FireOutcome { show: false, next_at_ms: None });
    assert!(s.pending_alarms().unwrap().is_empty());
}

#[test]
fn custom_reremind_intervals() {
    let (mut s, _) = store();
    s.set_reremind_minutes(vec![1, 2]).unwrap();
    assert!(s.set_reremind_minutes(vec![]).is_err());
    let id = s.add_task(task("x", Some(at(15, 10, 0)))).unwrap().task.id;
    let first = s.on_fired(&id, Source::Native).unwrap().next_at_ms.unwrap();
    assert_eq!(first - s.now(), MIN);
}

#[test]
fn pending_alarms_catch_up_missed() {
    let (mut s, _) = store();
    s.add_task(task("missed", Some(at(15, 8, 0)))).unwrap();
    s.add_task(task("later", Some(at(15, 18, 0)))).unwrap();
    s.add_task(task("undated", None)).unwrap();
    let p = s.pending_alarms().unwrap();
    assert_eq!(p.iter().map(|a| a.at_ms).collect::<Vec<_>>(), [at(15, 10, 0) + 5_000, at(15, 18, 0)]);
}

#[test]
fn soft_delete() {
    let (mut s, _) = store();
    let id = s.add_task(task("x", Some(at(15, 18, 0)))).unwrap().task.id;
    s.add_task(NewTask { title: "sub".into(), parent_id: Some(id.clone()), ..Default::default() }).unwrap();
    let c = s.delete_task(&id).unwrap();
    assert!(c.task.deleted_at.is_some());
    assert_eq!(c.alarm, AlarmChange::Cancel { task_id: id.clone() });
    assert!(titles(&s).is_empty());
    assert!(s.get_task(&id).is_err());
    assert!(s.list_subtasks(&id).unwrap().is_empty());
}

#[test]
fn settings_and_time_words() {
    let (mut s, _) = store();
    let st = s.settings().unwrap();
    assert_eq!(st.reremind_minutes, [5, 15, 30, 60]);
    assert_eq!(st.time_words["evening"], "18:00");
    s.set_time_word("Dawn", "05:30").unwrap();
    s.set_time_word("evening", "19:00").unwrap();
    assert!(s.set_time_word("x", "25:00").is_err());
    s.remove_time_word("noon").unwrap();
    s.set_backup_dir(Some("/sdcard/Sync".into())).unwrap();
    let st = s.settings().unwrap();
    assert_eq!(st.time_words.get("dawn").map(String::as_str), Some("05:30"));
    assert_eq!(st.time_words["evening"], "19:00");
    assert!(!st.time_words.contains_key("noon"));
    assert_eq!(st.backup_dir.as_deref(), Some("/sdcard/Sync"));
}

#[test]
fn export_snapshot_is_openable() {
    let (mut s, _) = store();
    s.add_task(task("keep me", None)).unwrap();
    let dir = std::env::temp_dir().join(format!("befikr-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("snap.db");
    s.export_to(path.to_str().unwrap()).unwrap();
    let copy = Store::open(path.to_str().unwrap()).unwrap();
    assert_eq!(titles(&copy), ["keep me"]);
    std::fs::remove_dir_all(dir).unwrap();
}
