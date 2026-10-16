use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";
const DAY: i64 = 86_400_000;

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

fn add(s: &mut Store, clock: &AtomicI64, created: i64, title: &str, due: Option<i64>, notes: &str) -> String {
    clock.store(created, Ordering::SeqCst);
    s.add_task(NewTask {
        title: title.into(),
        description: notes.into(),
        due_at: due,
        tz: due.map(|_| TZ.into()),
        ..Default::default()
    })
    .unwrap()
    .task
    .id
}

fn setup() -> (Store, Arc<AtomicI64>) {
    let clock = Arc::new(AtomicI64::new(0));
    let c = clock.clone();
    (Store::open_in_memory().unwrap().with_clock(move || c.load(Ordering::SeqCst)), clock)
}

#[test]
fn groups_titles_and_keeps_only_worthwhile_ones() {
    let (mut s, clock) = setup();
    let now = at(15, 10, 0);
    // Twice, long ago: kept.
    let a = add(&mut s, &clock, now - 200 * DAY, "call  mom", Some(at(1, 18, 0)), "ask about trip");
    s.complete_task(&a, Source::App).unwrap();
    let b = add(&mut s, &clock, now - 100 * DAY, "Call Mom", Some(at(2, 18, 0)), "");
    s.complete_task(&b, Source::App).unwrap();
    // Once, long ago, done: dropped.
    let c = add(&mut s, &clock, now - 90 * DAY, "renew passport", None, "");
    s.complete_task(&c, Source::App).unwrap();
    // Once, recently: kept.
    let d = add(&mut s, &clock, now - 3 * DAY, "water plants", None, "");
    s.complete_task(&d, Source::App).unwrap();
    // Once, long ago, still open: kept, with its id.
    let e = add(&mut s, &clock, now - 80 * DAY, "fix bike", None, "");
    // Deleted twice: dropped.
    for _ in 0..2 {
        let x = add(&mut s, &clock, now - DAY, "oops", None, "");
        s.delete_task(&x).unwrap();
    }
    clock.store(now, Ordering::SeqCst);

    let past = s.past_tasks(50).unwrap();
    let titles: Vec<_> = past.iter().map(|p| p.title.as_str()).collect();
    assert_eq!(titles.len(), 3, "{titles:?}");
    assert!(!titles.contains(&"renew passport") && !titles.contains(&"oops"));

    let mom = past.iter().find(|p| p.title == "Call Mom").expect("latest spelling");
    assert_eq!(mom.count, 2);
    assert_eq!(mom.notes, "ask about trip", "latest notes that aren't empty");
    assert_eq!(mom.usual_time.as_deref(), Some("18:00"));
    assert_eq!(mom.open_id, None);

    let bike = past.iter().find(|p| p.title == "fix bike").unwrap();
    assert_eq!(bike.open_id.as_deref(), Some(e.as_str()));

    // Recent one-off outranks old repeats (recency decay).
    assert_eq!(past[0].title, "water plants");
}

#[test]
fn usual_time_is_most_common_latest_wins_tie() {
    let (mut s, clock) = setup();
    let now = at(15, 10, 0);
    for (i, h) in [(10, 7), (9, 7), (8, 20), (7, 20), (6, 9)] {
        let id = add(&mut s, &clock, now - i * DAY, "gym", Some(at(1 + i as u32, h, 30)), "");
        s.complete_task(&id, Source::App).unwrap();
    }
    clock.store(now, Ordering::SeqCst);
    let past = s.past_tasks(10).unwrap();
    assert_eq!(past[0].count, 5);
    assert_eq!(past[0].usual_time.as_deref(), Some("20:30"));
}
