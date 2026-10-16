//! P2P sync glue: the app side of `befikr-sync` (store access, alarms,
//! events) and the commands behind Settings → Sync.

use befikr_core::{Conflict, MergeOutcome, Store, SyncMode};
use befikr_sync::{Host, Status, SyncService};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::{apply, err, Db, Res};

pub(crate) struct AppHost(AppHandle);

impl Host for AppHost {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> befikr_sync::Result<R> {
        let db = self.0.state::<Db>();
        let mut s = db.store.lock().map_err(err)?;
        f(&mut s).map_err(err)
    }

    fn merged(&self, outcome: MergeOutcome) {
        let db = self.0.state::<Db>();
        for alarm in &outcome.alarms {
            apply(&self.0, &db, alarm);
        }
        let _ = self.0.emit("tasks-changed", ());
    }

    fn status_changed(&self) {
        let _ = self.0.emit("sync-changed", ());
    }
}

pub(crate) type Sync = SyncService<AppHost>;

/// The app's own sync runs in this process: background sync (JNI
/// `syncOnce`) must not start a second endpoint with the same key.
pub(crate) static APP_SYNC: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Device key: a file next to the database, not in it (backups must not
/// carry it to another device).
pub(crate) fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let path = app.path().app_data_dir()?.join("sync.key");
    let key: [u8; 32] = match std::fs::read(&path).ok().and_then(|b| b.try_into().ok()) {
        Some(k) => k,
        None => {
            let k = befikr_sync::new_secret_key();
            std::fs::write(&path, k)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
            }
            k
        }
    };
    app.manage(SyncService::new(AppHost(app.clone()), key));
    APP_SYNC.store(true, std::sync::atomic::Ordering::SeqCst);
    let app = app.clone();
    tauri::async_runtime::spawn(async move { restart(&app).await });
    Ok(())
}

/// Bring the endpoint in line with the stored mode.
async fn restart(app: &AppHandle) {
    if let Err(e) = app.state::<Sync>().apply_mode().await {
        eprintln!("sync: {e}");
    }
}

#[tauri::command]
pub(crate) fn sync_status(sync: State<'_, Sync>) -> Status {
    sync.status()
}

#[tauri::command]
pub(crate) async fn set_sync_mode(app: AppHandle, db: State<'_, Db>, mode: SyncMode) -> Res<()> {
    db.store.lock().map_err(err)?.set_sync_mode(mode).map_err(err)?;
    restart(&app).await;
    Ok(())
}

#[tauri::command]
pub(crate) fn set_device_name(sync: State<'_, Sync>, db: State<'_, Db>, name: String) -> Res<()> {
    db.store.lock().map_err(err)?.set_device_name(&name).map_err(err)?;
    sync.stop_pairing(); // an open link carries the old name
    Ok(())
}

#[tauri::command]
pub(crate) fn start_pairing(sync: State<'_, Sync>) -> Res<()> {
    sync.start_pairing()
}

#[tauri::command]
pub(crate) fn stop_pairing(sync: State<'_, Sync>) {
    sync.stop_pairing()
}

#[tauri::command]
pub(crate) async fn pair_with_link(sync: State<'_, Sync>, link: String) -> Res<()> {
    sync.pair_with_link(&link).await.map(|_| ())
}

#[tauri::command]
pub(crate) async fn unpair(sync: State<'_, Sync>, endpoint: String) -> Res<()> {
    sync.unpair(&endpoint).await
}

#[tauri::command]
pub(crate) fn conflicts(db: State<'_, Db>) -> Res<Vec<Conflict>> {
    db.store.lock().map_err(err)?.conflicts().map_err(err)
}

/// `use_mine`: write this device's lost value back (it then syncs out).
#[tauri::command]
pub(crate) fn resolve_conflict(app: AppHandle, sync: State<'_, Sync>, db: State<'_, Db>, id: String, use_mine: bool) -> Res<()> {
    let change = db.store.lock().map_err(err)?.resolve_conflict(&id, use_mine).map_err(err)?;
    if let Some(c) = change {
        apply(&app, &db, &c.alarm);
    }
    let _ = app.emit("tasks-changed", ());
    let _ = app.emit("sync-changed", ());
    sync.nudge();
    Ok(())
}

#[tauri::command]
pub(crate) async fn sync_now(sync: State<'_, Sync>) -> Res<()> {
    sync.sync_now().await;
    match sync.status().last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
