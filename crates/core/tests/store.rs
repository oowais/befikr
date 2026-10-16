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
    assert_eq!(s.on_fired(&id, Source::Native).unwrap(), FireOutcome { show: false, next_at_ms: None, repeats: false });
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
fn late_fixed_completion_records_missed() {
    let (mut s, clock) = store(); // Oct 15 10:00
    let mut t = task("pills", Some(at(10, 9, 0)));
    t.rrule = Some("FREQ=DAILY".into());
    let id = s.add_task(t).unwrap().task.id;

    clock.store(at(15, 8, 0), Ordering::SeqCst);
    let c = s.complete_task(&id, Source::App).unwrap();
    assert_eq!(c.task.due_at, Some(at(16, 9, 0)), "not again at 09:00 today");
    let days: Vec<i64> = (11..=14).map(|d| at(d, 9, 0)).collect();
    assert_eq!(s.missed_occurrences(&id).unwrap(), days);

    // Undo drops them from the count.
    clock.store(at(15, 8, 1), Ordering::SeqCst);
    s.reopen_task(&id).unwrap();
    assert!(s.missed_occurrences(&id).unwrap().is_empty());
}

#[test]
fn restore_undoes_delete() {
    let (mut s, clock) = store();
    let id = s.add_task(task("x", Some(at(15, 18, 0)))).unwrap().task.id;
    let old = s.add_task(NewTask { title: "old sub".into(), parent_id: Some(id.clone()), ..Default::default() }).unwrap();
    s.add_task(NewTask { title: "sub".into(), parent_id: Some(id.clone()), ..Default::default() }).unwrap();
    s.delete_task(&old.task.id).unwrap();
    clock.fetch_add(MIN, Ordering::SeqCst);
    s.delete_task(&id).unwrap();
    let c = s.restore_task(&id).unwrap();
    assert!(c.task.deleted_at.is_none());
    assert!(matches!(c.alarm, AlarmChange::Schedule(_)));
    assert_eq!(titles(&s), ["x"]);
    // Only the subtasks deleted with it come back.
    let subs: Vec<_> = s.list_subtasks(&id).unwrap().into_iter().map(|t| t.title).collect();
    assert_eq!(subs, ["sub"]);
    assert!(s.restore_task(&id).is_err(), "not deleted any more");
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

#[test]
fn import_restores_snapshot_into_live_store() {
    let (mut s, _) = store();
    s.add_task(task("before backup", Some(at(16, 9, 0)))).unwrap();
    let dir = std::env::temp_dir().join(format!("befikr-test-{}", uuid::Uuid::now_v7()));
    std::fs::create_dir_all(&dir).unwrap();
    let snap = dir.join("snap.db");
    let snap = snap.to_str().unwrap();
    s.export_to(snap).unwrap();
    s.export_to(snap).unwrap(); // overwrites

    s.add_task(task("after backup", None)).unwrap();
    s.import_from(snap).unwrap();
    assert_eq!(titles(&s), ["before backup"]);
    assert_eq!(s.pending_alarms().unwrap().len(), 1, "alarms come from the restored data");
    // Still writable afterwards.
    s.add_task(task("new", None)).unwrap();

    let junk = dir.join("junk.db");
    std::fs::write(&junk, b"not sqlite").unwrap();
    assert!(s.import_from(junk.to_str().unwrap()).is_err());
    let other = dir.join("other.db");
    rusqlite::Connection::open(&other).unwrap().execute_batch("CREATE TABLE x (y)").unwrap();
    assert!(s.import_from(other.to_str().unwrap()).is_err());
    assert_eq!(titles(&s).len(), 2, "failed imports change nothing");
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn stats_count_lateness_snoozes_and_missed() {
    let (mut s, clock) = store(); // Thu Oct 15 10:00
    clock.store(at(6, 12, 0), Ordering::SeqCst);
    let old = s.add_task(task("old", None)).unwrap().task.id;
    s.complete_task(&old, Source::App).unwrap(); // before `since`, still in week 1

    clock.store(at(15, 10, 0), Ordering::SeqCst);
    let early = s.add_task(task("early", Some(at(15, 18, 0)))).unwrap().task.id;
    let late = s.add_task(task("late", Some(at(14, 9, 0)))).unwrap().task.id;
    let undated = s.add_task(task("undated", None)).unwrap().task.id;
    let gone = s.add_task(task("gone", None)).unwrap().task.id;
    let mut t = task("pills", Some(at(10, 9, 0)));
    t.rrule = Some("FREQ=DAILY".into());
    let pills = s.add_task(t).unwrap().task.id;
    s.snooze_task(&late, at(15, 11, 0), Source::App).unwrap();
    s.snooze_task(&late, at(15, 11, 30), Source::App).unwrap();

    clock.store(at(15, 12, 0), Ordering::SeqCst);
    for id in [&early, &late, &undated, &gone, &pills] {
        s.complete_task(id, Source::App).unwrap();
    }
    s.delete_task(&gone).unwrap();

    let st = s.stats(Some(at(10, 0, 0)), &[at(5, 0, 0), at(12, 0, 0)]).unwrap();
    assert_eq!((st.completed, st.on_time, st.late), (4, 1, 2));
    assert_eq!(st.late_median_ms, Some(at(15, 12, 0) - at(10, 9, 0)), "upper median of 27h and 5d3h");
    assert_eq!((st.snoozes, st.snoozed_tasks), (2, 1));
    assert_eq!(st.missed, 5, "Oct 11–15 09:00");
    assert_eq!(st.weeks[0], WeekStats { start: at(5, 0, 0), on_time: 0, late: 0, undated: 1 });
    assert_eq!(st.weeks[1], WeekStats { start: at(12, 0, 0), on_time: 1, late: 2, undated: 1 });

    // Undo takes a completion and its missed repeats back out.
    s.reopen_task(&pills).unwrap();
    let st = s.stats(None, &[]).unwrap();
    assert_eq!((st.completed, st.late, st.missed), (4, 1, 0));
}

#[test]
fn skip_moves_repeat_on_without_completing() {
    let (mut s, _) = store(); // Thu Oct 15 10:00
    let mut t = task("gym", Some(at(15, 9, 0)));
    t.rrule = Some("FREQ=DAILY".into());
    t.adaptive = true;
    let id = s.add_task(t).unwrap().task.id;
    s.snooze_task(&id, at(15, 11, 0), Source::App).unwrap();

    let c = s.skip_task(&id, Source::Native).unwrap();
    assert_eq!(c.task.due_at, Some(at(16, 9, 0)));
    assert_eq!((c.task.snoozed_until, c.task.completed_at), (None, None));
    assert_eq!(c.alarm, AlarmChange::Schedule(Alarm { task_id: id.clone(), title: "gym".into(), at_ms: at(16, 9, 0) }));
    assert!(s.list_completed(10).unwrap().is_empty());

    // A future occurrence skips to the one after it, even "from when done".
    let c = s.skip_task(&id, Source::App).unwrap();
    assert_eq!(c.task.due_at, Some(at(17, 9, 0)));

    let plain = s.add_task(task("once", Some(at(15, 18, 0)))).unwrap().task.id;
    assert!(s.skip_task(&plain, Source::App).is_err());
}

#[test]
fn late_means_a_later_day() {
    let (mut s, clock) = store(); // Thu Oct 15 10:00 Berlin
    let same_day = s.add_task(task("rent", Some(at(15, 9, 0)))).unwrap().task.id;
    let next_day = s.add_task(task("call", Some(at(14, 23, 30)))).unwrap().task.id;
    clock.store(at(15, 23, 59), Ordering::SeqCst);
    s.complete_task(&same_day, Source::App).unwrap(); // 15 h after 09:00, same day
    s.complete_task(&next_day, Source::App).unwrap(); // 30 min past midnight
    let st = s.stats(None, &[]).unwrap();
    assert_eq!((st.on_time, st.late), (1, 1));
}
