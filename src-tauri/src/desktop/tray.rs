//! Tray icon: keeps the app (and its reminders) running with the window
//! closed. Menu: Open, Quick add, Quit; a left click opens the window.
//! While a task is overdue the icon gets a red dot, and the tooltip says what
//! is overdue and what's next today (Windows; Linux tray hosts may not show it).

use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use befikr_core::TodayTask;
use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager};

/// Tasks turn overdue as time passes, with no write to tell us.
const RECHECK: Duration = Duration::from_secs(60);

/// Wakes the status thread (a channel, so callers never wait on the database).
static NUDGE: OnceLock<Mutex<Sender<()>>> = OnceLock::new();

/// A task changed: bring the tray up to date soon.
pub fn refresh() {
    if let Some(tx) = NUDGE.get() {
        let _ = tx.lock().map(|tx| tx.send(()));
    }
}

pub fn create(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Befikr", true, None::<&str>)?;
    let add = MenuItem::with_id(app, "add", "Quick add", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &add, &PredefinedMenuItem::separator(app)?, &quit])?;
    let mut tray = TrayIconBuilder::with_id("main")
        .tooltip("Befikr")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => super::show_main(app),
            "add" => {
                super::show_main(app);
                let _ = app.emit("quick-add", ());
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, e| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e {
                super::show_main(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    start_status(app.clone());
    Ok(())
}

fn start_status(app: AppHandle) {
    let (tx, rx) = mpsc::channel();
    let _ = NUDGE.set(Mutex::new(tx));
    std::thread::Builder::new()
        .name("tray-status".into())
        .spawn(move || {
            let (plain, dotted) = match app.default_window_icon() {
                Some(i) => (i.clone().to_owned(), with_dot(i)),
                None => return,
            };
            let mut shown: Option<(bool, String)> = None;
            loop {
                let today = app
                    .state::<crate::Db>()
                    .store
                    .lock()
                    .ok()
                    .and_then(|s| s.today(&super::local_tz()).ok())
                    .unwrap_or_default();
                let now = (today.iter().any(|t| t.overdue), tooltip(&today));
                if shown.as_ref() != Some(&now) {
                    if let Some(tray) = app.tray_by_id("main") {
                        let _ = tray.set_icon(Some(if now.0 { dotted.clone() } else { plain.clone() }));
                        let _ = tray.set_tooltip(Some(&now.1));
                    }
                    shown = Some(now);
                }
                match rx.recv_timeout(RECHECK) {
                    Ok(()) | Err(RecvTimeoutError::Timeout) => {
                        // Coalesce a burst of changes into one update.
                        while rx.try_recv().is_ok() {}
                    }
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            }
        })
        .map(|_| ())
        .unwrap_or_else(|e| eprintln!("tray status thread: {e}"));
}

/// "Befikr · 2 overdue · next: Call mum 10:00"
fn tooltip(today: &[TodayTask]) -> String {
    let overdue = today.iter().filter(|t| t.overdue).count();
    let mut parts = vec!["Befikr".to_string()];
    if overdue > 0 {
        parts.push(format!("{overdue} overdue"));
    }
    if let Some(next) = today.iter().find(|t| !t.overdue) {
        parts.push(format!("next: {} {}", next.title, next.when));
    }
    if parts.len() == 1 {
        parts.push("all clear today".into());
    }
    parts.join(" · ")
}

/// The app icon with a red dot in the top-right corner.
fn with_dot(icon: &Image<'_>) -> Image<'static> {
    let (w, h) = (icon.width(), icon.height());
    let mut rgba = icon.rgba().to_vec();
    let r = w.min(h) as f32 * 0.22;
    let (cx, cy) = (w as f32 - r - 0.5, r + 0.5);
    // Clear ring around the dot so it reads on any icon colour.
    let ring = r + w.min(h) as f32 * 0.06;
    for y in 0..h {
        for x in 0..w {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let i = ((y * w + x) * 4) as usize;
            if d <= r {
                rgba[i..i + 4].copy_from_slice(&[0xE5, 0x48, 0x4D, 0xFF]);
            } else if d <= ring {
                rgba[i + 3] = 0;
            }
        }
    }
    Image::new_owned(rgba, w, h)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(title: &str, when: &str, overdue: bool) -> TodayTask {
        TodayTask { id: title.into(), title: title.into(), when: when.into(), overdue }
    }

    #[test]
    fn dot_goes_top_right_with_a_clear_ring() {
        let (w, h) = (64u32, 64u32);
        let grey = [0x80u8, 0x80, 0x80, 0xFF];
        let icon = Image::new_owned(grey.repeat((w * h) as usize), w, h);
        let out = with_dot(&icon);
        let px = |x: u32, y: u32| out.rgba()[((y * w + x) * 4) as usize..][..4].to_vec();
        assert_eq!(px(w - 15, 14), [0xE5, 0x48, 0x4D, 0xFF], "dot centre");
        assert_eq!(px(w - 15, 30)[3], 0, "ring below the dot is clear");
        assert_eq!(px(10, 50), grey, "rest untouched");
    }

    #[test]
    fn tooltip_says_what_is_overdue_and_next() {
        assert_eq!(tooltip(&[]), "Befikr · all clear today");
        let today = [task("Pay bill", "Overdue", true), task("Call mum", "10:00", false)];
        assert_eq!(tooltip(&today), "Befikr · 1 overdue · next: Call mum 10:00");
        assert_eq!(tooltip(&today[1..]), "Befikr · next: Call mum 10:00");
    }
}
