use befikr_core::recurrence::to_utc_ms;
use befikr_core::*;
use chrono::TimeZone;

const TZ: &str = "Europe/Berlin";

fn at(d: u32, h: u32, m: u32) -> i64 {
    to_utc_ms(chrono_tz::Europe::Berlin.with_ymd_and_hms(2026, 10, d, h, m, 0).unwrap())
}

/// Thu 2026-10-15 10:00 Berlin.
fn store() -> Store {
    Store::open_in_memory().unwrap().with_clock(|| at(15, 10, 0))
}

fn add(s: &mut Store, t: NewTask) -> String {
    let tz = t.due_at.map(|_| TZ.into());
    s.add_task(NewTask { tz, ..t }).unwrap().task.id
}

fn events(ics: &str) -> Vec<&str> {
    ics.split("BEGIN:VEVENT\r\n").skip(1).collect()
}

#[test]
fn calendar_wrapper_and_crlf() {
    let ics = store().export_ics(TZ).unwrap();
    assert!(ics.starts_with("BEGIN:VCALENDAR\r\nVERSION:2.0\r\n"));
    assert!(ics.ends_with("END:VCALENDAR\r\n"));
    assert!(!ics.replace("\r\n", "").contains('\n'));
}

#[test]
fn one_off_in_wall_time_and_undated_left_out() {
    let mut s = store();
    add(&mut s, NewTask { title: "undated".into(), ..Default::default() });
    let id = add(&mut s, NewTask { title: "Call mum, then dad; ok".into(), description: "line 1\nline 2".into(), due_at: Some(at(16, 18, 0)), ..Default::default() });
    let ics = s.export_ics(TZ).unwrap();
    let e = events(&ics);
    assert_eq!(e.len(), 1);
    assert!(e[0].contains(&format!("UID:{id}@befikr\r\n")));
    assert!(e[0].contains("DTSTART;TZID=Europe/Berlin:20261016T180000\r\n"));
    assert!(e[0].contains("DTEND;TZID=Europe/Berlin:20261016T183000\r\n"));
    assert!(e[0].contains("SUMMARY:Call mum\\, then dad\\; ok\r\n"));
    assert!(e[0].contains("DESCRIPTION:line 1\\nline 2\r\n"));
    assert!(e[0].contains("DTSTAMP:20261015T080000Z\r\n"));
    assert!(!e[0].contains("RRULE"));
}

#[test]
fn snoozed_one_off_exports_where_it_now_is() {
    let mut s = store();
    let id = add(&mut s, NewTask { title: "bins".into(), due_at: Some(at(15, 9, 0)), ..Default::default() });
    s.snooze_task(&id, at(15, 20, 0), Source::App).unwrap();
    let ics = s.export_ics(TZ).unwrap();
    assert!(ics.contains("DTSTART;TZID=Europe/Berlin:20261015T200000\r\n"));
}

#[test]
fn repeat_is_a_series_with_snooze_as_override() {
    let mut s = store();
    let id = add(&mut s, NewTask { title: "gym".into(), due_at: Some(at(15, 18, 0)), rrule: Some("FREQ=WEEKLY;BYDAY=TU,TH".into()), ..Default::default() });
    s.snooze_task(&id, at(16, 18, 0), Source::App).unwrap();
    let ics = s.export_ics(TZ).unwrap();
    let e = events(&ics);
    assert_eq!(e.len(), 2);
    assert!(e[0].contains("DTSTART;TZID=Europe/Berlin:20261015T180000\r\nDTEND"));
    assert!(e[0].contains("RRULE:FREQ=WEEKLY;BYDAY=TU,TH\r\n"));
    assert!(e[1].contains(&format!("UID:{id}@befikr\r\n")));
    assert!(e[1].contains("RECURRENCE-ID;TZID=Europe/Berlin:20261015T180000\r\n"));
    assert!(e[1].contains("DTSTART;TZID=Europe/Berlin:20261016T180000\r\n"));
    assert!(!e[1].contains("RRULE"));
}

#[test]
fn adaptive_repeat_exports_only_the_next_one() {
    let mut s = store();
    add(&mut s, NewTask { title: "water plants".into(), due_at: Some(at(17, 9, 0)), rrule: Some("FREQ=DAILY;INTERVAL=3".into()), adaptive: true, ..Default::default() });
    let ics = s.export_ics(TZ).unwrap();
    assert_eq!(events(&ics).len(), 1);
    assert!(!ics.contains("RRULE"));
}

#[test]
fn done_and_deleted_left_out() {
    let mut s = store();
    let a = add(&mut s, NewTask { title: "done".into(), due_at: Some(at(16, 9, 0)), ..Default::default() });
    let b = add(&mut s, NewTask { title: "gone".into(), due_at: Some(at(16, 9, 0)), ..Default::default() });
    s.complete_task(&a, Source::App).unwrap();
    s.delete_task(&b).unwrap();
    assert!(events(&s.export_ics(TZ).unwrap()).is_empty());
}

#[test]
fn long_lines_fold_at_75_octets() {
    let mut s = store();
    let title = "ä".repeat(60);
    add(&mut s, NewTask { title: title.clone(), due_at: Some(at(16, 9, 0)), ..Default::default() });
    let ics = s.export_ics(TZ).unwrap();
    assert!(ics.split("\r\n").all(|l| l.len() <= 75));
    let unfolded = ics.replace("\r\n ", "");
    assert!(unfolded.contains(&format!("SUMMARY:{title}\r\n")));
}
