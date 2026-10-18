//! P2P sync, storage side (no networking here; see PLAN.md "P2P sync — plan").
//!
//! Triggers log every change to a synced field into `changes` (latest value
//! per field, stamped with a hybrid logical clock and the device that made
//! it). Peers exchange `changes` rows after the last `seq` they got from each
//! other and merge them per field: the newest (hlc, origin) wins. Applying
//! remote changes sets `sync_state.applying`, so triggers don't log them as
//! local ones; merge records them itself (with a new local `seq`, so they are
//! forwarded to other peers too).

use rusqlite::types::Value as SqlValue;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model::AlarmChange;
use crate::store::{finish, Store};
use crate::{Error, Result};

/// HLC = wall clock ms << 16 | counter.
const HLC_SHIFT: i64 = 1 << 16;

pub(crate) struct Synced {
    pub(crate) table: &'static str,
    pub(crate) key: &'static str,
    pub(crate) fields: &'static [&'static str],
    /// Rows outside this condition stay device-local (`ROW` = the row).
    only: Option<&'static str>,
}

/// What syncs. Not here: `alarm_state`, `sync_state`, `changes`, and
/// device-local settings. `updated_at` is local bookkeeping, not a field.
pub(crate) const SYNCED: &[Synced] = &[
    Synced {
        table: "tasks",
        key: "id",
        fields: &[
            "parent_id", "title", "description", "due_at", "tz", "rrule", "adaptive", "completed_at",
            "snoozed_until", "created_at", "deleted_at",
        ],
        only: None,
    },
    Synced {
        table: "completions",
        key: "id",
        fields: &["task_id", "completed_at", "due_at_then", "deleted_at", "not_done_at"],
        only: None,
    },
    Synced {
        table: "task_events",
        key: "id",
        fields: &["task_id", "kind", "at", "due_at_then", "snoozed_to", "source"],
        only: None,
    },
    // Keep in step with DEVICE_SETTINGS.
    Synced { table: "settings", key: "key", fields: &["value"], only: Some("ROW.key NOT IN ('backup_dir', 'digest_time')") },
    Synced { table: "time_words", key: "word", fields: &["time", "deleted_at"], only: None },
    Synced { table: "shopping_items", key: "id", fields: &["name", "bought_at", "created_at", "deleted_at"], only: None },
    Synced { table: "staples", key: "id", fields: &["name", "created_at", "deleted_at"], only: None },
];

/// Settings that belong to one device: where its backups go, and whether and
/// when it shows the morning digest (wanted on the phone, maybe not the PC).
pub(crate) const DEVICE_SETTINGS: &[&str] = &["backup_dir", "digest_time"];

/// Task fields whose change restarts the reminder (like a local edit would).
pub(crate) const SCHEDULE_FIELDS: &[&str] = &["due_at", "tz", "rrule", "snoozed_until", "completed_at", "deleted_at"];

/// One field's latest value, as exchanged between peers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncChange {
    pub seq: i64,
    pub table: String,
    pub row_id: String,
    pub field: String,
    pub value: Value,
    pub hlc: i64,
    pub origin: String,
    /// hlc of the version this change replaced, where it was made (None =
    /// a new row, or unknown: from before this was tracked).
    #[serde(default)]
    pub prev_hlc: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    /// Fields that won and were written.
    pub applied: u32,
    /// Alarm changes for tasks the merge touched; apply them like after a local write.
    pub alarms: Vec<AlarmChange>,
}

