//! Schema migrations. Append new migrations; never edit shipped ones.
//!
//! Sync-ready rules: UUIDv7 text keys, `updated_at` on every synced row,
//! soft deletes (`deleted_at`), instants as UTC epoch milliseconds.

use rusqlite_migration::{Migrations, M};

pub fn migrations() -> Migrations<'static> {
    Migrations::new(vec![
        M::up(
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
          kind        TEXT NOT NULL,      -- created|updated|snoozed|completed|missed|reopened|retired|deleted|restored|fired|dismissed
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
        ),
        M::up(
        "
        -- Shopping list: separate from tasks, no reminders.
        CREATE TABLE shopping_items (
          id         TEXT PRIMARY KEY,
          name       TEXT NOT NULL,
          bought_at  INTEGER,             -- ticked off; hidden from the list
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER
        );
        CREATE INDEX shopping_items_open ON shopping_items(bought_at, deleted_at);

        -- Things usually bought: chips that add to the list.
        CREATE TABLE staples (
          id         TEXT PRIMARY KEY,
          name       TEXT NOT NULL,
          created_at INTEGER NOT NULL,
          updated_at INTEGER NOT NULL,
          deleted_at INTEGER
        );
        ",
        ),
        M::up(
            "
        -- P2P sync (device-local bookkeeping; none of these tables sync).
        -- One row: this device's id, its hybrid logical clock, and whether
        -- remote changes are being applied (then triggers don't log).
        CREATE TABLE sync_state (
          id         INTEGER PRIMARY KEY CHECK (id = 1),
          device     TEXT NOT NULL,
          hlc        INTEGER NOT NULL DEFAULT 0,
          applying   INTEGER NOT NULL DEFAULT 0,
          backfilled INTEGER NOT NULL DEFAULT 0
        );
        INSERT INTO sync_state (id, device) VALUES (1, lower(hex(randomblob(16))));

        -- Latest value per synced field, filled by triggers (sync.rs).
        -- `seq` grows on every change: peers ask for everything after the
        -- last seq they got from us.
        CREATE TABLE changes (
          seq    INTEGER PRIMARY KEY AUTOINCREMENT,
          tbl    TEXT NOT NULL,
          row_id TEXT NOT NULL,
          field  TEXT NOT NULL,
          value,                          -- any type, as stored in the row
          hlc    INTEGER NOT NULL,
          origin TEXT NOT NULL,           -- device that made the change
          UNIQUE (tbl, row_id, field)
        );
        ",
        ),
        M::up(
            "
        -- Sync settings and paired devices (device-local, not synced).
        ALTER TABLE sync_state ADD COLUMN mode TEXT NOT NULL DEFAULT 'off';   -- off|lan|relay
        ALTER TABLE sync_state ADD COLUMN name TEXT NOT NULL DEFAULT '';      -- shown to other devices
        CREATE TABLE peers (
          endpoint     TEXT PRIMARY KEY,  -- iroh endpoint id (public key)
          device       TEXT NOT NULL,     -- their sync_state.device
          name         TEXT NOT NULL,
          their_seq    INTEGER NOT NULL DEFAULT 0,  -- last of their changes merged here
          paired_at    INTEGER NOT NULL,
          last_sync_at INTEGER,
          updated_at   INTEGER NOT NULL,
          deleted_at   INTEGER
        );
        ",
        ),
        M::up(
            "
        -- Last known addresses of a paired device (JSON list of \"ip:port\"),
        -- dialled directly so sync doesn't depend on mDNS.
        ALTER TABLE peers ADD COLUMN addrs TEXT NOT NULL DEFAULT '[]';
        ",
        ),
        M::up(
            "
        -- The hlc of the version a change replaced (NULL = unknown/new).
        -- Tells \"they overwrote my edit after seeing it\" from \"we both
        -- edited at once\" (a conflict).
        ALTER TABLE changes ADD COLUMN prev_hlc INTEGER;
        -- Scratch for the change-log triggers (always emptied first).
        CREATE TABLE change_prev (field TEXT PRIMARY KEY, hlc INTEGER);

        -- Device-local: this device's edits that lost to a concurrent edit
        -- from another device. Not synced. Values as in `changes` (JSON-ish).
        CREATE TABLE conflicts (
          id          TEXT PRIMARY KEY,   -- UUIDv7
          tbl         TEXT NOT NULL,
          row_id      TEXT NOT NULL,
          field       TEXT NOT NULL,
          lost_value,
          won_value,
          won_origin  TEXT NOT NULL,      -- device whose edit won
          at          INTEGER NOT NULL,   -- when it was noticed
          resolution  TEXT,               -- 'mine' | 'theirs' once reviewed
          updated_at  INTEGER NOT NULL,
          deleted_at  INTEGER
        );
        CREATE INDEX conflicts_open ON conflicts(resolution, at);
        ",
        ),
        M::up(
            "
        -- What a paired device is, as it says in its Hello: 'phone',
        -- 'computer', or '' (older version, not told yet).
        ALTER TABLE peers ADD COLUMN kind TEXT NOT NULL DEFAULT '';
        ",
        ),
        M::up(
            "
        -- A past completion the user said wasn't really done: kept, shown as
        -- not done, counted as missed. NULL again when marked done again.
        ALTER TABLE completions ADD COLUMN not_done_at INTEGER;
        ",
        ),
    ])
}

#[cfg(test)]
mod tests {
    #[test]
    fn migrations_are_valid() {
        super::migrations().validate().unwrap();
    }
}
