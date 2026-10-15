//! Manual two-process check of discovery + pairing + sync on one network:
//!
//!   cargo run -p befikr-sync --example peer -- /tmp/a.db A host
//!   cargo run -p befikr-sync --example peer -- /tmp/b.db B join <code>
//!
//! `host` prints a pairing code and waits; `join` pairs with the first nearby
//! device that is waiting, adds a task and syncs. Both print their tasks.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use befikr_core::{MergeOutcome, NewTask, Store, SyncMode};
use befikr_sync::{new_secret_key, Host, SyncService};

#[derive(Clone)]
struct H(Arc<Mutex<Store>>);
impl Host for H {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> befikr_sync::Result<R> {
        f(&mut self.0.lock().unwrap()).map_err(|e| e.to_string())
    }
    fn merged(&self, o: MergeOutcome) {
        println!("merged {} fields", o.applied);
    }
    fn status_changed(&self) {}
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (db, name, role) = (&args[0], &args[1], args[2].as_str());
    let mut s = Store::open(db).unwrap();
    s.set_sync_mode(SyncMode::Lan).unwrap();
    s.set_device_name(name).unwrap();
    s.add_task(NewTask { title: format!("task from {name}"), ..Default::default() }).unwrap();
    let h = H(Arc::new(Mutex::new(s)));
    let svc = SyncService::new(h.clone(), new_secret_key());
    svc.apply_mode().await.unwrap();
    match role {
        "host" => println!("code {}", svc.start_pairing().unwrap()),
        _ => {
            let code = &args[3];
            let target = loop {
                if let Some(n) = svc.status().nearby.first().cloned() {
                    break n;
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            };
            println!("found {} ({})", target.name, &target.endpoint[..10]);
            svc.pair_with(&target.endpoint, code).await.unwrap();
        }
    }
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let titles: Vec<String> = h.0.lock().unwrap().list_open().unwrap().into_iter().map(|i| i.task.title).collect();
        if titles.len() == 2 {
            println!("{name}: {titles:?}");
            break;
        }
    }
}
