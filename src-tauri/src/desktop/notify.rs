//! Reminder notifications with Complete and Snooze (and Skip for repeating
//! tasks), like on Android: tap opens the list, Complete and Skip run the same
//! core calls as the app, Snooze opens the snooze sheet.
//!
//! Linux talks D-Bus itself: one connection shows every notification and one
//! thread reads its answers. notify-rust starts a new message stream for each
//! message it reads, so a signal right after another is lost: Noctalia sends
//! `ActivationToken`, `ActionInvoked` and `NotificationClosed` at once, and
//! "Snooze" arrived as a plain dismiss (#22). Some servers also send the
//! answer only to the connection that showed the notification.
//! Windows uses notify-rust, each toast waiting for its answer on its own thread.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard};

use befikr_core::Source;
use tauri::{AppHandle, Emitter, Manager};

/// What the user did with a notification.
enum Answer {
    Open,
    Action(String),
    /// Swiped or closed by the user (re-remind).
    Dismissed,
    /// Expired, or closed by us.
    Gone,
}

/// Run the answer for task `id`.
fn answer(app: &AppHandle, id: &str, a: Answer) {
    match a {
        Answer::Action(a) if a == "complete" => act(app, id, false),
        Answer::Action(a) if a == "skip" => act(app, id, true),
        Answer::Action(a) if a == "snooze" => super::open_route(app, &format!("/?snooze={id}")),
        Answer::Dismissed => dismissed(app, id),
        Answer::Gone => {}
        Answer::Open | Answer::Action(_) => super::open_route(app, "/"),
    }
}

/// Buttons, as (action key, label).
fn actions(repeats: bool) -> Vec<(&'static str, &'static str)> {
    let mut a = vec![("default", "Open"), ("complete", "Complete"), ("snooze", "Snooze")];
    if repeats {
        a.push(("skip", "Skip"));
    }
    a
}

#[cfg(target_os = "linux")]
pub use linux::{close, show};
#[cfg(windows)]
pub use windows::{close, show};

#[cfg(target_os = "linux")]
mod linux {
    use std::collections::HashMap;
    use std::sync::OnceLock;

    use tauri::AppHandle;
    use zbus::blocking::{Connection, MessageIterator};
    use zbus::zvariant::Value;
    use zbus::MatchRule;

    use super::{answer, due_text, shown, Answer};

    const DEST: &str = "org.freedesktop.Notifications";
    const PATH: &str = "/org/freedesktop/Notifications";

    /// The one connection; its listener thread starts with it.
    static BUS: OnceLock<Option<Connection>> = OnceLock::new();

    fn bus(app: &AppHandle) -> Option<&'static Connection> {
        BUS.get_or_init(|| match connect(app) {
            Ok(c) => Some(c),
            Err(e) => {
                eprintln!("notifications unavailable: {e}");
                None
            }
        })
        .as_ref()
    }

    fn connect(app: &AppHandle) -> zbus::Result<Connection> {
        let conn = Connection::session()?;
        let rule = MatchRule::builder()
            .msg_type(zbus::message::Type::Signal)
            .interface(DEST)?
            .path(PATH)?
            .build();
        // Subscribed before the first notification, so no answer is missed.
        let signals = MessageIterator::for_match_rule(rule, &conn, Some(64))?;
        let app = app.clone();
        std::thread::Builder::new()
            .name("notifications".into())
            .spawn(move || listen(&app, signals))
            .map_err(|e| zbus::Error::Failure(e.to_string()))?;
        Ok(conn)
    }

    fn listen(app: &AppHandle, signals: MessageIterator) {
        for msg in signals.flatten() {
            let header = msg.header();
            let (nid, a) = match header.member().map(|m| m.as_str()) {
                Some("ActionInvoked") => match msg.body().deserialize::<(u32, String)>() {
                    Ok((nid, a)) if a == "default" => (nid, Answer::Open),
                    Ok((nid, a)) => (nid, Answer::Action(a)),
                    Err(_) => continue,
                },
                Some("NotificationClosed") => match msg.body().deserialize::<(u32, u32)>() {
                    // 2 = dismissed by the user; 1 expired, 3 closed by us.
                    Ok((nid, 2)) => (nid, Answer::Dismissed),
                    Ok((nid, _)) => (nid, Answer::Gone),
                    Err(_) => continue,
                },
                _ => continue,
            };
            // Answered: it's gone, nothing to close later. The close that
            // follows an action finds nothing left and does nothing.
            let id = {
                let mut s = shown();
                let Some(id) = s.iter().find(|(_, v)| v.0 == nid).map(|(k, _)| k.clone()) else { continue };
                s.remove(&id);
                id
            };
            answer(app, &id, a);
        }
    }

    /// `due_at`: an early reminder ("30 min before"); the text says when it's due.
    /// A re-remind replaces the task's notification still on screen.
    pub fn show(app: &AppHandle, id: &str, title: &str, repeats: bool, due_at: Option<i64>) {
        let Some(conn) = bus(app) else { return };
        let replaces = shown().get(id).map_or(0, |s| s.0);
        let body = due_at.map(due_text).unwrap_or_default();
        let actions: Vec<&str> = super::actions(repeats).into_iter().flat_map(|(k, l)| [k, l]).collect();
        // Critical: stays until answered.
        let hints = HashMap::from([("urgency", Value::U8(2))]);
        let reply = conn.call_method(
            Some(DEST),
            PATH,
            Some(DEST),
            "Notify",
            &("Befikr", replaces, "", title, body, actions, hints, 0i32),
        );
        match reply.and_then(|r| r.body().deserialize::<u32>()) {
            Ok(nid) => {
                shown().insert(id.to_string(), (nid, 0));
            }
            Err(e) => eprintln!("notification failed: {e}"),
        }
    }

    /// The task was completed, snoozed or deleted elsewhere: take its
    /// notification down.
    pub fn close(id: &str) {
        // Removed first: the "closed" signal then belongs to no task.
        let Some((nid, _)) = shown().remove(id) else { return };
        let Some(Some(conn)) = BUS.get() else { return };
        let _ = conn.call_method(Some(DEST), PATH, Some(DEST), "CloseNotification", &(nid,));
    }
}

