//! The app id changed from `com.befikr.app` to `com.owaisahmed.befikr`
//! (0.38), and with it every folder the app keeps data in. On the first start
//! under the new id, copy the old folders over (database, sync key, backup
//! and desktop settings, the webview's own storage), so the list, pairing
//! and settings carry on. Copied, not moved: the old app keeps working until
//! it's uninstalled. Android can't do this (one app can't read another's
//! storage): there it's backup → restore.

use std::fs;
use std::path::Path;
use tauri::{AppHandle, Manager};

const OLD_ID: &str = "com.befikr.app";

pub fn adopt_old_data(app: &AppHandle) {
    let p = app.path();
    let dirs = [p.app_data_dir(), p.app_config_dir(), p.app_local_data_dir()];
    let mut seen = Vec::new();
    for new in dirs.into_iter().flatten() {
        if seen.contains(&new) {
            continue;
        }
        seen.push(new.clone());
        let Some(old) = new.parent().map(|d| d.join(OLD_ID)) else { continue };
        // Only into a folder the new app hasn't used yet.
        if !old.is_dir() || new.exists() {
            continue;
        }
        match copy_dir(&old, &new) {
            Ok(()) => eprintln!("copied {} → {}", old.display(), new.display()),
            Err(e) => eprintln!("copy {} → {}: {e}", old.display(), new.display()),
        }
    }
}

/// Copy a folder tree; files that can't be read (in use) are skipped.
fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else if let Err(e) = fs::copy(entry.path(), &target) {
            eprintln!("skip {}: {e}", entry.path().display());
        }
    }
    Ok(())
}
