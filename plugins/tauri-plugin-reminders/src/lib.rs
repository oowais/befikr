//! Native Android reminders: exact alarms, notifications with background
//! actions (Complete, dismiss → re-fire) and boot restore.
//! Rust owns the data; Kotlin calls back into Rust via JNI (see app crate).

use serde::Serialize;
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Alarm {
    pub id: String,
    pub title: String,
    /// Fire time, Unix epoch milliseconds (UTC).
    pub at_ms: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScheduleArgs<'a> {
    db_path: &'a str,
    #[serde(flatten)]
    alarm: &'a Alarm,
    /// Leave a notification already shown for the task (re-arm on app start).
    keep_shown: bool,
}

#[cfg(target_os = "android")]
pub struct Reminders<R: Runtime>(tauri::plugin::PluginHandle<R>);
#[cfg(not(target_os = "android"))]
pub struct Reminders<R: Runtime>(std::marker::PhantomData<fn() -> R>);

impl<R: Runtime> Reminders<R> {
    /// Schedule (or replace) the exact alarm for a task; clears its shown notification.
    pub fn schedule(&self, db_path: &str, alarm: &Alarm) -> Result<(), String> {
        self.arm(ScheduleArgs { db_path, alarm, keep_shown: false })
    }

    /// Re-arm an unchanged alarm (app start, import); a shown notification stays.
    pub fn rearm(&self, db_path: &str, alarm: &Alarm) -> Result<(), String> {
        self.arm(ScheduleArgs { db_path, alarm, keep_shown: true })
    }

    fn arm(&self, args: ScheduleArgs) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<serde_json::Value>("schedule", args)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = args;
            Err("reminders plugin is Android-only".into())
        }
    }
}

impl<R: Runtime> Reminders<R> {
    /// Tell the native side where the database is (receivers, backups).
    pub fn configure(&self, db_path: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<serde_json::Value>("configure", serde_json::json!({ "dbPath": db_path }))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = db_path;
            Ok(())
        }
    }

    /// (Re)arm the morning digest from the current setting (cancels it when off).
    pub fn schedule_digest(&self) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<serde_json::Value>("scheduleDigest", serde_json::json!({}))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            Err("reminders plugin is Android-only".into())
        }
    }

    /// Cancel the task's alarm and any notification shown for it.
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            self.0
                .run_mobile_plugin::<serde_json::Value>("cancel", serde_json::json!({ "id": id }))
                .map(|_| ())
                .map_err(|e| e.to_string())
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = id;
            Err("reminders plugin is Android-only".into())
        }
    }
}

pub trait RemindersExt<R: Runtime> {
    fn reminders(&self) -> &Reminders<R>;
}

impl<R: Runtime, T: Manager<R>> RemindersExt<R> for T {
    fn reminders(&self) -> &Reminders<R> {
        self.state::<Reminders<R>>().inner()
    }
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("reminders")
        .setup(|app, api| {
            #[cfg(target_os = "android")]
            let reminders =
                Reminders(api.register_android_plugin("com.befikr.reminders", "RemindersPlugin")?);
            #[cfg(not(target_os = "android"))]
            let reminders = {
                let _ = api;
                Reminders::<R>(std::marker::PhantomData)
            };
            app.manage(reminders);
            Ok(())
        })
        .build()
}
