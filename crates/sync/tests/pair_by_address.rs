//! Pairing by typed address (no mDNS, no address handed over): the host
//! owns the fixed port, the joiner types "127.0.0.1". Own test binary so no
//! other test holds the port.

use std::sync::{Arc, Mutex};

use befikr_core::{MergeOutcome, NewTask, Store, SyncMode};
use befikr_sync::{new_secret_key, Host, SyncService};

#[derive(Clone)]
struct H(Arc<Mutex<Store>>);
impl Host for H {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> befikr_sync::Result<R> {
        f(&mut self.0.lock().unwrap()).map_err(|e| e.to_string())
    }
    fn merged(&self, _: MergeOutcome) {}
    fn status_changed(&self) {}
}

async fn device(name: &str) -> (H, SyncService<H>) {
    let mut s = Store::open_in_memory().unwrap();
    s.set_sync_mode(SyncMode::Lan).unwrap();
    s.set_device_name(name).unwrap();
    let h = H(Arc::new(Mutex::new(s)));
    let svc = SyncService::new(h.clone(), new_secret_key());
    svc.apply_mode().await.unwrap();
    (h, svc)
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_by_typed_address_and_remember_it() {
    let (pc, pc_svc) = device("Desktop").await;
    let (phone, phone_svc) = device("Pixel").await;
    pc.store(|s| s.add_task(NewTask { title: "from desktop".into(), ..Default::default() })).unwrap();

    let code = pc_svc.start_pairing().unwrap();
    assert!(phone_svc.pair_with_address("127.0.0.2:1", &code).await.is_err(), "nothing there");
    phone_svc.pair_with_address("127.0.0.1", &code).await.unwrap();
    let titles: Vec<String> = phone.0.lock().unwrap().list_open().unwrap().into_iter().map(|i| i.task.title).collect();
    assert_eq!(titles, ["from desktop"]);

    // Both remember where the other is.
    let pc_peer = &phone.0.lock().unwrap().peers().unwrap()[0];
    assert!(pc_peer.addrs.iter().any(|a| a.ends_with(&format!(":{}", befikr_sync::PORT))), "{:?}", pc_peer.addrs);
    assert!(!pc.0.lock().unwrap().peers().unwrap()[0].addrs.is_empty());
}
