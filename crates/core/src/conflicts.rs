//! Conflicts: this device's edits that lost to a concurrent edit from another
//! device (per-field newest-wins picked theirs). Device-local, not synced:
//! each device lists the edits *it* lost. Review: keep theirs (dismiss), or
//! use mine (written back as a new local edit, which then wins everywhere).
//!
//! "Concurrent" = the incoming change replaced an older version than the
//! one we had (its `prev_hlc` < our version's hlc), so its author never saw
//! our edit. Overwriting an edit you've seen is just editing.

use rusqlite::types::Value as SqlValue;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::Value;

use crate::model::Change;
use crate::store::{finish, new_id, Store};
use crate::sync::{to_json, to_sql, SyncChange, SCHEDULE_FIELDS, SYNCED};
use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conflict {
    pub id: String,
    pub table: String,
    pub row_id: String,
    pub field: String,
    /// What the edit was about: task title, time word, setting key.
    pub label: String,
    /// This device's value that lost.
    pub lost_value: Value,
    pub won_value: Value,
    /// Name of the paired device whose edit won (None: unknown, e.g. it
    /// came via another device).
    pub won_by: Option<String>,
    pub at: i64,
}

/// Fields worth asking about. Timestamps that only say "done"/"deleted"
/// count when that state differs, not when both are set at different times.
pub(crate) fn worth_logging(table: &str, field: &str, lost: &SqlValue, won: &Value) -> bool {
    let lost = to_json(lost.clone());
    match (table, field) {
        ("tasks", "completed_at" | "deleted_at") | ("time_words", "deleted_at") => lost.is_null() != won.is_null(),
        ("tasks", "title" | "description" | "due_at" | "rrule" | "adaptive")
        | ("time_words", "time")
        | ("settings", "value") => lost != *won,
        _ => false,
    }
}

/// Record that our `lost` value of `ch`'s field lost to `ch`. One open
/// conflict per field: a newer one replaces it.
pub(crate) fn log(c: &Connection, ch: &SyncChange, lost: SqlValue, now: i64) -> Result<()> {
    c.execute(
        "UPDATE conflicts SET deleted_at = ?4, updated_at = ?4
         WHERE tbl = ?1 AND row_id = ?2 AND field = ?3 AND resolution IS NULL AND deleted_at IS NULL",
        params![ch.table, ch.row_id, ch.field, now],
    )?;
    c.execute(
        "INSERT INTO conflicts (id, tbl, row_id, field, lost_value, won_value, won_origin, at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?8)",
        params![new_id(), ch.table, ch.row_id, ch.field, lost, to_sql(&ch.value), ch.origin, now],
    )?;
    Ok(())
}

impl Store {
    /// Open conflicts, newest first.
    pub fn conflicts(&self) -> Result<Vec<Conflict>> {
        let mut st = self.conn.prepare(
            "SELECT c.id, c.tbl, c.row_id, c.field, c.lost_value, c.won_value, c.at,
                    (SELECT p.name FROM peers p WHERE p.device = c.won_origin AND p.deleted_at IS NULL),
                    CASE c.tbl WHEN 'tasks' THEN (SELECT t.title FROM tasks t WHERE t.id = c.row_id) ELSE c.row_id END
             FROM conflicts c WHERE c.resolution IS NULL AND c.deleted_at IS NULL ORDER BY c.at DESC, c.id DESC",
        )?;
        let rows = st.query_map([], |r| {
            Ok(Conflict {
                id: r.get(0)?,
                table: r.get(1)?,
                row_id: r.get(2)?,
                field: r.get(3)?,
                lost_value: to_json(r.get(4)?),
                won_value: to_json(r.get(5)?),
                at: r.get(6)?,
                won_by: r.get(7)?,
                label: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Settle a conflict. `use_mine`: write our lost value back (a new local
    /// edit, so it syncs out and wins); otherwise keep theirs. Returns the
    /// alarm change when a task's schedule changed.
    pub fn resolve_conflict(&mut self, id: &str, use_mine: bool) -> Result<Option<Change>> {
        let row: Option<(String, String, String, SqlValue)> = self
            .conn
            .query_row(
                "SELECT tbl, row_id, field, lost_value FROM conflicts
                 WHERE id = ?1 AND resolution IS NULL AND deleted_at IS NULL",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )
            .optional()?;
        let Some((table, row_id, field, lost)) = row else { return Err(Error::NotFound(id.to_string())) };
        self.write(|c, now| {
            let resolution = if use_mine { "mine" } else { "theirs" };
            c.execute("UPDATE conflicts SET resolution = ?2, updated_at = ?3 WHERE id = ?1", params![id, resolution, now])?;
            if !use_mine {
                return Ok(None);
            }
            // Names come from our own table, but only ever touch synced fields.
            let t = SYNCED
                .iter()
                .find(|t| t.table == table && t.fields.contains(&field.as_str()))
                .ok_or_else(|| Error::Invalid(format!("can't restore {table}.{field}")))?;
            c.execute(
                &format!("UPDATE {} SET {field} = ?1, updated_at = ?2 WHERE {} = ?3", t.table, t.key),
                params![lost, now, row_id],
            )?;
            if t.table == "tasks" {
                return Ok(Some(finish(c, &row_id, SCHEDULE_FIELDS.contains(&field.as_str()))?));
            }
            Ok(None)
        })
    }
}
