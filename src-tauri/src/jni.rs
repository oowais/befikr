//! JNI entry points for `com.befikr.reminders.Native`. Called from broadcast
//! receivers, possibly in a cold process with no Tauri app or webview.

use befikr_core::{Source, Store};
use jni::objects::{JClass, JString};
use jni::sys::jstring;
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
            "dismiss" => {
                s.on_dismissed(&id, Source::Native).map_err(|e| e.to_string())?;
                Ok(json!({ "op": "none" }))
            }
            other => Err(format!("unknown action: {other}")),
        }
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
