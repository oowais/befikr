//! P2P sync glue: the app side of `befikr-sync` (store access, alarms,
//! events) and the commands behind Settings → Sync.

use befikr_core::{MergeOutcome, Store, SyncMode};
use befikr_sync::{Host, Status, SyncService};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_reminders::RemindersExt;

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
    let app = app.clone();
    tauri::async_runtime::spawn(async move { restart(&app).await });
    Ok(())
}

/// Bring the endpoint in line with the stored mode.
async fn restart(app: &AppHandle) {
    let sync = app.state::<Sync>();
    let r = sync.apply_mode().await;
    let on = sync.status().running;
    if let Err(e) = app.reminders().set_multicast(on) {
        eprintln!("multicast lock: {e}");
    }
    if let Err(e) = r {
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
    sync.stop_pairing(); // re-advertises the new name
    Ok(())
}

#[tauri::command]
pub(crate) fn start_pairing(sync: State<'_, Sync>) -> Res<String> {
    sync.start_pairing()
}

#[tauri::command]
pub(crate) fn stop_pairing(sync: State<'_, Sync>) {
    sync.stop_pairing()
}

#[tauri::command]
pub(crate) async fn pair_with(sync: State<'_, Sync>, endpoint: String, code: String) -> Res<()> {
    sync.pair_with(&endpoint, &code).await.map(|_| ())
}

#[tauri::command]
pub(crate) async fn pair_with_address(sync: State<'_, Sync>, address: String, code: String) -> Res<()> {
    sync.pair_with_address(&address, &code).await.map(|_| ())
}

#[tauri::command]
pub(crate) async fn unpair(sync: State<'_, Sync>, endpoint: String) -> Res<()> {
    sync.unpair(&endpoint).await
}

#[tauri::command]
pub(crate) async fn rescan_sync(sync: State<'_, Sync>) -> Res<()> {
    sync.rescan().await
}

#[tauri::command]
pub(crate) async fn sync_now(sync: State<'_, Sync>) -> Res<()> {
    sync.sync_now().await;
    match sync.status().last_error {
        Some(e) => Err(e),
        None => Ok(()),
    }
}
