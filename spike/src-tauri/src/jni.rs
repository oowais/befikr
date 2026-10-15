//! JNI entry points for `com.befikr.reminders.Native`. Called from broadcast
//! receivers, possibly in a cold process with no Tauri app running.

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;

use crate::db;

fn arg(env: &mut JNIEnv, s: &JString) -> Result<String, String> {
    env.get_string(s).map(Into::into).map_err(|e| e.to_string())
}

/// Never unwind across FFI: errors and panics become `{"error": ...}`.
fn respond(env: &mut JNIEnv, f: impl FnOnce(&mut JNIEnv) -> Result<String, String>) -> jstring {
    let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(env)))
        .unwrap_or_else(|_| Err("panic in rust".into()))
        .unwrap_or_else(|e| serde_json::json!({ "error": e }).to_string());
    env.new_string(out).map(|s| s.into_raw()).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_onEvent<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
    task_id: JString<'l>,
    kind: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let (path, id, kind) = (arg(env, &db_path)?, arg(env, &task_id)?, arg(env, &kind)?);
        let conn = db::open(&path)?;
        let out = db::on_event(&conn, &id, &kind, db::now_ms(), "native")?;
        serde_json::to_string(&out).map_err(|e| e.to_string())
    })
}

#[no_mangle]
pub extern "system" fn Java_com_befikr_reminders_Native_pendingAlarms<'l>(
    mut env: JNIEnv<'l>,
    _class: JClass<'l>,
    db_path: JString<'l>,
) -> jstring {
    respond(&mut env, |env| {
        let conn = db::open(&arg(env, &db_path)?)?;
        let alarms = db::pending_alarms(&conn, db::now_ms())?;
        serde_json::to_string(&alarms).map_err(|e| e.to_string())
    })
}
