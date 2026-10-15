//! Sync settings and paired devices. Device-local: none of this syncs.

use rusqlite::{params, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use crate::store::Store;
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    Off,
    /// Same network only: local discovery, no relays.
    Lan,
    /// Same network first, relays when away.
    Relay,
}

impl SyncMode {
    fn as_str(self) -> &'static str {
        match self {
            SyncMode::Off => "off",
            SyncMode::Lan => "lan",
            SyncMode::Relay => "relay",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "lan" => SyncMode::Lan,
            "relay" => SyncMode::Relay,
            _ => SyncMode::Off,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncSettings {
    pub mode: SyncMode,
    /// This device's name for other devices ("" = not set yet).
    pub name: String,
    pub device: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Peer {
    pub endpoint: String,
    pub device: String,
    pub name: String,
    pub their_seq: i64,
    pub paired_at: i64,
    pub last_sync_at: Option<i64>,
}

fn row_peer(r: &Row) -> rusqlite::Result<Peer> {
    Ok(Peer {
        endpoint: r.get(0)?,
        device: r.get(1)?,
        name: r.get(2)?,
        their_seq: r.get(3)?,
        paired_at: r.get(4)?,
        last_sync_at: r.get(5)?,
    })
}

const PEER_COLS: &str = "endpoint, device, name, their_seq, paired_at, last_sync_at";

impl Store {
    pub fn sync_settings(&self) -> Result<SyncSettings> {
        Ok(self.conn.query_row("SELECT mode, name, device FROM sync_state", [], |r| {
            Ok(SyncSettings { mode: SyncMode::parse(&r.get::<_, String>(0)?), name: r.get(1)?, device: r.get(2)? })
        })?)
    }

    pub fn set_sync_mode(&mut self, mode: SyncMode) -> Result<()> {
        self.conn.execute("UPDATE sync_state SET mode = ?1", [mode.as_str()])?;
        Ok(())
    }

    pub fn set_device_name(&mut self, name: &str) -> Result<()> {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        if name.is_empty() || name.chars().count() > 40 {
            return Err(Error::Invalid("name must be 1–40 characters".into()));
        }
        self.conn.execute("UPDATE sync_state SET name = ?1", [name])?;
        Ok(())
    }

    /// Paired devices, oldest first.
    pub fn peers(&self) -> Result<Vec<Peer>> {
        let mut st =
            self.conn.prepare(&format!("SELECT {PEER_COLS} FROM peers WHERE deleted_at IS NULL ORDER BY paired_at"))?;
        let rows = st.query_map([], row_peer)?;
        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }

    pub fn peer(&self, endpoint: &str) -> Result<Option<Peer>> {
        Ok(self
            .conn
            .query_row(
                &format!("SELECT {PEER_COLS} FROM peers WHERE endpoint = ?1 AND deleted_at IS NULL"),
                [endpoint],
                row_peer,
            )
            .optional()?)
    }

    /// Pair (or re-pair) a device. Re-pairing keeps what was already merged.
    pub fn add_peer(&mut self, endpoint: &str, device: &str, name: &str) -> Result<()> {
        let now = self.now();
        self.conn.execute(
            "INSERT INTO peers (endpoint, device, name, paired_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(endpoint) DO UPDATE SET device = excluded.device, name = excluded.name,
               paired_at = CASE WHEN deleted_at IS NULL THEN paired_at ELSE excluded.paired_at END,
               updated_at = excluded.updated_at, deleted_at = NULL",
            params![endpoint, device, name, now],
        )?;
        Ok(())
    }

    pub fn remove_peer(&mut self, endpoint: &str) -> Result<()> {
        let now = self.now();
        self.conn.execute("UPDATE peers SET deleted_at = ?2, updated_at = ?2 WHERE endpoint = ?1", params![endpoint, now])?;
        Ok(())
    }

    /// A sync finished: remember the last of their changes we merged.
    pub fn peer_synced(&mut self, endpoint: &str, their_seq: i64, name: &str) -> Result<()> {
        let now = self.now();
        self.conn.execute(
            "UPDATE peers SET their_seq = max(their_seq, ?2), name = ?3, last_sync_at = ?4, updated_at = ?4 WHERE endpoint = ?1",
            params![endpoint, their_seq, name, now],
        )?;
        Ok(())
    }
}