/// (Re)create the change-logging triggers and log rows that predate them.
/// Runs on every open, so triggers always match [`SYNCED`].
pub(crate) fn install(c: &Connection) -> Result<()> {
    for t in SYNCED {
        let Synced { table, key, .. } = t;
        let arms = |old: bool| {
            t.fields
                .iter()
                .map(|f| {
                    let changed = if old { format!(" WHERE NEW.{f} IS NOT OLD.{f}") } else { String::new() };
                    format!("SELECT '{f}' AS f, NEW.{f} AS v{changed}")
                })
                .collect::<Vec<_>>()
                .join(" UNION ALL ")
        };
        let only = t.only.map(|o| format!(" AND {}", o.replace("ROW", "NEW"))).unwrap_or_default();
        let body = |arms: String| {
            format!(
                // Delete + insert gives the row a new seq (so it's sent
                // again); the replaced versions' hlcs wait in `change_prev`.
                // (Not INSERT OR REPLACE: an upsert firing the trigger would
                // override its conflict handling.)
                "DELETE FROM change_prev;
                 INSERT INTO change_prev (field, hlc)
                   SELECT field, hlc FROM changes
                   WHERE tbl = '{table}' AND row_id = NEW.{key} AND field IN (SELECT f FROM ({arms}));
                 DELETE FROM changes WHERE tbl = '{table}' AND row_id = NEW.{key} AND field IN (SELECT f FROM ({arms}));
                 INSERT INTO changes (tbl, row_id, field, value, hlc, origin, prev_hlc)
                   SELECT '{table}', NEW.{key}, f, v, s.hlc, s.device, (SELECT p.hlc FROM change_prev p WHERE p.field = f)
                   FROM sync_state s, ({arms});"
            )
        };
        c.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS sync_{table}_ins;
             DROP TRIGGER IF EXISTS sync_{table}_upd;
             CREATE TRIGGER sync_{table}_ins AFTER INSERT ON {table}
               WHEN (SELECT applying FROM sync_state) = 0{only}
               BEGIN {} END;
             CREATE TRIGGER sync_{table}_upd AFTER UPDATE ON {table}
               WHEN (SELECT applying FROM sync_state) = 0{only}
               BEGIN {} END;",
            body(arms(false)),
            body(arms(true)),
        ))?;
    }
    // Device settings logged before they became device-only (digest, 0.25).
    for key in DEVICE_SETTINGS {
        c.execute("DELETE FROM changes WHERE tbl = 'settings' AND row_id = ?1", [key])?;
    }
    let backfilled: bool = c.query_row("SELECT backfilled FROM sync_state", [], |r| r.get(0))?;
    if !backfilled {
        bump_clock(c, crate::store::system_now())?;
        for t in SYNCED {
            let only = t.only.map(|o| format!(" WHERE {}", o.replace("ROW", "t"))).unwrap_or_default();
            for f in t.fields {
                c.execute(
                    &format!(
                        "INSERT OR IGNORE INTO changes (tbl, row_id, field, value, hlc, origin)
                         SELECT '{table}', t.{key}, '{f}', t.{f}, s.hlc, s.device FROM {table} t, sync_state s{only}",
                        table = t.table,
                        key = t.key,
                    ),
                    [],
                )?;
            }
        }
        c.execute("UPDATE sync_state SET backfilled = 1", [])?;
    }
    Ok(())
}

/// Advance the clock for a local write at `now` (ms).
pub(crate) fn bump_clock(c: &Connection, now: i64) -> Result<()> {
    c.execute("UPDATE sync_state SET hlc = max(hlc + 1, ?1)", [now.saturating_mul(HLC_SHIFT)])?;
    Ok(())
}

impl Store {
    /// This device's sync id.
    pub fn device_id(&self) -> Result<String> {
        Ok(self.conn.query_row("SELECT device FROM sync_state", [], |r| r.get(0))?)
    }

    /// Newest change seq here (what a peer has after taking everything).
    pub fn last_seq(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT coalesce(max(seq), 0) FROM changes", [], |r| r.get(0))?)
    }

