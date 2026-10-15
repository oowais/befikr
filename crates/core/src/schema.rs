//! Schema migrations. Append new migrations; never edit shipped ones.
//!
//! Sync-ready rules: UUIDv7 text keys, `updated_at` on every synced row,
//! soft deletes (`deleted_at`), instants as UTC epoch milliseconds.

use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(
        "
        CREATE TABLE tasks (
          id            TEXT PRIMARY KEY,
          parent_id     TEXT REFERENCES tasks(id),
          title         TEXT NOT NULL,
          description   TEXT NOT NULL DEFAULT '',
          due_at        INTEGER,          -- UTC ms
          tz            TEXT,             -- IANA zone; anchors recurrence
          rrule         TEXT,             -- RRULE body without DTSTART
          adaptive      INTEGER NOT NULL DEFAULT 0,
          completed_at  INTEGER,
          snoozed_until INTEGER,
          created_at    INTEGER NOT NULL,
          updated_at    INTEGER NOT NULL,
          deleted_at    INTEGER
        );
        CREATE INDEX tasks_parent ON tasks(parent_id);
        CREATE INDEX tasks_open ON tasks(completed_at, deleted_at);

        -- History, incl. each occurrence of recurring tasks.
        CREATE TABLE completions (
          id           TEXT PRIMARY KEY,
          task_id      TEXT NOT NULL REFERENCES tasks(id),
          completed_at INTEGER NOT NULL,
          due_at_then  INTEGER,
          updated_at   INTEGER NOT NULL,
          deleted_at   INTEGER
        );
        CREATE INDEX completions_task ON completions(task_id, completed_at);

        -- Append-only log for stats (snoozes, lateness, ...).
        CREATE TABLE task_events (
          id          TEXT PRIMARY KEY,
          task_id     TEXT NOT NULL,
          kind        TEXT NOT NULL,      -- created|updated|snoozed|completed|missed|reopened|deleted|restored|fired|dismissed
          at          INTEGER NOT NULL,
          due_at_then INTEGER,
          snoozed_to  INTEGER,
          source      TEXT NOT NULL       -- app|native
        );
        CREATE INDEX task_events_task ON task_events(task_id, at);

        CREATE TABLE settings (
          key        TEXT PRIMARY KEY,
          value      TEXT NOT NULL,       -- JSON
          updated_at INTEGER NOT NULL
        );

        CREATE TABLE time_words (
          word       TEXT PRIMARY KEY,    -- lowercase
          time       TEXT NOT NULL,       -- HH:MM
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER
        );
        INSERT INTO time_words (word, time, updated_at) VALUES
          ('morning', '09:00', 0), ('noon', '12:00', 0), ('afternoon', '14:00', 0),
          ('evening', '18:00', 0), ('night', '21:00', 0);

        -- Device-local reminder state. Not synced.
        CREATE TABLE alarm_state (
          task_id      TEXT PRIMARY KEY,
          next_fire_at INTEGER,
          fires        INTEGER NOT NULL DEFAULT 0
        );
        ",
    )])
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_valid() {
        super::migrations().validate().unwrap();
    }
}
