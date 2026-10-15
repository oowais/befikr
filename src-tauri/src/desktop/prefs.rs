//! This computer's startup settings (not synced): start at login (the OS
//! entry the autostart plugin manages, launching with `--autostart`) and
//! start hidden (a small JSON file next to it).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopPrefs {
    pub autostart: bool,
    pub start_hidden: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Stored {
    start_hidden: bool,
}

fn file(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_config_dir().ok().map(|d| d.join("desktop.json"))
}

fn stored(app: &AppHandle) -> Stored {
    file(app)
        .and_then(|f| std::fs::read(f).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

pub fn get(app: &AppHandle) -> DesktopPrefs {
    DesktopPrefs {
        autostart: app.autolaunch().is_enabled().unwrap_or(false),
        start_hidden: stored(app).start_hidden,
    }
}

pub fn set_autostart(app: &AppHandle, on: bool) -> Result<(), String> {
    let a = app.autolaunch();
    if on { a.enable() } else { a.disable() }.map_err(|e| e.to_string())
}

pub fn set_start_hidden(app: &AppHandle, on: bool) -> Result<(), String> {
    let f = file(app).ok_or("no config folder")?;
    if let Some(dir) = f.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_vec(&Stored { start_hidden: on }).map_err(|e| e.to_string())?;
    std::fs::write(f, json).map_err(|e| e.to_string())
}

/// Start without a window: `--hidden`, or launched at login with "start hidden" on.
pub fn launch_hidden(app: &AppHandle) -> bool {
    let has = |flag: &str| std::env::args().any(|a| a == flag);
    has("--hidden") || (has("--autostart") && stored(app).start_hidden)
}
