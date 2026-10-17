//! Whether Windows will show our toasts at all. Two switches can silence
//! every reminder without the app noticing: Windows' own "Notifications"
//! (Settings → System → Notifications) and Befikr's entry under it. Read from
//! the registry (absent = on); Settings warns and links there. Other systems
//! report nothing.

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
// Only Windows reports a block; elsewhere `blocked` is always None.
#[cfg_attr(not(windows), allow(dead_code))]
pub enum Blocked {
    /// All notifications are off in Windows.
    Windows,
    /// Befikr's notifications are off in Windows.
    App,
}

#[cfg(windows)]
pub fn blocked(app: &tauri::AppHandle) -> Option<Blocked> {
    use windows_registry::CURRENT_USER;
    let off = |path: &str, value: &str| CURRENT_USER.open(path).and_then(|k| k.get_u32(value)).is_ok_and(|v| v == 0);
    if off(r"Software\Microsoft\Windows\CurrentVersion\PushNotifications", "ToastEnabled") {
        return Some(Blocked::Windows);
    }
    // Installed builds notify as the app; dev builds borrow PowerShell's
    // identity (see notify.rs), so only the installed one is checked.
    let id = &app.config().identifier;
    let key = format!(r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings\{id}");
    (!cfg!(debug_assertions) && off(&key, "Enabled")).then_some(Blocked::App)
}

#[cfg(not(windows))]
pub fn blocked(_app: &tauri::AppHandle) -> Option<Blocked> {
    None
}

/// Windows' notification settings page.
#[cfg(windows)]
pub fn open_settings() -> Result<(), String> {
    std::process::Command::new("explorer.exe").arg("ms-settings:notifications").spawn().map(|_| ()).map_err(|e| e.to_string())
}

#[cfg(not(windows))]
pub fn open_settings() -> Result<(), String> {
    Err("Windows only".into())
}
