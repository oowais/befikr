//! Befikr core: storage, recurrence and reminder logic.
//!
//! All writes go through [`Store`]; it is the single place to emit sync
//! change events later. No Tauri or platform code here: the app calls it
//! from Tauri commands, Android calls it via JNI when no webview runs.

mod model;
pub mod recurrence;
mod schema;
mod store;

pub use model::*;
pub use store::Store;

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
