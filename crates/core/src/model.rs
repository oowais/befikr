use chrono::Datelike;
use chrono_tz::Tz;
use serde::{Deserialize, Deserializer, Serialize};

use crate::recurrence::from_ms;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub parent_id: Option<String>,
    pub title: String,
    pub description: String,
    /// UTC ms.
    pub due_at: Option<i64>,
    /// IANA zone; anchors recurrence.
    pub tz: Option<String>,
    /// RRULE body without DTSTART.
    pub rrule: Option<String>,
    pub adaptive: bool,
    pub completed_at: Option<i64>,
    pub snoozed_until: Option<i64>,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl Task {
    /// When the task is effectively due: snooze wins over the scheduled due.
    pub fn effective_due(&self) -> Option<i64> {
        self.snoozed_until.or(self.due_at)
    }

    pub fn is_open(&self) -> bool {
        self.completed_at.is_none() && self.deleted_at.is_none()
    }
}

/// Main-list row.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ListItem {
    #[serde(flatten)]
    pub task: Task,
    pub subtasks_done: u32,
    pub subtasks_total: u32,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NewTask {
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub parent_id: Option<String>,
    pub due_at: Option<i64>,
    pub tz: Option<String>,
    pub rrule: Option<String>,
    #[serde(default)]
    pub adaptive: bool,
}

/// Partial update. Outer `None` = leave unchanged; `Some(None)` = clear.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskPatch {
    pub title: Option<String>,
    pub description: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub due_at: Option<Option<i64>>,
    #[serde(default, deserialize_with = "double_option")]
    pub tz: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub rrule: Option<Option<String>>,
    pub adaptive: Option<bool>,
}

fn double_option<'de, T: Deserialize<'de>, D: Deserializer<'de>>(d: D) -> Result<Option<Option<T>>, D::Error> {
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Completion {
    pub id: String,
    pub task_id: String,
    pub title: String,
    pub completed_at: i64,
    pub due_at_then: Option<i64>,
}

/// A task's completions, as one row of the history ("Water plants · 58×").
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletedGroup {
    pub task_id: String,
    pub title: String,
    pub count: u32,
    /// Latest completion, or when it was retired if later.
    pub last_at: i64,
    /// Due time of the latest completion (for "late").
    pub due_at_then: Option<i64>,
    pub repeating: bool,
    /// A repeating task ended for good with [`crate::Store::retire_task`].
    pub retired: bool,
}

/// Completion stats for a time range (see `Store::stats`).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stats {
    pub completed: u32,
    /// Done on or before the due day. `on_time + late` = completions with a due date.
    pub on_time: u32,
    pub late: u32,
    /// Median of how long late completions sat overdue.
    pub late_median_ms: Option<i64>,
    pub snoozes: u32,
    pub snoozed_tasks: u32,
    /// Occurrences of fixed-schedule repeats that passed while overdue.
    pub missed: u32,
    pub weeks: Vec<WeekStats>,
}

/// Completions per bucket; buckets start at the caller's local week starts.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeekStats {
    pub start: i64,
    pub on_time: u32,
    pub late: u32,
    pub undated: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    App,
    Native,
}

impl Source {
    pub fn as_str(self) -> &'static str {
        match self {
            Source::App => "app",
            Source::Native => "native",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub task_id: String,
    pub title: String,
    pub at_ms: i64,
}

/// What the platform scheduler must do after a write.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "op", rename_all = "camelCase")]
pub enum AlarmChange {
    Schedule(Alarm),
    #[serde(rename_all = "camelCase")]
    Cancel { task_id: String },
}

/// Result of a write: the task as stored and the alarm to apply.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Change {
    pub task: Task,
    pub alarm: AlarmChange,
}

/// Answer to "an alarm went off".
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FireOutcome {
    pub show: bool,
    pub next_at_ms: Option<i64>,
    /// Repeating task: the notification offers Skip.
    pub repeats: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Re-remind escalation in minutes; the last value repeats.
    pub reremind_minutes: Vec<i64>,
    pub backup_dir: Option<String>,
    /// word → 'HH:MM', incl. user-defined words.
    pub time_words: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Urgency {
    /// Overdue or due within 1 hour.
    Red,
    /// Due today.
    Yellow,
    /// Due after today.
    Green,
    /// No date.
    Gray,
}

const HOUR_MS: i64 = 3_600_000;

/// List colour. `tz` is the device's current zone ("today" is local).
pub fn urgency(effective_due: Option<i64>, now: i64, tz: Tz) -> Urgency {
    let Some(due) = effective_due else { return Urgency::Gray };
    if due - now <= HOUR_MS {
        return Urgency::Red;
    }
    let (d, n) = (from_ms(due, tz), from_ms(now, tz));
    if (d.year(), d.ordinal()) == (n.year(), n.ordinal()) {
        Urgency::Yellow
    } else {
        Urgency::Green
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn urgency_colours() {
        let tz = chrono_tz::Europe::Berlin;
        let at = |h, m| crate::recurrence::to_utc_ms(tz.with_ymd_and_hms(2026, 10, 15, h, m, 0).unwrap());
        let now = at(10, 0);
        assert_eq!(urgency(None, now, tz), Urgency::Gray);
        assert_eq!(urgency(Some(at(9, 0)), now, tz), Urgency::Red);
        assert_eq!(urgency(Some(at(10, 59)), now, tz), Urgency::Red);
        assert_eq!(urgency(Some(at(23, 59)), now, tz), Urgency::Yellow);
        assert_eq!(urgency(Some(at(23, 59) + 60_000), now, tz), Urgency::Green);
    }

    #[test]
    fn patch_distinguishes_clear_from_unchanged() {
        let p: TaskPatch = serde_json::from_str(r#"{"dueAt": null}"#).unwrap();
        assert_eq!(p.due_at, Some(None));
        assert_eq!(p.rrule, None);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingItem {
    pub id: String,
    pub name: String,
    pub bought_at: Option<i64>,
}

/// A chip of something usually bought.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Staple {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShoppingList {
    pub items: Vec<ShoppingItem>,
    pub staples: Vec<Staple>,
}
