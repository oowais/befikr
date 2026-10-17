//! Calendar (desktop week and month views): open tasks placed on their
//! (effective) due, plus the later occurrences of repeating tasks, which are
//! worked out here and not stored. Only the current occurrence is a real
//! task; moving it is a snooze, so the series keeps its schedule.

use serde::Serialize;

use crate::model::Task;
use crate::recurrence::{parse_tz, upcoming};
use crate::store::Store;
use crate::Result;

/// Later repeats per task in one range (a daily task over six weeks fits).
const MAX_REPEATS: u16 = 64;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarItem {
    #[serde(flatten)]
    pub task: Task,
    /// Where it sits on the calendar, UTC ms.
    pub at: i64,
    /// A later occurrence of a repeating task (not draggable, not a row).
    pub upcoming: bool,
}

impl Store {
    /// Open top-level tasks due in `[from, to)` and the later repeats that
    /// fall there, in time order. `tz` is the zone the series are read in
    /// when a task has none.
    pub fn calendar(&self, from: i64, to: i64, tz: &str) -> Result<Vec<CalendarItem>> {
        let fallback = parse_tz(tz)?;
        let mut out = Vec::new();
        for item in self.list_open()? {
            let t = item.task;
            let Some(at) = t.effective_due() else { continue };
            if (from..to).contains(&at) {
                out.push(CalendarItem { task: t.clone(), at, upcoming: false });
            }
            let (Some(rrule), Some(due)) = (t.rrule.as_deref(), t.due_at) else { continue };
            let zone = t.tz.as_deref().map(parse_tz).transpose()?.unwrap_or(fallback);
            // Still to come: after the current occurrence, a snooze that moved
            // it on, and now (missed ones are skipped when it's done).
            let after = (from - 1).max(at).max(due).max(self.now());
            for when in upcoming(rrule, zone, due, after, to, MAX_REPEATS)? {
                out.push(CalendarItem { task: t.clone(), at: when, upcoming: true });
            }
        }
        out.sort_by(|a, b| (a.at, &a.task.id).cmp(&(b.at, &b.task.id)));
        Ok(out)
    }
}
