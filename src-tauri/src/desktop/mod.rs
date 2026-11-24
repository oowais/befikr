//! Desktop (Linux, Windows): reminders run in this process. A scheduler
//! thread fires them as notifications, re-armed from the database on start;
//! the tray keeps the app running with the window closed. Android uses the
//! Kotlin plugin instead.

pub mod backup;
pub mod digest;
pub mod migrate;
mod notify;
pub mod notify_settings;
pub mod prefs;
mod scheduler;
mod tray;

pub use notify::close as close_notification;
pub use tray::refresh as refresh_tray;
pub use scheduler::Scheduler;

use std::sync::atomic::{AtomicBool, Ordering};

use tauri::{AppHandle, Emitter, Manager, WindowEvent};

/// This computer's IANA zone ("today" for the digest and the tray).
pub fn local_tz() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into())
}

/// Show, unminimize and focus the main window.
pub fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        page_visible(&w, true);
    }
}

/// Tell the page whether it's on screen. WebView2 isn't told when the window
/// is hidden to the tray: the page stays "visible" and keeps its timers and
/// animations going (~0.5 MB/s of renderer traffic while hidden). Hidden, it
/// pauses them and gets `visibilitychange`. Other platforms follow the window.
fn page_visible(w: &tauri::WebviewWindow, visible: bool) {
    #[cfg(windows)]
    let _ = w.with_webview(move |pw| unsafe {
        let _ = pw.controller().SetIsVisible(visible);
    });
    #[cfg(not(windows))]
    let _ = (w, visible);
}

fn hide_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        page_visible(&w, false);
        let _ = w.hide();
    }
}

/// Bring the window up on a route (e.g. the snooze sheet for a task).
pub fn open_route(app: &AppHandle, route: &str) {
    show_main(app);
    let _ = app.emit("open-route", route);
}

/// Whether the tray is up; without it, closing the window must quit.
static TRAY: AtomicBool = AtomicBool::new(false);

/// Tray, then the window: shown unless starting hidden (see `prefs`).
pub fn setup(app: &AppHandle) {
    match tray::create(app) {
        Ok(()) => TRAY.store(true, Ordering::Relaxed),
        Err(e) => eprintln!("tray unavailable: {e}"),
    }
    backup::start_daily(app.clone());
    digest::schedule(app);
    let hidden = prefs::launch_hidden(app);
    if !hidden || !TRAY.load(Ordering::Relaxed) {
        show_main(app);
    } else {
        hide_main(app);
    }
}

/// Closing the window hides it to the tray; reminders keep running.
pub fn on_window_event(window: &tauri::Window, event: &WindowEvent) {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if TRAY.load(Ordering::Relaxed) {
            api.prevent_close();
            hide_main(window.app_handle());
        }
    }
}
