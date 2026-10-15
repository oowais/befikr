//! Spike storage. Shared by Tauri commands (webview) and JNI (no webview).

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri_plugin_reminders::Alarm;

/// Re-remind escalation in minutes; last value repeats.
/// Spike uses short values; plan default is 5 → 15 → 30 → 60.
const REREMIND_MIN: &[i64] = &[1, 2, 5];

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn open(path: &str) -> Result<Connection, String> {
    let conn = Connection::open(path).map_err(err)?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(err)?;
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         CREATE TABLE IF NOT EXISTS tasks (
           id TEXT PRIMARY KEY,
           title TEXT NOT NULL,
           due_at INTEGER NOT NULL,
           next_fire_at INTEGER,
           fires INTEGER NOT NULL DEFAULT 0,
           alarm_mode TEXT NOT NULL,
           completed_at INTEGER
         );
         CREATE TABLE IF NOT EXISTS events (
           id TEXT PRIMARY KEY,
           task_id TEXT NOT NULL,
           kind TEXT NOT NULL,
           at INTEGER NOT NULL,
           source TEXT NOT NULL
         );",
    )
    .map_err(err)?;
    Ok(conn)
}

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub title: String,
    pub due_at: i64,
    pub next_fire_at: Option<i64>,
    pub fires: i64,
    pub alarm_mode: String,
    pub completed_at: Option<i64>,
}

