//! One thread, one timer: sleeps until the earliest alarm, asks the core
//! whether to show it (re-remind escalation lives there) and re-arms.

use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use befikr_core::{Alarm, Source};
use tauri::{AppHandle, Manager};

/// Wake at least this often: wall-clock time moves on while the machine
/// sleeps, but a timed wait doesn't.
const MAX_SLEEP: Duration = Duration::from_secs(30);

type Alarms = HashMap<String, Alarm>;

#[derive(Clone, Default)]
pub struct Scheduler(Arc<(Mutex<Alarms>, Condvar)>);

fn now_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

impl Scheduler {
    pub fn start(app: AppHandle) -> Self {
        let s = Self::default();
        let me = s.clone();
        std::thread::Builder::new()
            .name("reminders".into())
            .spawn(move || me.run(&app))
            .expect("spawn reminders thread");
        s
    }

    fn alarms(&self) -> MutexGuard<'_, Alarms> {
        self.0 .0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Schedule (or replace) the task's alarm.
    pub fn schedule(&self, alarm: Alarm) {
        self.alarms().insert(alarm.task_id.clone(), alarm);
        self.0 .1.notify_one();
    }

    pub fn cancel(&self, task_id: &str) {
        self.alarms().remove(task_id);
        self.0 .1.notify_one();
    }

    fn run(&self, app: &AppHandle) {
        let mut alarms = self.alarms();
        loop {
            let now = now_ms();
            let due: Vec<Alarm> = alarms.values().filter(|a| a.at_ms <= now).cloned().collect();
            if !due.is_empty() {
                for a in &due {
                    alarms.remove(&a.task_id);
                }
                drop(alarms);
                for a in due {
                    self.fire(app, a);
                }
                alarms = self.alarms();
                continue;
            }
            let wait = alarms
                .values()
                .map(|a| Duration::from_millis((a.at_ms - now).max(0) as u64))
                .min()
                .unwrap_or(MAX_SLEEP)
                .min(MAX_SLEEP);
            alarms = self.0 .1.wait_timeout(alarms, wait).map_or_else(|e| e.into_inner().0, |(g, _)| g);
        }
    }

    /// Same as the Android receiver: the core decides whether to show it and
    /// when to come back if it's ignored.
    fn fire(&self, app: &AppHandle, alarm: Alarm) {
        let db = app.state::<crate::Db>();
        let out = match db.store.lock() {
            Ok(mut s) => s.on_fired(&alarm.task_id, Source::Native),
            Err(_) => return,
        };
        match out {
            Ok(o) => {
                if o.show {
                    super::notify::show(app, &alarm.task_id, &alarm.title, o.repeats);
                }
                if let Some(next) = o.next_at_ms {
                    self.schedule(Alarm { at_ms: next, ..alarm });
                }
            }
            Err(e) => eprintln!("reminder {}: {e}", alarm.task_id),
        }
    }
}
