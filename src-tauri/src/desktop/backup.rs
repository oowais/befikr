//! Backups on this computer (Android does this in Kotlin, see `Backup.kt`):
//! daily into a folder the user picked, keeping the newest [`KEEP`]; one-off
//! export to a chosen file; picking a file to restore. Rust makes the snapshot
//! (`Store::export_to`). State (folder, last run) is a JSON file next to the
//! desktop prefs, not synced.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::Db;

const KEEP: usize = 7;
/// A backup is due once the last one is this old (the daily run drifts a little).
const DUE_MS: i64 = 23 * 3_600_000;
const CHECK_EVERY: Duration = Duration::from_secs(3600);

#[derive(Default, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupStatus {
    pub folder: Option<String>,
    pub last_at: Option<i64>,
    pub last_error: Option<String>,
}

/// The daily thread and "Back up now" never write at the same time.
static RUN: Mutex<()> = Mutex::new(());

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("backup.json"))
}

pub fn status(app: &AppHandle) -> BackupStatus {
    file(app)
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save(app: &AppHandle, s: &BackupStatus) -> Result<(), String> {
    let f = file(app).ok_or("no config folder")?;
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(f, serde_json::to_vec(s).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn now_ms() -> i64 {
    chrono::Utc::now().timestamp_millis()
}

/// `befikr-YYYY-MM-DD-HHmm.db`, local time (same as Android).
fn file_name() -> String {
    format!("befikr-{}.db", chrono::Local::now().format("%Y-%m-%d-%H%M"))
}

fn is_backup_name(name: &str) -> bool {
    let Some(stamp) = name.strip_prefix("befikr-").and_then(|n| n.strip_suffix(".db")) else {
        return false;
    };
    stamp.len() == 15
        && stamp.char_indices().all(|(i, c)| if [4, 7, 10].contains(&i) { c == '-' } else { c.is_ascii_digit() })
}

/// Backup files to delete so the newest [`KEEP`] remain. Names sort by time.
fn to_prune(names: &[String]) -> Vec<String> {
    let mut ours: Vec<&String> = names.iter().filter(|n| is_backup_name(n)).collect();
    ours.sort();
    let extra = ours.len().saturating_sub(KEEP);
    ours.into_iter().take(extra).cloned().collect()
}

/// Snapshot the live database to `out` (written beside it first, then renamed,
/// so a failed run never leaves a half file under the real name).
fn snapshot(app: &AppHandle, out: &Path) -> Result<(), String> {
    let part = out.with_extension("db.part");
    let db = app.state::<Db>();
    db.store.lock().map_err(|e| e.to_string())?.export_to(&part.to_string_lossy()).map_err(|e| e.to_string())?;
    std::fs::rename(&part, out).map_err(|e| {
        let _ = std::fs::remove_file(&part);
        e.to_string()
    })
}

/// Daily backup now: snapshot into the folder, prune to [`KEEP`].
pub fn run_now(app: &AppHandle) -> Result<BackupStatus, String> {
    let _guard = RUN.lock().unwrap_or_else(|e| e.into_inner());
    let mut s = status(app);
    let folder = s.folder.clone().ok_or("No backup folder chosen")?;
    let result = (|| {
        let dir = PathBuf::from(&folder);
        if !dir.is_dir() {
            return Err("Backup folder is missing; choose it again".to_string());
        }
        snapshot(app, &dir.join(file_name()))?;
        let names: Vec<String> = std::fs::read_dir(&dir)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok()?.file_name().into_string().ok())
            .collect();
        for name in to_prune(&names) {
            let _ = std::fs::remove_file(dir.join(name));
        }
        Ok(())
    })();
    match &result {
        Ok(()) => {
            s.last_at = Some(now_ms());
            s.last_error = None;
        }
        Err(e) => s.last_error = Some(e.clone()),
    }
    save(app, &s)?;
    result.map(|()| s)
}

/// Daily backups while the app runs (it lives in the tray): checked hourly, so
/// a computer that slept catches up within the hour after waking.
pub fn start_daily(app: AppHandle) {
    std::thread::spawn(move || loop {
        let s = status(&app);
        if s.folder.is_some() && s.last_at.is_none_or(|t| now_ms() - t >= DUE_MS) {
            if let Err(e) = run_now(&app) {
                eprintln!("daily backup failed: {e}");
            }
        }
        std::thread::sleep(CHECK_EVERY);
    });
}

/// Choose the daily folder and back up into it right away. Cancel keeps the old one.
pub fn pick_folder(app: &AppHandle) -> Result<BackupStatus, String> {
    let Some(dir) = app.dialog().file().set_title("Folder for daily backups").blocking_pick_folder() else {
        return Ok(status(app));
    };
    let dir = dir.into_path().map_err(|e| e.to_string())?;
    save(app, &BackupStatus { folder: Some(dir.to_string_lossy().into_owned()), last_at: None, last_error: None })?;
    run_now(app)
}

/// Daily backup off; files already in the folder stay.
pub fn clear_folder(app: &AppHandle) -> Result<BackupStatus, String> {
    let s = BackupStatus { folder: None, ..status(app) };
    save(app, &s)?;
    Ok(s)
}

/// One-off copy to a file the user names. `false` when cancelled.
pub fn export(app: &AppHandle) -> Result<bool, String> {
    let mut dialog = app.dialog().file().set_title("Export a copy").set_file_name(file_name()).add_filter("Befikr backup", &["db"]);
    if let Ok(docs) = app.path().document_dir() {
        dialog = dialog.set_directory(docs);
    }
    let Some(out) = dialog.blocking_save_file() else {
        return Ok(false);
    };
    snapshot(app, &out.into_path().map_err(|e| e.to_string())?)?;
    Ok(true)
}

/// Pick a backup to restore. Returns a copy in the app cache, since
/// `import_backup` deletes the file it restores from (the user's stays).
pub fn pick_import(app: &AppHandle) -> Result<Option<String>, String> {
    let Some(src) = app.dialog().file().set_title("Restore from a backup").add_filter("Befikr backup", &["db"]).blocking_pick_file() else {
        return Ok(None);
    };
    let src = src.into_path().map_err(|e| e.to_string())?;
    let dir = app.path().app_cache_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let out = dir.join("befikr-import.db");
    std::fs::copy(&src, &out).map_err(|e| e.to_string())?;
    Ok(Some(out.to_string_lossy().into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert!(is_backup_name("befikr-2026-10-16-2130.db"));
        assert!(!is_backup_name("befikr-2026-10-16-2130.db.part"));
        assert!(!is_backup_name("befikr-2026-10-16.db"));
        assert!(!is_backup_name("befikr-before-import.db"));
        assert!(!is_backup_name("notes.db"));
    }

    #[test]
    fn prunes_oldest_beyond_keep() {
        let mut names: Vec<String> = (1..=9).map(|d| format!("befikr-2026-10-{d:02}-0300.db")).collect();
        names.push("mine.db".into());
        names.reverse();
        assert_eq!(to_prune(&names), vec!["befikr-2026-10-01-0300.db", "befikr-2026-10-02-0300.db"]);
        assert!(to_prune(&names[..5]).is_empty());
    }
}
