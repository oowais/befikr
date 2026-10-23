//! `.ics` export (RFC 5545): open dated tasks as calendar events, so another
//! calendar app can show them. One-way and read-only; nothing is imported.
//! A repeat exports as its series from the current occurrence on (missed
//! ones are not shown); a snoozed occurrence is an override of that series.
//! Adaptive repeats ("3 days after done") can't be written as an RRULE, so
//! only their current occurrence goes out.

use chrono::{Duration, Utc};
use chrono_tz::Tz;

use crate::model::Task;
use crate::recurrence::{from_ms, parse_tz};
use crate::store::Store;
use crate::Result;

/// Events need a length; tasks have none.
const EVENT_MINUTES: i64 = 30;

impl Store {
    /// The calendar file. `tz` is the zone used for a task that has none.
    pub fn export_ics(&self, tz: &str) -> Result<String> {
        let fallback = parse_tz(tz)?;
        let stamp = utc(self.now());
        let mut out = String::new();
        for line in ["BEGIN:VCALENDAR", "VERSION:2.0", "PRODID:-//Befikr//Befikr//EN", "CALSCALE:GREGORIAN", "X-WR-CALNAME:Befikr"] {
            push(&mut out, line);
        }
        for item in self.list_open()? {
            let t = item.task;
            let Some(due) = t.due_at else { continue };
            let zone = t.tz.as_deref().map(parse_tz).transpose()?.unwrap_or(fallback);
            match t.rrule.as_deref().filter(|_| !t.adaptive) {
                Some(rrule) => {
                    event(&mut out, &t, &stamp, zone, due, Some(rrule), None);
                    if let Some(moved) = t.snoozed_until.filter(|&s| s != due) {
                        event(&mut out, &t, &stamp, zone, moved, None, Some(due));
                    }
                }
                None => event(&mut out, &t, &stamp, zone, t.snoozed_until.unwrap_or(due), None, None),
            }
        }
        push(&mut out, "END:VCALENDAR");
        Ok(out)
    }
}

fn event(out: &mut String, t: &Task, stamp: &str, zone: Tz, start: i64, rrule: Option<&str>, recurrence_id: Option<i64>) {
    push(out, "BEGIN:VEVENT");
    push(out, &format!("UID:{}@befikr", t.id));
    push(out, &format!("DTSTAMP:{stamp}"));
    push(out, &format!("LAST-MODIFIED:{}", utc(t.updated_at)));
    if let Some(id) = recurrence_id {
        push(out, &format!("RECURRENCE-ID{}", local(id, zone)));
    }
    push(out, &format!("DTSTART{}", local(start, zone)));
    push(out, &format!("DTEND{}", local(start + Duration::minutes(EVENT_MINUTES).num_milliseconds(), zone)));
    if let Some(r) = rrule {
        push(out, &format!("RRULE:{r}"));
    }
    push(out, &format!("SUMMARY:{}", escape(&t.title)));
    if !t.description.trim().is_empty() {
        push(out, &format!("DESCRIPTION:{}", escape(&t.description)));
    }
    // Early reminders as the calendar's own alerts.
    for m in &t.remind_before {
        push(out, "BEGIN:VALARM");
        push(out, "ACTION:DISPLAY");
        push(out, &format!("DESCRIPTION:{}", escape(&t.title)));
        push(out, &format!("TRIGGER:-PT{m}M"));
        push(out, "END:VALARM");
    }
    push(out, "END:VEVENT");
}

/// `:20261015T080000Z`
fn utc(ms: i64) -> String {
    from_ms(ms, Tz::UTC).with_timezone(&Utc).format("%Y%m%dT%H%M%SZ").to_string()
}

/// `;TZID=Europe/Berlin:20261015T100000` (wall time, so repeats keep their
/// hour across DST), or UTC for UTC tasks.
fn local(ms: i64, zone: Tz) -> String {
    if zone == Tz::UTC {
        return format!(":{}", utc(ms));
    }
    format!(";TZID={}:{}", zone.name(), from_ms(ms, zone).format("%Y%m%dT%H%M%S"))
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace("\r\n", "\\n").replace(['\n', '\r'], "\\n")
}

/// One content line, folded at 75 octets (continuations start with a space).
fn push(out: &mut String, line: &str) {
    let mut width = 0;
    for c in line.chars() {
        if width + c.len_utf8() > 75 {
            out.push_str("\r\n ");
            width = 1;
        }
        out.push(c);
        width += c.len_utf8();
    }
    out.push_str("\r\n");
}
