//! Repository layer: the only place that writes to the database.
//! Every write logs a `task_events` row, bumps `updated_at` and returns the
//! alarm change the platform must apply.

use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::model::*;
use crate::recurrence::{self, parse_tz};
use crate::{schema, sync, Error, Result};


const DEFAULT_REREMIND: &[i64] = &[15, 30, 60];
/// Alarms that should have fired already are re-armed this far in the future.
const CATCH_UP_MS: i64 = 5_000;

pub struct Store {
    pub(crate) conn: Connection,
    clock: Box<dyn Fn() -> i64 + Send>,
}

pub(crate) fn system_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub(crate) fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

impl Store {
    pub fn open(path: &str) -> Result<Self> {
        Self::init(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(mut conn: Connection) -> Result<Self> {
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        schema::migrations().to_latest(&mut conn).map_err(|e| Error::Migration(e.to_string()))?;
        sync::install(&conn)?;
        Ok(Self { conn, clock: Box::new(system_now) })
    }

    /// Replace the clock (tests).
    pub fn with_clock(mut self, clock: impl Fn() -> i64 + Send + 'static) -> Self {
        self.clock = Box::new(clock);
        self
    }

    pub fn now(&self) -> i64 {
        (self.clock)()
    }

    pub(crate) fn write<T>(&mut self, f: impl FnOnce(&Connection, i64) -> Result<T>) -> Result<T> {
        let now = self.now();
        let tx = self.conn.transaction()?;
        sync::bump_clock(&tx, now)?;
        let out = f(&tx, now)?;
        tx.commit()?;
        Ok(out)
    }

    // ------------------------------------------------------------ reads

    pub fn get_task(&self, id: &str) -> Result<Task> {
        get(&self.conn, id)
    }

    /// Main list: open top-level tasks. Overdue (oldest first) → upcoming →
    /// no date. Ties in creation order (UUIDv7 ids sort by time).
    pub fn list_open(&self) -> Result<Vec<ListItem>> {
        let mut st = self.conn.prepare(&format!(
            "SELECT {COLS},
               (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id AND s.deleted_at IS NULL AND s.completed_at IS NOT NULL),
               (SELECT COUNT(*) FROM tasks s WHERE s.parent_id = t.id AND s.deleted_at IS NULL)
             FROM tasks t
             WHERE parent_id IS NULL AND completed_at IS NULL AND deleted_at IS NULL
             ORDER BY COALESCE(snoozed_until, due_at) IS NULL, COALESCE(snoozed_until, due_at), id"
        ))?;
        let rows = st.query_map([], |r| {
            Ok(ListItem { task: row_task(r)?, subtasks_done: r.get(13)?, subtasks_total: r.get(14)? })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn list_subtasks(&self, parent_id: &str) -> Result<Vec<Task>> {
        let mut st = self
            .conn
            .prepare(&format!("SELECT {COLS} FROM tasks WHERE parent_id = ?1 AND deleted_at IS NULL ORDER BY id"))?;
        let rows = st.query_map([parent_id], row_task)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Completion history, newest first.
    pub fn list_completed(&self, limit: u32) -> Result<Vec<Completion>> {
        let mut st = self.conn.prepare(
            "SELECT c.id, c.task_id, t.title, c.completed_at, c.due_at_then
             FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.deleted_at IS NULL AND t.deleted_at IS NULL
             ORDER BY c.completed_at DESC, c.id DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit], |r| {
            Ok(Completion {
                id: r.get(0)?,
                task_id: r.get(1)?,
                title: r.get(2)?,
                completed_at: r.get(3)?,
                due_at_then: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Completion history grouped by task, most recent first: how often each
    /// was done, and whether a repeating one was retired.
    pub fn completed_groups(&self) -> Result<Vec<CompletedGroup>> {
        let mut st = self.conn.prepare(
            "SELECT t.id, t.title, COUNT(c.id), MAX(c.completed_at), t.completed_at, t.rrule IS NOT NULL,
               (SELECT c2.due_at_then FROM completions c2 WHERE c2.task_id = t.id AND c2.deleted_at IS NULL
                ORDER BY c2.completed_at DESC, c2.id DESC LIMIT 1)
             FROM tasks t LEFT JOIN completions c ON c.task_id = t.id AND c.deleted_at IS NULL
             WHERE t.deleted_at IS NULL AND t.parent_id IS NULL
             GROUP BY t.id
             HAVING COUNT(c.id) > 0 OR (t.completed_at IS NOT NULL AND t.rrule IS NOT NULL)",
        )?;
        let mut rows: Vec<CompletedGroup> = st
            .query_map([], |r| {
                let last: Option<i64> = r.get(3)?;
                let done_at: Option<i64> = r.get(4)?;
                let repeating: bool = r.get(5)?;
                Ok(CompletedGroup {
                    task_id: r.get(0)?,
                    title: r.get(1)?,
                    count: r.get(2)?,
                    last_at: last.max(done_at).unwrap_or_default(),
                    due_at_then: r.get(6)?,
                    repeating,
                    retired: repeating && done_at.is_some() && done_at != last,
                })
            })?
            .collect::<rusqlite::Result<_>>()?;
        rows.sort_by(|a, b| b.last_at.cmp(&a.last_at).then_with(|| b.task_id.cmp(&a.task_id)));
        Ok(rows)
    }

    /// One task's completions, newest first.
    pub fn task_history(&self, id: &str) -> Result<Vec<Completion>> {
        let mut st = self.conn.prepare(
            "SELECT c.id, c.task_id, t.title, c.completed_at, c.due_at_then
             FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.task_id = ?1 AND c.deleted_at IS NULL
             ORDER BY c.completed_at DESC, c.id DESC",
        )?;
        let rows = st.query_map([id], |r| {
            Ok(Completion {
                id: r.get(0)?,
                task_id: r.get(1)?,
                title: r.get(2)?,
                completed_at: r.get(3)?,
                due_at_then: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Stats for completions, snoozes and missed repeats since `since`.
    /// A completion is late when it falls on a later day than its due time.
    /// (`None` = all time), up to now. `week_starts` (ascending, the
    /// caller's local week starts) bucket completions for a chart; the last
    /// bucket runs to now. Undone completions and deleted tasks don't count.
    pub fn stats(&self, since: Option<i64>, week_starts: &[i64]) -> Result<Stats> {
        let since = since.unwrap_or(i64::MIN);
        let from = since.min(week_starts.first().copied().unwrap_or(since));
        let mut st = self.conn.prepare(
            "SELECT c.completed_at, c.due_at_then, t.tz FROM completions c JOIN tasks t ON t.id = c.task_id
             WHERE c.deleted_at IS NULL AND t.deleted_at IS NULL AND c.completed_at >= ?1",
        )?;
        let rows = st.query_map([from], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, Option<i64>>(1)?, r.get::<_, Option<String>>(2)?))
        })?;
        let mut s = Stats {
            weeks: week_starts.iter().map(|&start| WeekStats { start, ..Default::default() }).collect(),
            ..Default::default()
        };
        let mut lateness = Vec::new();
        for row in rows {
            let (done, due, tz) = row?;
            let late = due.map(|d| is_late(d, done, tz.as_deref()));
            if done >= since {
                s.completed += 1;
                match (late, due) {
                    (Some(true), Some(d)) => {
                        s.late += 1;
                        lateness.push(done - d);
                    }
                    (Some(false), _) => s.on_time += 1,
                    _ => {}
                }
            }
            if let Some(w) = s.weeks.iter_mut().rev().find(|w| done >= w.start) {
                match late {
                    Some(true) => w.late += 1,
                    Some(false) => w.on_time += 1,
                    None => w.undated += 1,
                }
            }
        }
        lateness.sort_unstable();
        s.late_median_ms = lateness.get(lateness.len() / 2).copied();
        (s.snoozes, s.snoozed_tasks) = self.conn.query_row(
            "SELECT COUNT(*), COUNT(DISTINCT e.task_id) FROM task_events e JOIN tasks t ON t.id = e.task_id
             WHERE e.kind = 'snoozed' AND t.deleted_at IS NULL AND e.at >= ?1",
            [since],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        s.missed = self.conn.query_row(
            "SELECT COUNT(*) FROM task_events e JOIN tasks t ON t.id = e.task_id
             WHERE e.kind = 'missed' AND t.deleted_at IS NULL AND e.at >= ?1 AND NOT EXISTS (
               SELECT 1 FROM completions c
               WHERE c.task_id = e.task_id AND c.completed_at = e.at AND c.deleted_at IS NOT NULL)",
            [since],
            |r| r.get(0),
        )?;
        Ok(s)
    }

    /// Occurrences of a recurring task that passed while it sat overdue
    /// (for stats), oldest first. Skips those of undone completions.
    pub fn missed_occurrences(&self, task_id: &str) -> Result<Vec<i64>> {
        let mut st = self.conn.prepare(
            "SELECT e.due_at_then FROM task_events e
             WHERE e.task_id = ?1 AND e.kind = 'missed' AND NOT EXISTS (
               SELECT 1 FROM completions c
               WHERE c.task_id = e.task_id AND c.completed_at = e.at AND c.deleted_at IS NOT NULL)
             ORDER BY e.due_at_then",
        )?;
        let rows = st.query_map([task_id], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ------------------------------------------------------------ task writes

    pub fn add_task(&mut self, new: NewTask) -> Result<Change> {
        self.write(|c, now| {
            let title = new.title.trim();
            if title.is_empty() {
                return Err(Error::Invalid("title is empty".into()));
            }
            if let Some(pid) = &new.parent_id {
                let parent = get(c, pid)?;
                if parent.parent_id.is_some() {
                    return Err(Error::Invalid("subtasks cannot have subtasks".into()));
                }
                if new.due_at.is_some() || new.rrule.is_some() {
                    return Err(Error::Invalid("subtasks follow their parent's date".into()));
                }
            }
            check_schedule(new.due_at, new.tz.as_deref(), new.rrule.as_deref())?;
            let id = new_id();
            c.execute(
                "INSERT INTO tasks (id, parent_id, title, description, due_at, tz, rrule, adaptive, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                params![id, new.parent_id, title, new.description, new.due_at, new.tz, new.rrule, new.adaptive, now],
            )?;
            log(c, &id, "created", now, new.due_at, None, Source::App)?;
            finish(c, &id, true)
        })
    }

    pub fn update_task(&mut self, id: &str, patch: TaskPatch) -> Result<Change> {
        self.write(|c, now| {
            let mut t = get(c, id)?;
            if let Some(title) = patch.title {
                let title = title.trim().to_string();
                if title.is_empty() {
                    return Err(Error::Invalid("title is empty".into()));
                }
                t.title = title;
            }
            if let Some(d) = patch.description {
                t.description = d;
            }
            let schedule_changed = patch.due_at.is_some() || patch.rrule.is_some() || patch.tz.is_some();
            if let Some(v) = patch.due_at {
                t.due_at = v;
            }
            if let Some(v) = patch.tz {
                t.tz = v;
            }
            if let Some(v) = patch.rrule {
                t.rrule = v;
            }
            if let Some(v) = patch.adaptive {
                t.adaptive = v;
            }
            if t.parent_id.is_some() && (t.due_at.is_some() || t.rrule.is_some()) {
                return Err(Error::Invalid("subtasks follow their parent's date".into()));
            }
            check_schedule(t.due_at, t.tz.as_deref(), t.rrule.as_deref())?;
            if schedule_changed {
                // A new date replaces any snooze.
                t.snoozed_until = None;
            }
            c.execute(
                "UPDATE tasks SET title = ?2, description = ?3, due_at = ?4, tz = ?5, rrule = ?6, adaptive = ?7,
                   snoozed_until = ?8, updated_at = ?9 WHERE id = ?1",
                params![id, t.title, t.description, t.due_at, t.tz, t.rrule, t.adaptive, t.snoozed_until, now],
            )?;
            log(c, id, "updated", now, t.due_at, None, Source::App)?;
            finish(c, id, schedule_changed)
        })
    }

    /// Complete a task (or the current occurrence of a recurring one).
    pub fn complete_task(&mut self, id: &str, source: Source) -> Result<Change> {
        self.write(|c, now| complete(c, id, now, source))
    }

    /// End a repeating task for good: no more occurrences or reminders, off
    /// the list, its history and stats kept (unlike delete). Undo with
    /// [`Store::reopen_task`].
    pub fn retire_task(&mut self, id: &str) -> Result<Change> {
        self.write(|c, now| {
            let t = get(c, id)?;
            if t.rrule.is_none() || t.parent_id.is_some() || !t.is_open() {
                return Err(Error::Invalid("only open repeating tasks can be retired".into()));
            }
            c.execute(
                "UPDATE tasks SET completed_at = ?2, snoozed_until = NULL, updated_at = ?2 WHERE id = ?1",
                params![id, now],
            )?;
            log(c, id, "retired", now, t.effective_due(), None, Source::App)?;
            finish(c, id, true)
        })
    }

    /// Undo the latest completion, or bring back a retired task.
    pub fn reopen_task(&mut self, id: &str) -> Result<Change> {
        self.write(|c, now| {
            let t = get(c, id)?;
            let last: Option<(String, Option<i64>, i64)> = c
                .query_row(
                    "SELECT id, due_at_then, completed_at FROM completions WHERE task_id = ?1 AND deleted_at IS NULL
                     ORDER BY completed_at DESC, id DESC LIMIT 1",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                )
                .optional()?;
            // Retired (done with no completion at that moment): reopen only,
            // its completions stay.
            if t.rrule.is_some() && t.completed_at.is_some() && last.as_ref().map(|l| l.2) != t.completed_at {
                c.execute("UPDATE tasks SET completed_at = NULL, updated_at = ?2 WHERE id = ?1", params![id, now])?;
                log(c, id, "reopened", now, None, None, Source::App)?;
                return finish(c, id, true);
            }
            if let Some(done_at) = t.completed_at {
                c.execute("UPDATE tasks SET completed_at = NULL, updated_at = ?2 WHERE id = ?1", params![id, now])?;
                // Reopen subtasks that were auto-checked with the parent.
                c.execute(
                    "UPDATE tasks SET completed_at = NULL, updated_at = ?3 WHERE parent_id = ?1 AND completed_at = ?2",
                    params![id, done_at, now],
                )?;
            } else if t.rrule.is_some() {
                let Some((_, due_then, _)) = &last else {
                    return Err(Error::Invalid("nothing to reopen".into()));
                };
                c.execute(
                    "UPDATE tasks SET due_at = ?2, snoozed_until = NULL, updated_at = ?3 WHERE id = ?1",
                    params![id, due_then, now],
                )?;
            } else {
                return Err(Error::Invalid("task is not completed".into()));
            }
            if let Some((cid, _, _)) = last {
                c.execute("UPDATE completions SET deleted_at = ?2, updated_at = ?2 WHERE id = ?1", params![cid, now])?;
            }
            log(c, id, "reopened", now, None, None, Source::App)?;
            finish(c, id, true)
        })
    }

    /// Skip the current occurrence of a repeating task: move to the next one
    /// on its schedule without recording a completion. "From when done"
    /// repeats skip along their pattern too (counting from now could land
    /// before the occurrence being skipped).
    pub fn skip_task(&mut self, id: &str, source: Source) -> Result<Change> {
        self.write(|c, now| {
            let t = get(c, id)?;
            let (Some(rule), Some(due), Some(tz), None, true) = (&t.rrule, t.due_at, &t.tz, &t.parent_id, t.is_open())
            else {
                return Err(Error::Invalid("only open repeating tasks can be skipped".into()));
            };
            let Some(next) = recurrence::next_due(rule, parse_tz(tz)?, due, false, now)? else {
                return Err(Error::Invalid("no later occurrence to skip to".into()));
            };
            c.execute(
                "UPDATE tasks SET due_at = ?2, snoozed_until = NULL, updated_at = ?3 WHERE id = ?1",
                params![id, next, now],
            )?;
            c.execute(
                "UPDATE tasks SET completed_at = NULL, updated_at = ?2 WHERE parent_id = ?1 AND completed_at IS NOT NULL",
                params![id, now],
            )?;
            log(c, id, "skipped", now, t.effective_due(), None, source)?;
            finish(c, id, true)
        })
    }

    pub fn snooze_task(&mut self, id: &str, until: i64, source: Source) -> Result<Change> {
        self.write(|c, now| {
            let t = get(c, id)?;
            if t.parent_id.is_some() || !t.is_open() {
                return Err(Error::Invalid("only open top-level tasks can be snoozed".into()));
            }
            c.execute(
                "UPDATE tasks SET snoozed_until = ?2, updated_at = ?3 WHERE id = ?1",
                params![id, until, now],
            )?;
            log(c, id, "snoozed", now, t.effective_due(), Some(until), source)?;
            finish(c, id, true)
        })
    }

    /// Soft delete, with subtasks.
    pub fn delete_task(&mut self, id: &str) -> Result<Change> {
        self.write(|c, now| {
            get(c, id)?;
            c.execute(
                "UPDATE tasks SET deleted_at = ?2, updated_at = ?2 WHERE (id = ?1 OR parent_id = ?1) AND deleted_at IS NULL",
                params![id, now],
            )?;
            log(c, id, "deleted", now, None, None, Source::App)?;
            c.execute("DELETE FROM alarm_state WHERE task_id = ?1", [id])?;
            let task = get_any(c, id)?;
            Ok(Change { task, alarm: AlarmChange::Cancel { task_id: id.to_string() } })
        })
    }

    /// Undo a delete: brings back the task and the subtasks deleted with it.
    pub fn restore_task(&mut self, id: &str) -> Result<Change> {
        self.write(|c, now| {
            let t = get_any(c, id)?;
            let Some(deleted_at) = t.deleted_at else {
                return Err(Error::Invalid("task is not deleted".into()));
            };
            c.execute(
                "UPDATE tasks SET deleted_at = NULL, updated_at = ?3 WHERE (id = ?1 OR parent_id = ?1) AND deleted_at = ?2",
                params![id, deleted_at, now],
            )?;
            log(c, id, "restored", now, None, None, Source::App)?;
            finish(c, id, true)
        })
    }

    // ------------------------------------------------------------ reminders

    /// An alarm went off. Decides whether to show it and when to re-remind.
    pub fn on_fired(&mut self, id: &str, source: Source) -> Result<FireOutcome> {
        let reremind = self.settings()?.reremind_minutes;
        self.write(|c, now| {
            let none = FireOutcome { show: false, next_at_ms: None, repeats: false };
            let Ok(t) = get(c, id) else { return Ok(none) };
            let fires: Option<i64> =
                c.query_row("SELECT fires FROM alarm_state WHERE task_id = ?1", [id], |r| r.get(0)).optional()?;
            let (Some(fires), true) = (fires, t.is_open()) else { return Ok(none) };
            log(c, id, "fired", now, t.effective_due(), None, source)?;
            let idx = (fires as usize).min(reremind.len().saturating_sub(1));
            let next = now + reremind.get(idx).copied().unwrap_or(60) * 60_000;
            c.execute(
                "UPDATE alarm_state SET fires = ?2, next_fire_at = ?3 WHERE task_id = ?1",
                params![id, fires + 1, next],
            )?;
            Ok(FireOutcome { show: true, next_at_ms: Some(next), repeats: t.rrule.is_some() })
        })
    }

    /// Notification swiped away. The next re-remind is already scheduled.
    pub fn on_dismissed(&mut self, id: &str, source: Source) -> Result<()> {
        self.write(|c, now| {
            if get(c, id).is_ok() {
                log(c, id, "dismissed", now, None, None, source)?;
            }
            Ok(())
        })
    }

    /// Every alarm that should exist now (app start, boot, time change).
    pub fn pending_alarms(&self) -> Result<Vec<Alarm>> {
        let now = self.now();
        let mut st = self.conn.prepare(
            "SELECT t.id, t.title, a.next_fire_at FROM alarm_state a JOIN tasks t ON t.id = a.task_id
             WHERE a.next_fire_at IS NOT NULL AND t.completed_at IS NULL AND t.deleted_at IS NULL
             ORDER BY a.next_fire_at",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Alarm { task_id: r.get(0)?, title: r.get(1)?, at_ms: r.get::<_, i64>(2)?.max(now + CATCH_UP_MS) })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    // ------------------------------------------------------------ settings

    pub fn settings(&self) -> Result<Settings> {
        let reremind = self
            .setting::<Vec<i64>>("reremind_minutes")?
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| DEFAULT_REREMIND.to_vec());
        let mut st = self.conn.prepare("SELECT word, time FROM time_words WHERE deleted_at IS NULL")?;
        let time_words = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<rusqlite::Result<_>>()?;
        Ok(Settings {
            reremind_minutes: reremind,
            backup_dir: self.setting::<String>("backup_dir")?,
            time_words,
            // Never set: on at the default time. Set to null: off.
            digest_time: self
                .setting::<Option<String>>("digest_time")?
                .unwrap_or_else(|| Some(crate::digest::DEFAULT_DIGEST_TIME.into())),
        })
    }

    fn setting<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let raw: Option<String> =
            self.conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0)).optional()?;
        Ok(raw.and_then(|s| serde_json::from_str(&s).ok()))
    }

    fn set_setting(&mut self, key: &str, value: &impl serde::Serialize) -> Result<()> {
        let json = serde_json::to_string(value).map_err(|e| Error::Invalid(e.to_string()))?;
        self.write(|c, now| {
            c.execute(
                "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
                params![key, json, now],
            )?;
            Ok(())
        })
    }

    pub fn set_reremind_minutes(&mut self, minutes: Vec<i64>) -> Result<()> {
        if minutes.is_empty() || minutes.iter().any(|m| *m <= 0) {
            return Err(Error::Invalid("re-remind intervals must be positive".into()));
        }
        self.set_setting("reremind_minutes", &minutes)
    }

    /// Morning digest time ('HH:MM'), or `None` to turn it off.
    pub fn set_digest_time(&mut self, time: Option<&str>) -> Result<()> {
        if time.is_some_and(|t| !valid_hhmm(t)) {
            return Err(Error::Invalid("time must be HH:MM".into()));
        }
        self.set_setting("digest_time", &time)
    }

    pub fn set_backup_dir(&mut self, dir: Option<String>) -> Result<()> {
        self.set_setting("backup_dir", &dir)
    }

    pub fn set_time_word(&mut self, word: &str, time: &str) -> Result<()> {
        let word = word.trim().to_lowercase();
        if word.is_empty() || !word.chars().all(|ch| ch.is_alphabetic() || ch == ' ') {
            return Err(Error::Invalid("time word must be letters".into()));
        }
        if !valid_hhmm(time) {
            return Err(Error::Invalid("time must be HH:MM".into()));
        }
        self.write(|c, now| {
            c.execute(
                "INSERT INTO time_words (word, time, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(word) DO UPDATE SET time = excluded.time, updated_at = excluded.updated_at, deleted_at = NULL",
                params![word, time, now],
            )?;
            Ok(())
        })
    }

    pub fn remove_time_word(&mut self, word: &str) -> Result<()> {
        let word = word.trim().to_lowercase();
        self.write(|c, now| {
            c.execute(
                "UPDATE time_words SET deleted_at = ?2, updated_at = ?2 WHERE word = ?1",
                params![word, now],
            )?;
            Ok(())
        })
    }

    // ------------------------------------------------------------ backup

    /// Consistent snapshot via `VACUUM INTO`.
    pub fn export_to(&self, path: &str) -> Result<()> {
        // VACUUM INTO refuses to overwrite.
        let _ = std::fs::remove_file(path);
        self.conn.execute("VACUUM INTO ?1", [path])?;
        Ok(())
    }

    /// Replace everything with a snapshot made by [`Store::export_to`]. The
    /// live connection stays open (SQLite backup API), then migrations bring
    /// an older snapshot up to date. Refuses files that aren't Befikr
    /// databases or come from a newer app version.
    pub fn import_from(&mut self, path: &str) -> Result<()> {
        let src = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .map_err(|_| Error::Invalid("not a database file".into()))?;
        let is_befikr: bool = src
            .query_row("SELECT count(*) = 1 FROM sqlite_master WHERE type = 'table' AND name = 'tasks'", [], |r| r.get(0))
            .map_err(|_| Error::Invalid("not a database file".into()))?;
        if !is_befikr {
            return Err(Error::Invalid("not a Befikr backup".into()));
        }
        let version: i64 = src.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let latest: i64 = self.conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version > latest {
            return Err(Error::Invalid("backup is from a newer version of Befikr; update the app first".into()));
        }
        // The snapshot may come from another device: keep this one's sync
        // id, settings and paired devices.
        let sync = self.sync_settings()?;
        let peers = self.peers()?;
        let seq_before = self.last_seq()?.max(self.conn.query_row(
            "SELECT coalesce(max(seq), 0) FROM sqlite_sequence WHERE name = 'changes'",
            [],
            |r| r.get::<_, i64>(0),
        )?);
        rusqlite::backup::Backup::new(&src, &mut self.conn)?
            .run_to_completion(256, std::time::Duration::from_millis(0), None)?;
        schema::migrations().to_latest(&mut self.conn).map_err(|e| Error::Migration(e.to_string()))?;
        self.conn.execute("UPDATE sync_state SET device = ?1, applying = 0", [&sync.device])?;
        self.set_sync_mode(sync.mode)?;
        if !sync.name.is_empty() {
            self.set_device_name(&sync.name)?;
        }
        // Peers already have our changes up to seq_before: new ones must come
        // after it, or they'd never be sent (the snapshot's counter is lower).
        let bumped = self
            .conn
            .execute("UPDATE sqlite_sequence SET seq = max(seq, ?1) WHERE name = 'changes'", [seq_before])?;
        if bumped == 0 {
            self.conn.execute("INSERT INTO sqlite_sequence (name, seq) VALUES ('changes', ?1)", [seq_before])?;
        }
        // Paired devices stay paired, but we ask each for everything again
        // (their_seq 0): what they sent after the snapshot was rolled back
        // here. Merging is per-field newest-wins, so repeats are harmless.
        // Open conflicts in the snapshot are stale (maybe settled since).
        self.conn.execute(
            "UPDATE conflicts SET deleted_at = ?1, updated_at = ?1 WHERE deleted_at IS NULL AND resolution IS NULL",
            [self.now()],
        )?;
        self.conn.execute("DELETE FROM peers", [])?; // device-local table, replaced wholesale
        for p in peers {
            self.add_peer(&p.endpoint, &p.device, &p.name)?;
            self.set_peer_addrs(&p.endpoint, &p.addrs)?;
        }
        sync::install(&self.conn)?;
        Ok(())
    }
}

// ---------------------------------------------------------------- helpers

/// Late = done after the due day ended (in the task's zone); any time on
/// the due day counts as on time.
fn is_late(due: i64, done: i64, tz: Option<&str>) -> bool {
    let tz = tz.and_then(|z| parse_tz(z).ok()).unwrap_or(chrono_tz::UTC);
    recurrence::from_ms(done, tz).date_naive() > recurrence::from_ms(due, tz).date_naive()
}

const COLS: &str = "id, parent_id, title, description, due_at, tz, rrule, adaptive, completed_at, \
                    snoozed_until, created_at, updated_at, deleted_at";

fn row_task(r: &Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        parent_id: r.get(1)?,
        title: r.get(2)?,
        description: r.get(3)?,
        due_at: r.get(4)?,
        tz: r.get(5)?,
        rrule: r.get(6)?,
        adaptive: r.get(7)?,
        completed_at: r.get(8)?,
        snoozed_until: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
        deleted_at: r.get(12)?,
    })
}

fn get_any(c: &Connection, id: &str) -> Result<Task> {
    c.query_row(&format!("SELECT {COLS} FROM tasks WHERE id = ?1"), [id], row_task)
        .optional()?
        .ok_or_else(|| Error::NotFound(id.to_string()))
}

/// Non-deleted task.
fn get(c: &Connection, id: &str) -> Result<Task> {
    let t = get_any(c, id)?;
    if t.deleted_at.is_some() {
        return Err(Error::NotFound(id.to_string()));
    }
    Ok(t)
}

/// Event kinds: see the `task_events` comment in `schema.rs`, plus `skipped`.
fn log(
    c: &Connection,
    task_id: &str,
    kind: &str,
    at: i64,
    due_at_then: Option<i64>,
    snoozed_to: Option<i64>,
    source: Source,
) -> Result<()> {
    c.execute(
        "INSERT INTO task_events (id, task_id, kind, at, due_at_then, snoozed_to, source)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![new_id(), task_id, kind, at, due_at_then, snoozed_to, source.as_str()],
    )?;
    Ok(())
}

fn valid_hhmm(s: &str) -> bool {
    let Some((h, m)) = s.split_once(':') else { return false };
    h.len() == 2 && m.len() == 2 && h.parse::<u8>().is_ok_and(|h| h < 24) && m.parse::<u8>().is_ok_and(|m| m < 60)
}

fn check_schedule(due_at: Option<i64>, tz: Option<&str>, rrule: Option<&str>) -> Result<()> {
    if let Some(tz) = tz {
        parse_tz(tz)?;
    }
    if let Some(rule) = rrule {
        if due_at.is_none() || tz.is_none() {
            return Err(Error::Invalid("recurring tasks need a due date and time zone".into()));
        }
        recurrence::validate(rule)?;
    }
    Ok(())
}

fn complete(c: &Connection, id: &str, now: i64, source: Source) -> Result<Change> {
    let t = get(c, id)?;
    if t.completed_at.is_some() {
        return finish(c, id, false); // idempotent: double tap / notification + app
    }
    let next = match (&t.rrule, t.due_at, &t.tz) {
        (Some(rule), Some(due), Some(tz)) => recurrence::next_due(rule, parse_tz(tz)?, due, t.adaptive, now)?,
        _ => None,
    };
    c.execute(
        "INSERT INTO completions (id, task_id, completed_at, due_at_then, updated_at) VALUES (?1, ?2, ?3, ?4, ?3)",
        params![new_id(), id, now, t.due_at],
    )?;
    log(c, id, "completed", now, t.effective_due(), None, source)?;
    // Fixed schedule done late: log each occurrence that passed meanwhile
    // (stats only). `at` = this completion, so an undone completion
    // (completions.deleted_at set) identifies its missed rows.
    if let (Some(rule), Some(due), Some(tz), false) = (&t.rrule, t.due_at, &t.tz, t.adaptive) {
        for m in recurrence::missed(rule, parse_tz(tz)?, due, now)? {
            log(c, id, "missed", now, Some(m), None, source)?;
        }
    }
    if let Some(next) = next {
        // Next occurrence: new due, snooze cleared, subtasks unchecked.
        c.execute(
            "UPDATE tasks SET due_at = ?2, snoozed_until = NULL, updated_at = ?3 WHERE id = ?1",
            params![id, next, now],
        )?;
        c.execute(
            "UPDATE tasks SET completed_at = NULL, updated_at = ?2 WHERE parent_id = ?1 AND completed_at IS NOT NULL",
            params![id, now],
        )?;
    } else {
        c.execute("UPDATE tasks SET completed_at = ?2, updated_at = ?2 WHERE id = ?1", params![id, now])?;
        // Parent done → open subtasks done too (same timestamp, so undo can find them).
        c.execute(
            "UPDATE tasks SET completed_at = ?2, updated_at = ?2 WHERE parent_id = ?1 AND completed_at IS NULL AND deleted_at IS NULL",
            params![id, now],
        )?;
    }
    finish(c, id, true)
}

/// Re-read the task and bring its alarm in line with it. `reset` restarts
/// the reminder at the (effective) due time; otherwise an alarm already
/// in flight (e.g. mid re-remind escalation) is kept.
pub(crate) fn finish(c: &Connection, id: &str, reset: bool) -> Result<Change> {
    let task = get_any(c, id)?;
    let due = task.effective_due().filter(|_| task.parent_id.is_none() && task.is_open());
    let Some(due) = due else {
        c.execute("DELETE FROM alarm_state WHERE task_id = ?1", [id])?;
        return Ok(Change { alarm: AlarmChange::Cancel { task_id: id.to_string() }, task });
    };
    let current: Option<Option<i64>> = c
        .query_row("SELECT next_fire_at FROM alarm_state WHERE task_id = ?1", [id], |r| r.get(0))
        .optional()?;
    let at = match current {
        Some(Some(at)) if !reset => at,
        _ => {
            c.execute(
                "INSERT INTO alarm_state (task_id, next_fire_at, fires) VALUES (?1, ?2, 0)
                 ON CONFLICT(task_id) DO UPDATE SET next_fire_at = excluded.next_fire_at, fires = 0",
                params![id, due],
            )?;
            due
        }
    };
    let alarm = AlarmChange::Schedule(Alarm { task_id: id.to_string(), title: task.title.clone(), at_ms: at });
    Ok(Change { task, alarm })
}
