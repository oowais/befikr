#[cfg(desktop)]
mod desktop;
mod jni;
mod sync;

use std::sync::Mutex;

use befikr_core::{
    urgency, AlarmChange, Change, CompletedGroup, Completion, ListItem, NewTask, Settings, ShoppingItem, ShoppingList, Source, Staple, Stats, Store, Task, TaskPatch, Urgency,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_reminders::RemindersExt;

pub(crate) struct Db {
    pub(crate) store: Mutex<Store>,
    /// For the Android alarms (receivers open the DB themselves).
    #[cfg_attr(desktop, allow(dead_code))]
    path: String,
}

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Hand the core's alarm decision to the platform scheduler: the in-process
/// one on desktop, AlarmManager on Android. Scheduling failures must not fail
/// the user's write.
pub(crate) fn apply(app: &AppHandle, db: &Db, alarm: &AlarmChange) {
    #[cfg(desktop)]
    {
        let _ = db;
        let scheduler = app.state::<desktop::Scheduler>();
        // A shown reminder is stale once its task changes (done, snoozed, moved).
        match alarm {
            AlarmChange::Schedule(a) => {
                desktop::close_notification(&a.task_id);
                scheduler.schedule(a.clone());
            }
            AlarmChange::Cancel { task_id } => {
                desktop::close_notification(task_id);
                scheduler.cancel(task_id);
            }
        }
    }
    #[cfg(mobile)]
    apply_mobile(app, db, alarm);
}

#[cfg(mobile)]
fn apply_mobile(app: &AppHandle, db: &Db, alarm: &AlarmChange) {
    let r = match alarm {
        AlarmChange::Schedule(a) => app.reminders().schedule(
            &db.path,
            &tauri_plugin_reminders::Alarm { id: a.task_id.clone(), title: a.title.clone(), at_ms: a.at_ms },
        ),
        AlarmChange::Cancel { task_id } => app.reminders().cancel(task_id),
    };
    if let Err(e) = r {
        eprintln!("reminder scheduling skipped: {e}");
    }
}

fn write(app: &AppHandle, db: &Db, f: impl FnOnce(&mut Store) -> befikr_core::Result<Change>) -> Res<Task> {
    let change = f(&mut *db.store.lock().map_err(err)?).map_err(err)?;
    apply(app, db, &change.alarm);
    Ok(change.task)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Row {
    #[serde(flatten)]
    item: ListItem,
    urgency: Urgency,
}

/// Main list. `tz` is the device zone, for "today".
#[tauri::command]
fn list_open(db: State<Db>, tz: String) -> Res<Vec<Row>> {
    let s = db.store.lock().map_err(err)?;
    let tz: chrono_tz::Tz = tz.parse().unwrap_or(chrono_tz::UTC);
    let now = s.now();
    Ok(s.list_open()
        .map_err(err)?
        .into_iter()
        .map(|item| Row { urgency: urgency(item.task.effective_due(), now, tz), item })
        .collect())
}

#[tauri::command]
fn get_task(db: State<Db>, id: String) -> Res<Task> {
    db.store.lock().map_err(err)?.get_task(&id).map_err(err)
}

#[tauri::command]
fn list_subtasks(db: State<Db>, id: String) -> Res<Vec<Task>> {
    db.store.lock().map_err(err)?.list_subtasks(&id).map_err(err)
}

#[tauri::command]
fn list_completed(db: State<Db>, limit: u32) -> Res<Vec<Completion>> {
    db.store.lock().map_err(err)?.list_completed(limit).map_err(err)
}

#[tauri::command]
fn completed_groups(db: State<Db>) -> Res<Vec<CompletedGroup>> {
    db.store.lock().map_err(err)?.completed_groups().map_err(err)
}

#[tauri::command]
fn task_history(db: State<Db>, id: String) -> Res<Vec<Completion>> {
    db.store.lock().map_err(err)?.task_history(&id).map_err(err)
}

#[tauri::command]
fn stats(db: State<Db>, since: Option<i64>, week_starts: Vec<i64>) -> Res<Stats> {
    db.store.lock().map_err(err)?.stats(since, &week_starts).map_err(err)
}

#[tauri::command]
fn add_task(app: AppHandle, db: State<Db>, task: NewTask) -> Res<Task> {
    write(&app, &db, |s| s.add_task(task))
}

#[tauri::command]
fn update_task(app: AppHandle, db: State<Db>, id: String, patch: TaskPatch) -> Res<Task> {
    write(&app, &db, |s| s.update_task(&id, patch))
}

#[tauri::command]
fn complete_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.complete_task(&id, Source::App))
}

#[tauri::command]
fn reopen_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.reopen_task(&id))
}

#[tauri::command]
fn skip_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.skip_task(&id, Source::App))
}

#[tauri::command]
fn retire_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.retire_task(&id))
}

#[tauri::command]
fn snooze_task(app: AppHandle, db: State<Db>, id: String, until: i64) -> Res<Task> {
    write(&app, &db, |s| s.snooze_task(&id, until, Source::App))
}

#[tauri::command]
fn delete_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.delete_task(&id))
}

#[tauri::command]
fn restore_task(app: AppHandle, db: State<Db>, id: String) -> Res<Task> {
    write(&app, &db, |s| s.restore_task(&id))
}

#[tauri::command]
fn shopping_list(db: State<Db>) -> Res<ShoppingList> {
    db.store.lock().map_err(err)?.shopping_list().map_err(err)
}

#[tauri::command]
fn add_shopping_item(db: State<Db>, name: String) -> Res<ShoppingItem> {
    db.store.lock().map_err(err)?.add_shopping_item(&name).map_err(err)
}

