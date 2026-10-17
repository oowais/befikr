//! Morning digest on computers: the scheduler fires it like a reminder (under
//! `DIGEST_ID`); the core decides the time and the text, as on Android.

use befikr_core::{Alarm, Digest, DIGEST_ID};
use notify_rust::{Notification, NotificationResponse, Timeout};
use tauri::{AppHandle, Manager};

/// This computer's IANA zone ("today" for the digest).
fn local_tz() -> String {
    iana_time_zone::get_timezone().unwrap_or_else(|_| "UTC".into())
}

/// Arm the next digest from the current setting, or cancel it when off.
pub fn schedule(app: &AppHandle) {
    let next = match app.state::<crate::Db>().store.lock() {
        Ok(s) => s.next_digest_at(&local_tz()),
        Err(_) => return,
    };
    let scheduler = app.state::<super::Scheduler>();
    match next {
        Ok(Some(at_ms)) => scheduler.schedule(Alarm { task_id: DIGEST_ID.into(), title: String::new(), at_ms }),
        Ok(None) => scheduler.cancel(DIGEST_ID),
        Err(e) => eprintln!("digest: {e}"),
    }
}

/// Digest time: show today's summary (if anything is due), arm tomorrow's.
pub fn fire(app: &AppHandle) {
    let digest = match app.state::<crate::Db>().store.lock() {
        Ok(s) => s.digest(&local_tz()),
        Err(_) => return,
    };
    match digest {
        Ok(Some(d)) => show(app, &d),
        Ok(None) => {}
        Err(e) => eprintln!("digest: {e}"),
    }
    schedule(app);
}

/// A plain notification (no sound urgency): tap opens the list.
fn show(app: &AppHandle, d: &Digest) {
    let mut n = Notification::new();
    n.appname("Befikr").summary(&d.title).body(&d.lines.join("\n")).action("default", "Open").timeout(Timeout::Default);
    #[cfg(windows)]
    if !cfg!(debug_assertions) {
        n.app_id(&app.config().identifier);
    }
    let handle = match n.show() {
        Ok(h) => h,
        Err(e) => return eprintln!("digest notification failed: {e}"),
    };
    let app = app.clone();
    std::thread::spawn(move || {
        // Like reminders: any answer but closing it opens the app.
        let _ = handle.wait_for_response(|r: &NotificationResponse| {
            if !matches!(r, NotificationResponse::Closed(_)) {
                super::open_route(&app, "/");
            }
        });
    });
}
