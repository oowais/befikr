//! Befikr core: storage, recurrence and reminder logic.
//!
//! All writes go through [`Store`]; it is the single place to emit sync
//! change events later. No Tauri or platform code here: the app calls it
//! from Tauri commands, Android calls it via JNI when no webview runs.

mod carefree;
mod conflicts;
mod digest;
mod model;
mod past;
mod peers;
pub mod recurrence;
mod schema;
mod shopping;
mod snooze;
mod store;
mod sync;

pub use model::*;
pub use peers::{Peer, SyncMode, SyncSettings};
pub use digest::{Digest, TodayTask, DEFAULT_DIGEST_TIME, DIGEST_ID};
pub use snooze::{snooze_picks, SnoozePick};
pub use store::Store;
pub use conflicts::Conflict;
pub use sync::{MergeOutcome, SyncChange};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid: {0}")]
    Invalid(String),
    #[error("database: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("migration: {0}")]
    Migration(String),
}

pub type Result<T> = std::result::Result<T, Error>;
