//! Reminder notifications with Complete and Snooze (and Skip for repeating
//! tasks), like on Android: tap opens the list, Complete and Skip run the same
//! core calls as the app, Snooze opens the snooze sheet. Each notification waits for its answer on its own thread.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use befikr_core::Source;
use notify_rust::{Notification, NotificationResponse, Timeout, Urgency};
use tauri::{AppHandle, Emitter, Manager};

/// Shown notification per task: (notification id, generation). A re-remind
/// replaces the old notification under the same id (Linux), and both waiting
/// threads then hear its answer: only the latest generation acts on it.
static SHOWN: Mutex<BTreeMap<String, (u32, u64)>> = Mutex::new(BTreeMap::new());

fn shown() -> MutexGuard<'static, BTreeMap<String, (u32, u64)>> {
    SHOWN.lock().unwrap_or_else(|e| e.into_inner())
}

fn build(title: &str, repeats: bool) -> Notification {
    let mut n = Notification::new();
    n.appname("Befikr")
        .summary(title)
        .action("default", "Open")
        .action("complete", "Complete")
        .action("snooze", "Snooze");
    if repeats {
        n.action("skip", "Skip");
    }
    // Stays until answered (Windows: reminder toast).
    n.urgency(Urgency::Critical).timeout(Timeout::Never);
    n
}

pub fn show(app: &AppHandle, id: &str, title: &str, repeats: bool) {
    let mut n = build(title, repeats);
    let prev = shown().get(id).copied();
    if let Some((nid, _)) = prev {
        if cfg!(target_os = "linux") {
            n.id(nid);
        }
    }
    // Installed builds have a Start menu entry under the app identifier;
    // dev builds borrow PowerShell's.
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        n.app_id(&app.config().identifier);
    }
    let handle = match n.show() {
        Ok(h) => h,
        Err(e) => return eprintln!("notification failed: {e}"),
    };
    let generation = prev.map_or(0, |(_, g)| g + 1);
    #[cfg(target_os = "linux")]
    let nid = handle.id();
    #[cfg(not(target_os = "linux"))]
    let nid = 0;
    shown().insert(id.to_string(), (nid, generation));

    let (app, id) = (app.clone(), id.to_string());
    std::thread::spawn(move || {
        let _ = handle.wait_for_response(|r: &NotificationResponse| {
            // Linux: answers to a replaced (older) notification arrive here too.
            if cfg!(target_os = "linux") && shown().get(&id).map(|s| s.1) != Some(generation) {
                return;
            }
            // Answered: it's gone, nothing to close later.
            shown().remove(&id);
            match r {
                NotificationResponse::Action(a) if a == "complete" => act(&app, &id, false),
                NotificationResponse::Action(a) if a == "skip" => act(&app, &id, true),
                NotificationResponse::Action(a) if a == "snooze" => super::open_route(&app, &format!("/?snooze={id}")),
                NotificationResponse::Closed(_) => dismissed(&app, &id),
                _ => super::open_route(&app, "/"),
            }
        });
    });
}

/// The task was completed, snoozed or deleted elsewhere: take its
/// notification down (Linux; Windows toasts stay, and acting on them is safe).
pub fn close(id: &str) {
    let Some((nid, _)) = shown().remove(id) else { return };
    #[cfg(target_os = "linux")]
    {
        // Closing needs a handle: replace under the same id, then close it.
        let mut n = Notification::new();
        if let Ok(h) = n.id(nid).appname("Befikr").summary("").show() {
            h.close();
        }
    }
    let _ = nid;
}

/// Complete, or skip to the next occurrence.
fn act(app: &AppHandle, id: &str, skip: bool) {
    let db = app.state::<crate::Db>();
    let change = db.store.lock().map_err(|e| e.to_string()).and_then(|mut s| {
        let r = if skip { s.skip_task(id, Source::Native) } else { s.complete_task(id, Source::Native) };
        r.map_err(|e| e.to_string())
    });
    match change {
        Ok(c) => {
            crate::apply(app, &db, &c.alarm);
            let _ = app.emit("tasks-changed", ());
        }
        Err(e) => eprintln!("{} from notification: {e}", if skip { "skip" } else { "complete" }),
    }
}

fn dismissed(app: &AppHandle, id: &str) {
    if let Ok(mut s) = app.state::<crate::Db>().store.lock() {
        let _ = s.on_dismissed(id, Source::Native);
    }
}
