//! JNI entry points for `com.befikr.reminders.Native`. Called from broadcast
//! receivers, possibly in a cold process with no Tauri app or webview.

use std::sync::{Arc, Mutex};

use befikr_core::{AlarmChange, MergeOutcome, Source, Store};
use jni::objects::{JClass, JString};
use jni::sys::{jlong, jstring};
use jni::JNIEnv;
use serde_json::json;

fn arg(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(Into::into).map_err(|e| e.to_string())
}

/// Never unwind across FFI: errors and panics become `{"error": ...}`.
fn respond(env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv) -> Result<serde_json::Value, String>) -> jstring {
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(env)))
        .unwrap_or_else(|_| Err("panic in rust".into()))
        .unwrap_or_else(|e| json!({ "error": e }));
    env.new_string(out.to_string()).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

fn store(path: &str) -> Result<Store, String> {
    Store::open(path).map_err(|e| e.to_string())
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_onFired<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    task_id: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id) = (arg(env, &db_path)?, arg(env, &task_id)?);
        let out = store(&path)?.on_fired(&id, Source::Native).map_err(|e| e.to_string())?;
        serde_json::to_value(out).map_err(|e| e.to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_onAction<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    task_id: JString<'l>,
    action: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id, action) = (arg(env, &db_path)?, arg(env, &task_id)?, arg(env, &action)?);
        let mut s = store(&path)?;
        match action.as_str() {
            "complete" => {
                let change = s.complete_task(&id, Source::Native).map_err(|e| e.to_string())?;
                serde_json::to_value(change.alarm).map_err(|e| e.to_string())
            }
            "skip" => {
                let change = s.skip_task(&id, Source::Native).map_err(|e| e.to_string())?;
                serde_json::to_value(change.alarm).map_err(|e| e.to_string())
            }
            "dismiss" => {
                s.on_dismissed(&id, Source::Native).map_err(|e| e.to_string())?;
                Ok(json!({ "op": "none" }))
            }
            other => Err(format!("unknown action: {other}")),
        }
    })
}

/// Snooze overlay: quick picks for now in `tz`. → `[{"label","atMs"}]`
#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_snoozePicks<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let now = chrono::Utc::now().timestamp_millis();
        let picks = befikr_core::snooze_picks(now, &arg(env, &tz)?).map_err(|e| e.to_string())?;
        serde_json::to_value(picks).map_err(|e| e.to_string())
    })
}

/// Snooze overlay: snooze until `until_ms`. → AlarmChange JSON
#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_snooze<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    task_id: JString<'l>,
    until_ms: jlong,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id) = (arg(env, &db_path)?, arg(env, &task_id)?);
        let change = store(&path)?.snooze_task(&id, until_ms, Source::Native).map_err(|e| e.to_string())?;
        serde_json::to_value(change.alarm).map_err(|e| e.to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_pendingAlarms<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let alarms = store(&arg(env, &db_path)?)?.pending_alarms().map_err(|e| e.to_string())?;
        serde_json::to_value(alarms).map_err(|e| e.to_string())
    })
}

/// Morning digest for now in `tz`. → `{"digest": Digest | null}`
#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_digest<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, tz) = (arg(env, &db_path)?, arg(env, &tz)?);
        let digest = store(&path)?.digest(&tz).map_err(|e| e.to_string())?;
        Ok(json!({ "digest": digest }))
    })
}

/// When the next morning digest fires in `tz`. → `{"atMs": i64 | null}`
#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_nextDigestAt<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    tz: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, tz) = (arg(env, &db_path)?, arg(env, &tz)?);
        let at = store(&path)?.next_digest_at(&tz).map_err(|e| e.to_string())?;
        Ok(json!({ "atMs": at }))
    })
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_exportTo<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    out_path: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (db, out) = (arg(env, &db_path)?, arg(env, &out_path)?);
        store(&db)?.export_to(&out).map_err(|e| e.to_string())?;
        Ok(json!({}))
    })
}

/// Sync side of a background run: the store, and the alarm changes merges
/// asked for (Kotlin applies them).
#[derive(Clone)]
struct BackgroundHost(Arc<(Mutex<Store>, Mutex<Vec<AlarmChange>>)>);

impl befikr_sync::Host for BackgroundHost {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> befikr_sync::Result<R> {
        let mut s = self.0 .0.lock().map_err(|e| e.to_string())?;
        f(&mut s).map_err(|e| e.to_string())
    }
    fn merged(&self, outcome: MergeOutcome) {
        if let Ok(mut alarms) = self.0 .1.lock() {
            alarms.extend(outcome.alarms);
        }
    }
    fn status_changed(&self) {}
}

/// Background sync (WorkManager, app closed): one round with every paired
/// device. → `{"alarms": [AlarmChange], "skipped"?: why, "problem"?: error}`.
#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_syncOnce<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    key_path: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (db, key_path) = (arg(env, &db_path)?, arg(env, &key_path)?);
        if crate::sync::APP_SYNC.load(std::sync::atomic::Ordering::SeqCst) {
            return Ok(json!({ "alarms": [], "skipped": "app is syncing" }));
        }
        let s = store(&db)?;
        let off = s.sync_settings().map_err(|e| e.to_string())?.mode == befikr_core::SyncMode::Off;
        if off || s.peers().map_err(|e| e.to_string())?.is_empty() {
            return Ok(json!({ "alarms": [], "skipped": "sync off or no paired devices" }));
        }
        let Some(key) = std::fs::read(&key_path).ok().and_then(|b| <[u8; 32]>::try_from(b).ok()) else {
            return Ok(json!({ "alarms": [], "skipped": "no device key yet" }));
        };
        let host = BackgroundHost(Arc::new((Mutex::new(s), Mutex::new(Vec::new()))));
        let svc = befikr_sync::SyncService::new(host.clone(), key);
        let r = tauri::async_runtime::block_on(async {
            tokio_timeout(std::time::Duration::from_secs(90), svc.sync_once()).await
        });
        let alarms = std::mem::take(&mut *host.0 .1.lock().map_err(|e| e.to_string())?);
        let mut out = json!({ "alarms": alarms });
        if let Err(e) = r {
            out["problem"] = e.into();
        }
        Ok(out)
    })
}

/// Bound a background run (a peer that never answers must not hold the
/// worker for its whole 10 minutes).
async fn tokio_timeout<T>(
    d: std::time::Duration,
    f: impl std::future::Future<Output = befikr_sync::Result<T>>,
) -> befikr_sync::Result<T> {
    match tokio::time::timeout(d, f).await {
        Ok(r) => r,
        Err(_) => Err("background sync timed out".into()),
    }
}
