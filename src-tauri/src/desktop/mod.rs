//! Desktop (Linux, Windows): reminders run in this process. A scheduler
//! thread fires them as notifications, re-armed from the database on start.
//! Android uses the Kotlin plugin instead.

mod notify;
mod scheduler;

pub use notify::close as close_notification;
pub use scheduler::Scheduler;

use tauri::{AppHandle, Emitter, Manager};

/// Show, unminimize and focus the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// Bring the window up on a route (e.g. the snooze sheet for a task).
pub fn open_route(app: &AppHandle, route: &str) {
    show_main(app);
    let _ = app.emit("open-route", route);
}
