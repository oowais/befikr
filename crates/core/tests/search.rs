use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

fn store() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(at(15, 10, 0)));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

fn add(s: &mut Store, title: &str, due: Option<i64>, notes: &str) -> String {
    let t = NewTask {
        title: title.into(),
        description: notes.into(),
        due_at: due,
        tz: due.map(|_| TZ.into()),
        ..Default::default()
    };
    s.add_task(t).unwrap().task.id
}

fn found(s: &Store, q: &str) -> Vec<String> {
    s.search(q).unwrap().into_iter().map(|h| h.task.title).collect()
}

#[test]
fn matches_title_notes_and_subtasks_case_insensitive() {
    let (mut s, _) = store();
    add(&mut s, "Buy Milk", None, "");
    add(&mut s, "Call bank", None, "ask about the milk subscription");
    let p = add(&mut s, "Party", None, "");
    s.add_task(NewTask { title: "oat milk".into(), parent_id: Some(p), ..Default::default() }).unwrap();
    add(&mut s, "Gym", None, "");
    assert_eq!(found(&s, "MILK"), ["Buy Milk", "Call bank", "Party"]);
}

#[test]
fn every_word_must_appear() {
    let (mut s, _) = store();
    add(&mut s, "pay rent", None, "");
    add(&mut s, "pay phone bill", None, "");
    assert_eq!(found(&s, "pay bill"), ["pay phone bill"]);
    assert_eq!(found(&s, "  "), Vec::<String>::new());
}

#[test]
fn wildcards_are_literal() {
    let (mut s, _) = store();
    add(&mut s, "50% off", None, "");
    add(&mut s, "500 off", None, "");
    add(&mut s, "file_name", None, "");
    add(&mut s, "filename", None, "");
    assert_eq!(found(&s, "50%"), ["50% off"]);
    assert_eq!(found(&s, "e_n"), ["file_name"]);
}

#[test]
fn open_first_in_list_order_then_done_newest_first_deleted_never() {
    let (mut s, clock) = store();
    let a = add(&mut s, "tax a", None, "");
    add(&mut s, "tax later", Some(at(20, 9, 0)), "");
    add(&mut s, "tax soon", Some(at(16, 9, 0)), "");
    let b = add(&mut s, "tax b", None, "");
    let gone = add(&mut s, "tax gone", None, "");
    s.complete_task(&a, Source::App).unwrap();
    clock.store(at(15, 11, 0), Ordering::SeqCst);
    s.complete_task(&b, Source::App).unwrap();
    s.delete_task(&gone).unwrap();
    assert_eq!(found(&s, "tax"), ["tax soon", "tax later", "tax b", "tax a"]);
    let hits = s.search("tax b").unwrap();
    assert_eq!(hits[0].last_done_at, Some(at(15, 11, 0)));
}

#[test]
fn repeating_task_reports_its_last_completion() {
    let (mut s, clock) = store();
    let id = s
        .add_task(NewTask {
            title: "water plants".into(),
            due_at: Some(at(15, 9, 0)),
            tz: Some(TZ.into()),
            rrule: Some("FREQ=DAILY".into()),
            ..Default::default()
        })
        .unwrap()
        .task
        .id;
    clock.store(at(15, 12, 0), Ordering::SeqCst);
    s.complete_task(&id, Source::App).unwrap();
    let hit = &s.search("plants").unwrap()[0];
    assert!(hit.task.completed_at.is_none(), "still open: the next one is due");
    assert_eq!(hit.last_done_at, Some(at(15, 12, 0)));
}
