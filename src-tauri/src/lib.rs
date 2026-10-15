mod jni;

use std::sync::Mutex;

use befikr_core::{
    urgency, AlarmChange, Change, Completion, ListItem, NewTask, Settings, Source, Store, Task, TaskPatch, Urgency,
};
use serde::Serialize;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_reminders::RemindersExt;

struct Db {
    store: Mutex<Store>,
    path: String,
}

type Res<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

/// Hand the core's alarm decision to the platform scheduler. Scheduling
/// failures (desktop has no scheduler yet) must not fail the user's write.
fn apply(app: &AppHandle, db: &Db, alarm: &AlarmChange) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_reminders::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("befikr.db").to_string_lossy().into_owned();
            let store = Store::open(&path)?;
            if let Err(e) = app.reminders().configure(&path) {
                eprintln!("reminders configure failed: {e}");
            }
            let db = Db { store: Mutex::new(store), path };
            // Re-arm everything on start (same as the boot receiver).
            let pending = db.store.lock().map_err(err)?.pending_alarms()?;
            for a in pending {
                apply(app.handle(), &db, &AlarmChange::Schedule(a));
            }
            app.manage(db);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_open,
            get_task,
            list_subtasks,
            list_completed,
            add_task,
            update_task,
            complete_task,
            reopen_task,
            snooze_task,
            delete_task,
            restore_task,
            get_settings,
            set_time_word,
            remove_time_word,
            set_reremind_minutes,
            import_backup,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