#[cfg(windows)]
mod windows {
    use notify_rust::{CloseReason, Notification, NotificationResponse, Timeout, Urgency};
    use tauri::{AppHandle, Manager};

    use super::{answer, due_text, shown, Answer};

    /// `due_at`: an early reminder ("30 min before"); the text says when it's due.
    pub fn show(app: &AppHandle, id: &str, title: &str, repeats: bool, due_at: Option<i64>) {
        let mut n = Notification::new();
        n.appname("Befikr").summary(title);
        if let Some(at) = due_at {
            n.body(&due_text(at));
        }
        for (k, l) in super::actions(repeats) {
            n.action(k, l);
        }
        // Stays until answered (reminder toast).
        n.urgency(Urgency::Critical).timeout(Timeout::Never);
        // Installed builds have a Start menu entry under the app identifier;
        // dev builds borrow PowerShell's.
        if !cfg!(debug_assertions) {
            n.app_id(&app.config().identifier);
        }
        let handle = match n.show() {
            Ok(h) => h,
            Err(e) => return eprintln!("notification failed: {e}"),
        };
        // A newer reminder for the task replaces this one's turn to answer.
        let generation = shown().get(id).map_or(0, |s| s.1 + 1);
        shown().insert(id.to_string(), (0, generation));

        let (app, id) = (app.clone(), id.to_string());
        std::thread::spawn(move || {
            let _ = handle.wait_for_response(|r: &NotificationResponse| {
                {
                    let mut s = shown();
                    if s.get(&id).map(|s| s.1) != Some(generation) {
                        return;
                    }
                    // Answered: it's gone, nothing to close later.
                    s.remove(&id);
                }
                let a = match r {
                    NotificationResponse::Action(a) => Answer::Action(a.clone()),
                    // Only the user's own close counts as a swipe (re-remind).
                    NotificationResponse::Closed(CloseReason::Dismissed) => Answer::Dismissed,
                    NotificationResponse::Closed(_) => Answer::Gone,
                    _ => Answer::Open,
                };
                answer(&app, &id, a);
            });
        });
    }

    /// Windows toasts stay; acting on one later is safe.
    pub fn close(id: &str) {
        shown().remove(id);
    }
}

/// Shown notification per task: (notification id on Linux, generation on Windows).
static SHOWN: Mutex<BTreeMap<String, (u32, u64)>> = Mutex::new(BTreeMap::new());

fn shown() -> MutexGuard<'static, BTreeMap<String, (u32, u64)>> {
    SHOWN.lock().unwrap_or_else(|e| e.into_inner())
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

/// Swiped away: arm the re-remind, if it's on.
fn dismissed(app: &AppHandle, id: &str) {
    let db = app.state::<crate::Db>();
    let alarm = db.store.lock().ok().and_then(|mut s| s.on_dismissed(id, Source::Native).ok().flatten());
    if let Some(a) = alarm {
        app.state::<super::Scheduler>().schedule(a);
        super::refresh_tray();
    }
}

/// "Due at 18:00", "Due tomorrow at 09:00", "Due Mon 26 Oct at 09:00".
fn due_text(at: i64) -> String {
    use chrono::{Local, TimeZone};
    let Some(due) = Local.timestamp_millis_opt(at).single() else { return String::new() };
    let today = Local::now().date_naive();
    let day = match (due.date_naive() - today).num_days() {
        0 => String::new(),
        1 => "tomorrow ".into(),
        _ => format!("{} ", due.format("%a %-d %b")),
    };
    format!("Due {day}at {}", due.format("%H:%M"))
}
