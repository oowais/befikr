mod db;
mod jni;

use std::path::PathBuf;

use tauri::{Manager, State};
use tauri_plugin_reminders::RemindersExt;

struct DbPath(String);

type Res<T> = Result<T, String>;

fn conn(db: &State<DbPath>) -> Res<rusqlite::Connection> {
    db::open(&db.0)
}

fn arm(app: &tauri::AppHandle, db: &State<DbPath>, task: &db::Task) -> Res<()> {
    match task.alarm() {
        Some(a) => app.reminders().schedule(&db.0, &a),
        None => app.reminders().cancel(&task.id),
    }
}

#[tauri::command]
fn create_task(app: tauri::AppHandle, db: State<DbPath>, title: String, in_seconds: i64, mode: String) -> Res<db::Task> {
    let task = db::create_task(&conn(&db)?, &title, db::now_ms() + in_seconds * 1000, &mode)?;
    arm(&app, &db, &task)?;
    Ok(task)
}

#[tauri::command]
fn snooze_task(app: tauri::AppHandle, db: State<DbPath>, id: String, in_seconds: i64) -> Res<db::Task> {
    let task = db::snooze(&conn(&db)?, &id, db::now_ms() + in_seconds * 1000)?;
    arm(&app, &db, &task)?;
    Ok(task)
}

#[tauri::command]
fn complete_task(app: tauri::AppHandle, db: State<DbPath>, id: String) -> Res<()> {
    db::on_event(&conn(&db)?, &id, "completed", db::now_ms(), "app")?;
    app.reminders().cancel(&id)
}

#[tauri::command]
fn list_tasks(db: State<DbPath>) -> Res<Vec<db::Task>> {
    db::list_tasks(&conn(&db)?)
}

#[tauri::command]
fn list_events(db: State<DbPath>) -> Res<Vec<db::Event>> {
    db::list_events(&conn(&db)?, 100)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_reminders::init())
        .setup(|app| {
            let dir: PathBuf = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let path = dir.join("spike.db").to_string_lossy().into_owned();
            let c = db::open(&path)?;
            // reschedule_all on start: same path the boot receiver takes.
            for alarm in db::pending_alarms(&c, db::now_ms())? {
                if let Err(e) = app.reminders().schedule(&path, &alarm) {
                    eprintln!("schedule failed: {e}");
                }
            }
            app.manage(DbPath(path));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_task,
            snooze_task,
            complete_task,
            list_tasks,
            list_events
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
