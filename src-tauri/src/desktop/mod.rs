//! Desktop (Linux, Windows): reminders run in this process. A scheduler
//! thread fires them as notifications, re-armed from the database on start;
//! the tray keeps the app running with the window closed. Android uses the
//! Kotlin plugin instead.

mod notify;
mod scheduler;
mod tray;

pub use notify::close as close_notification;
pub use scheduler::Scheduler;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, WindowEvent};

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

/// Whether the tray is up; without it, closing the window must quit.
static TRAY: AtomicBool = AtomicBool::new(false);

/// Tray, then the window: shown unless started with `--hidden`.
pub fn setup(app: &AppHandle) {
    match tray::create(app) {
        Ok(()) => TRAY.store(true, Ordering::Relaxed),
        Err(e) => eprintln!("tray unavailable: {e}"),
    }
    let hidden = std::env::args().any(|a| a == "--hidden");
    if !hidden || !TRAY.load(Ordering::Relaxed) {
        show_main(app);
    }
}

/// Closing the window hides it to the tray; reminders keep running.
pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if TRAY.load(Ordering::Relaxed) {
            api.prevent_close();
            let _ = window.hide();
        }
    }
}