#[tauri::command]
fn set_shopping_bought(db: State<Db>, id: String, bought: bool) -> Res<ShoppingItem> {
    db.store.lock().map_err(err)?.set_shopping_bought(&id, bought).map_err(err)
}

#[tauri::command]
fn set_shopping_deleted(db: State<Db>, id: String, deleted: bool) -> Res<()> {
    db.store.lock().map_err(err)?.set_shopping_deleted(&id, deleted).map_err(err)
}

#[tauri::command]
fn shopping_suggestions(db: State<Db>) -> Res<Vec<String>> {
    db.store.lock().map_err(err)?.shopping_suggestions(12).map_err(err)
}

#[tauri::command]
fn add_staple(db: State<Db>, name: String) -> Res<Staple> {
    db.store.lock().map_err(err)?.add_staple(&name).map_err(err)
}

#[tauri::command]
fn remove_staple(db: State<Db>, id: String) -> Res<()> {
    db.store.lock().map_err(err)?.remove_staple(&id).map_err(err)
}

#[tauri::command]
fn get_settings(db: State<Db>) -> Res<Settings> {
    db.store.lock().map_err(err)?.settings().map_err(err)
}

#[tauri::command]
fn set_time_word(db: State<Db>, word: String, time: String) -> Res<()> {
    db.store.lock().map_err(err)?.set_time_word(&word, &time).map_err(err)
}

#[tauri::command]
fn remove_time_word(db: State<Db>, word: String) -> Res<()> {
    db.store.lock().map_err(err)?.remove_time_word(&word).map_err(err)
}

#[tauri::command]
fn set_reremind_minutes(db: State<Db>, minutes: Vec<i64>) -> Res<()> {
    db.store.lock().map_err(err)?.set_reremind_minutes(minutes).map_err(err)
}

/// Replace all data with a backup file (path from the native file picker).
/// The current data is snapshotted to the cache first; alarms are re-armed.
#[tauri::command]
fn import_backup(app: AppHandle, db: State<Db>, path: String) -> Res<()> {
    let pending = {
        let mut s = db.store.lock().map_err(err)?;
        if let Ok(dir) = app.path().app_cache_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = s.export_to(&dir.join("befikr-before-import.db").to_string_lossy());
        }
        s.import_from(&path).map_err(err)?;
        s.pending_alarms().map_err(err)?
    };
    let _ = std::fs::remove_file(&path);
    for a in pending {
        apply(&app, &db, &AlarmChange::Schedule(a));
    }
    Ok(())
}

// ---- This computer's startup settings (desktop; Android has none).

#[cfg(desktop)]
#[tauri::command]
fn desktop_prefs(app: AppHandle) -> desktop::prefs::DesktopPrefs {
    desktop::prefs::get(&app)
}

#[cfg(desktop)]
#[tauri::command]
fn set_autostart(app: AppHandle, on: bool) -> Res<()> {
    desktop::prefs::set_autostart(&app, on)
}

#[cfg(desktop)]
#[tauri::command]
fn set_start_hidden(app: AppHandle, on: bool) -> Res<()> {
    desktop::prefs::set_start_hidden(&app, on)
}

#[cfg(mobile)]
const DESKTOP_ONLY: &str = "desktop only";

#[cfg(mobile)]
#[tauri::command]
fn desktop_prefs() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn set_autostart() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg(mobile)]
#[tauri::command]
fn set_start_hidden() -> Res<()> {
    Err(DESKTOP_ONLY.into())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Wayland app id / X11 class `com.befikr.app` (matches StartupWMClass in
    // packaging/linux/befikr.desktop). The default, `befikr`, made launchers
    // that also match window titles focus any window titled "befikr" instead.
    #[cfg(target_os = "linux")]
    gtk::glib::set_prgname(Some("com.befikr.app"));
    let builder = tauri::Builder::default();
    // A second launch (e.g. from the menu while in the tray) focuses this one.
    #[cfg(desktop)]
    let builder = builder
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| desktop::show_main(app)))
        .plugin(tauri_plugin_autostart::init(tauri_plugin_autostart::MacosLauncher::LaunchAgent, Some(vec!["--autostart"])))
        .on_window_event(desktop::on_window_event);
    builder
        .plugin(tauri_plugin_reminders::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("befikr.db").to_string_lossy().into_owned();
            let store = Store::open(&path)?;
            if let Err(e) = app.reminders().configure(&path) {
                eprintln!("reminders configure failed: {e}");
            }
            app.manage(Db { store: Mutex::new(store), path });
            #[cfg(desktop)]
            app.manage(desktop::Scheduler::start(app.handle().clone()));
            // Re-arm everything on start (same as the boot receiver).
            let db = app.state::<Db>();
            let pending = db.store.lock().map_err(err)?.pending_alarms()?;
            for a in pending {
                apply(app.handle(), &db, &AlarmChange::Schedule(a));
            }
            #[cfg(desktop)]
            desktop::setup(app.handle());
            sync::setup(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_open,
            get_task,
            list_subtasks,
            list_completed,
            stats,
            add_task,
            update_task,
            complete_task,
            reopen_task,
            snooze_task,
            skip_task,
            retire_task,
            completed_groups,
            task_history,
            delete_task,
            restore_task,
            shopping_list,
            add_shopping_item,
            set_shopping_bought,
            set_shopping_deleted,
            shopping_suggestions,
            add_staple,
            remove_staple,
            get_settings,
            set_time_word,
            remove_time_word,
            set_reremind_minutes,
            import_backup,
            desktop_prefs,
            set_autostart,
            set_start_hidden,
            sync::sync_status,
            sync::set_sync_mode,
            sync::set_device_name,
            sync::start_pairing,
            sync::stop_pairing,
            sync::pair_with_link,
            sync::unpair,
            sync::sync_now,
            sync::conflicts,
            sync::resolve_conflict,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