    /// Changes after `seq` (exclusive), oldest first, at most `limit`.
    /// `skip_origin`: leave out changes made by that device (the peer asking
    /// has them already).
    pub fn changes_since(&self, seq: i64, limit: u32, skip_origin: Option<&str>) -> Result<Vec<SyncChange>> {
        let mut st = self.conn.prepare(
            "SELECT seq, tbl, row_id, field, value, hlc, origin, prev_hlc FROM changes
             WHERE seq > ?1 AND origin IS NOT ?3 ORDER BY seq LIMIT ?2",
        )?;
        let rows = st.query_map(params![seq, limit, skip_origin], |r| {
            Ok(SyncChange {
                seq: r.get(0)?,
                table: r.get(1)?,
                row_id: r.get(2)?,
                field: r.get(3)?,
                value: to_json(r.get(4)?),
                hlc: r.get(5)?,
                origin: r.get(6)?,
                prev_hlc: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    /// Apply a peer's changes: per field, the newest (hlc, origin) wins.
    /// Unknown tables/fields and device-local rows are ignored.
    pub fn merge(&mut self, incoming: &[SyncChange]) -> Result<MergeOutcome> {
        let me = self.device_id()?;
        self.write(|c, now| {
            c.execute("UPDATE sync_state SET applying = 1", [])?;
            // Rows may arrive before the rows they point to (subtask before parent).
            c.pragma_update(None, "defer_foreign_keys", "ON")?;
            let mut out = MergeOutcome::default();
            // (table, row) → winning (field, value), in arrival order.
            let mut rows: Vec<RowWrite> = Vec::new();
            let mut max_hlc = 0;
            for ch in incoming {
                max_hlc = max_hlc.max(ch.hlc);
                let Some(t) = SYNCED.iter().find(|t| t.table == ch.table) else { continue };
                let Some(field) = t.fields.iter().find(|f| **f == ch.field) else { continue };
                if t.table == "settings" && DEVICE_SETTINGS.contains(&ch.row_id.as_str()) {
                    continue;
                }
                let local: Option<(i64, String, SqlValue)> = c
                    .query_row(
                        "SELECT hlc, origin, value FROM changes WHERE tbl = ?1 AND row_id = ?2 AND field = ?3",
                        params![ch.table, ch.row_id, ch.field],
                        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
                    )
                    .optional()?;
                if local.as_ref().is_some_and(|(hlc, origin, _)| (*hlc, origin.as_str()) >= (ch.hlc, ch.origin.as_str())) {
                    continue;
                }
                // Our own edit loses to one made without seeing it (it
                // replaced an older version than ours): keep it for the
                // Conflicts page. A newer base means ours was already
                // superseded (e.g. we restored a backup), so no conflict.
                if let Some((hlc, origin, value)) = local {
                    let concurrent = ch.prev_hlc.is_some_and(|p| p < hlc);
                    if origin == me && concurrent && crate::conflicts::worth_logging(t.table, field, &value, &ch.value) {
                        crate::conflicts::log(c, ch, value, now)?;
                    }
                }
                c.execute(
                    "INSERT OR REPLACE INTO changes (tbl, row_id, field, value, hlc, origin, prev_hlc)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![ch.table, ch.row_id, ch.field, to_sql(&ch.value), ch.hlc, ch.origin, ch.prev_hlc],
                )?;
                match rows.iter_mut().find(|(rt, id, _)| rt.table == t.table && *id == ch.row_id) {
                    Some((_, _, fields)) => fields.push((field, &ch.value)),
                    None => rows.push((t, &ch.row_id, vec![(field, &ch.value)])),
                }
                out.applied += 1;
            }
            let mut touched: Vec<(String, bool)> = Vec::new();
            for (t, id, fields) in &rows {
                write_row(c, t, id, fields, now)?;
                if t.table == "tasks" {
                    let reset = fields.iter().any(|(f, _)| SCHEDULE_FIELDS.contains(f));
                    touched.push((id.to_string(), reset));
                }
            }
            c.execute("UPDATE sync_state SET applying = 0, hlc = max(hlc, ?1)", [max_hlc])?;
            for (id, reset) in touched {
                out.alarms.push(finish(c, &id, reset)?.alarm);
            }
            Ok(out)
        })
    }
}

/// One row's winning fields: (table, row id, [(field, value)]).
type RowWrite<'a> = (&'static Synced, &'a str, Vec<(&'static str, &'a Value)>);

pub(crate) fn write_row(c: &Connection, t: &Synced, id: &str, fields: &[(&str, &Value)], now: i64) -> Result<()> {
    let has_updated_at = t.table != "task_events";
    let exists: bool =
        c.query_row(&format!("SELECT count(*) > 0 FROM {} WHERE {} = ?1", t.table, t.key), [id], |r| r.get(0))?;
    let mut values: Vec<SqlValue> = fields.iter().map(|(_, v)| to_sql(v)).collect();
    let sql = if exists {
        let mut set: Vec<String> = fields.iter().enumerate().map(|(i, (f, _))| format!("{f} = ?{}", i + 2)).collect();
        if has_updated_at {
            set.push(format!("updated_at = ?{}", fields.len() + 2));
        }
        format!("UPDATE {} SET {} WHERE {} = ?1", t.table, set.join(", "), t.key)
    } else {
        let mut cols: Vec<&str> = fields.iter().map(|(f, _)| *f).collect();
        if has_updated_at {
            cols.push("updated_at");
        }
        let marks: Vec<String> = (2..cols.len() + 2).map(|i| format!("?{i}")).collect();
        format!("INSERT INTO {} ({}, {}) VALUES (?1, {})", t.table, t.key, cols.join(", "), marks.join(", "))
    };
    if has_updated_at {
        values.push(SqlValue::Integer(now));
    }
    values.insert(0, SqlValue::Text(id.to_string()));
    c.execute(&sql, rusqlite::params_from_iter(values))
        .map_err(|e| Error::Invalid(format!("sync: {} {id}: {e}", t.table)))?;
    Ok(())
}

pub(crate) fn to_json(v: SqlValue) -> Value {
    match v {
        SqlValue::Null => Value::Null,
        SqlValue::Integer(i) => i.into(),
        SqlValue::Real(f) => f.into(),
        SqlValue::Text(s) => s.into(),
        SqlValue::Blob(b) => b.into(),
    }
}

pub(crate) fn to_sql(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Bool(b) => SqlValue::Integer(*b as i64),
        Value::Number(n) => n.as_i64().map(SqlValue::Integer).unwrap_or_else(|| SqlValue::Real(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => SqlValue::Text(s.clone()),
        other => SqlValue::Text(other.to_string()),
    }
}