impl Task {
    pub fn alarm(&self) -> Option<Alarm> {
        self.next_fire_at.map(|at| Alarm {
            id: self.id.clone(),
            title: self.title.clone(),
            at_ms: at,
            mode: self.alarm_mode.clone(),
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub task_id: String,
    pub title: Option<String>,
    pub kind: String,
    pub at: i64,
    pub source: String,
}

#[derive(Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    /// Show the notification now.
    pub show: bool,
    /// Schedule the next re-fire at this time.
    pub next_at_ms: Option<i64>,
}

fn log(conn: &Connection, task_id: &str, kind: &str, at: i64, source: &str) -> Result<(), String> {
    conn.execute(
        "INSERT INTO events (id, task_id, kind, at, source) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![new_id(), task_id, kind, at, source],
    )
    .map(|_| ())
    .map_err(err)
}

const TASK_COLS: &str = "id, title, due_at, next_fire_at, fires, alarm_mode, completed_at";

fn row_task(r: &rusqlite::Row) -> rusqlite::Result<Task> {
    Ok(Task {
        id: r.get(0)?,
        title: r.get(1)?,
        due_at: r.get(2)?,
        next_fire_at: r.get(3)?,
        fires: r.get(4)?,
        alarm_mode: r.get(5)?,
        completed_at: r.get(6)?,
    })
}

pub fn get_task(conn: &Connection, id: &str) -> Result<Option<Task>, String> {
    conn.query_row(&format!("SELECT {TASK_COLS} FROM tasks WHERE id = ?1"), [id], row_task)
        .optional()
        .map_err(err)
}

pub fn create_task(conn: &Connection, title: &str, due_at: i64, mode: &str) -> Result<Task, String> {
    let id = new_id();
    conn.execute(
        "INSERT INTO tasks (id, title, due_at, next_fire_at, alarm_mode) VALUES (?1, ?2, ?3, ?3, ?4)",
        params![id, title, due_at, mode],
    )
    .map_err(err)?;
    log(conn, &id, "created", now_ms(), "app")?;
    get_task(conn, &id)?.ok_or_else(|| "insert failed".into())
}

pub fn list_tasks(conn: &Connection) -> Result<Vec<Task>, String> {
    let mut st = conn
        .prepare(&format!(
            "SELECT {TASK_COLS} FROM tasks ORDER BY completed_at IS NOT NULL, due_at, id"
        ))
        .map_err(err)?;
    let rows = st.query_map([], row_task).map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

pub fn list_events(conn: &Connection, limit: i64) -> Result<Vec<Event>, String> {
    let mut st = conn
        .prepare(
            "SELECT e.task_id, t.title, e.kind, e.at, e.source FROM events e
             LEFT JOIN tasks t ON t.id = e.task_id ORDER BY e.at DESC, e.id DESC LIMIT ?1",
        )
        .map_err(err)?;
    let rows = st
        .query_map([limit], |r| {
            Ok(Event {
                task_id: r.get(0)?,
                title: r.get(1)?,
                kind: r.get(2)?,
                at: r.get(3)?,
                source: r.get(4)?,
            })
        })
        .map_err(err)?;
    rows.collect::<Result<_, _>>().map_err(err)
}

pub fn snooze(conn: &Connection, id: &str, until: i64) -> Result<Task, String> {
    conn.execute(
        "UPDATE tasks SET next_fire_at = ?2, fires = 0 WHERE id = ?1 AND completed_at IS NULL",
        params![id, until],
    )
    .map_err(err)?;
    log(conn, id, "snoozed", now_ms(), "app")?;
    get_task(conn, id)?.ok_or_else(|| "no such task".into())
}

/// Single entry point for alarm/notification events, from app or JNI.
pub fn on_event(conn: &Connection, id: &str, kind: &str, now: i64, source: &str) -> Result<Outcome, String> {
    let none = Outcome { show: false, next_at_ms: None };
    let Some(task) = get_task(conn, id)? else {
        return Ok(none);
    };
    log(conn, id, kind, now, source)?;
    match kind {
        "fired" => {
            if task.completed_at.is_some() {
                return Ok(none);
            }
            let fires = task.fires + 1;
            let idx = (task.fires as usize).min(REREMIND_MIN.len() - 1);
            let next = now + REREMIND_MIN[idx] * 60_000;
            conn.execute(
                "UPDATE tasks SET fires = ?2, next_fire_at = ?3 WHERE id = ?1",
                params![id, fires, next],
            )
            .map_err(err)?;
            Ok(Outcome { show: true, next_at_ms: Some(next) })
        }
        "completed" => {
            conn.execute(
                "UPDATE tasks SET completed_at = ?2, next_fire_at = NULL WHERE id = ?1 AND completed_at IS NULL",
                params![id, now],
            )
            .map_err(err)?;
            Ok(none)
        }
        // Re-fire is already scheduled at fire time, so dismiss is only recorded.
        "dismissed" => Ok(none),
        other => Err(format!("unknown event kind: {other}")),
    }
}

/// Every alarm that should exist right now (boot, app start, time change).
/// Missed alarms fire a few seconds from now.
pub fn pending_alarms(conn: &Connection, now: i64) -> Result<Vec<Alarm>, String> {
    Ok(list_tasks(conn)?
        .into_iter()
        .filter(|t| t.completed_at.is_none())
        .filter_map(|t| t.alarm())
        .map(|mut a| {
            a.at_ms = a.at_ms.max(now + 5_000);
            a
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mem() -> Connection {
        open(":memory:").unwrap()
    }

    #[test]
    fn fire_escalates_then_repeats_last_interval() {
        let c = mem();
        let t = create_task(&c, "x", 0, "exact").unwrap();
        let mins: Vec<i64> = (0..5)
            .map(|_| (on_event(&c, &t.id, "fired", 0, "test").unwrap().next_at_ms.unwrap()) / 60_000)
            .collect();
        assert_eq!(mins, vec![1, 2, 5, 5, 5]);
    }

    #[test]
    fn completed_stops_reminders() {
        let c = mem();
        let t = create_task(&c, "x", 0, "exact").unwrap();
        on_event(&c, &t.id, "completed", 10, "test").unwrap();
        let o = on_event(&c, &t.id, "fired", 20, "test").unwrap();
        assert_eq!(o, Outcome { show: false, next_at_ms: None });
        assert!(pending_alarms(&c, 0).unwrap().is_empty());
    }

    #[test]
    fn snooze_resets_escalation_and_pending_clamps_past() {
        let c = mem();
        let t = create_task(&c, "x", 1_000, "alarmClock").unwrap();
        on_event(&c, &t.id, "fired", 1_000, "test").unwrap();
        let t = snooze(&c, &t.id, 50_000).unwrap();
        assert_eq!((t.fires, t.next_fire_at), (0, Some(50_000)));
        let p = pending_alarms(&c, 100_000).unwrap();
        assert_eq!(p[0].at_ms, 105_000);
        assert_eq!(p[0].mode, "alarmClock");
    }
}
