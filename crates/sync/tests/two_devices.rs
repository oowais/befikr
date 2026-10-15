//! Two services on this machine, same-network mode, addresses handed over
//! directly (no mDNS in tests).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use befikr_core::{MergeOutcome, NewTask, Source, Store, SyncMode};
use befikr_sync::{new_secret_key, Host, SyncService};

#[derive(Clone)]
struct TestHost(Arc<(Mutex<Store>, AtomicU32)>);

impl TestHost {
    fn new(name: &str) -> Self {
        let mut s = Store::open_in_memory().unwrap();
        s.set_sync_mode(SyncMode::Lan).unwrap();
        s.set_device_name(name).unwrap();
        Self(Arc::new((Mutex::new(s), AtomicU32::new(0))))
    }
    fn titles(&self) -> Vec<String> {
        self.0 .0.lock().unwrap().list_open().unwrap().into_iter().map(|i| i.task.title).collect()
    }
}

impl Host for TestHost {
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> befikr_sync::Result<R> {
        f(&mut self.0 .0.lock().unwrap()).map_err(|e| e.to_string())
    }
    fn merged(&self, _: MergeOutcome) {
        self.0 .1.fetch_add(1, Ordering::SeqCst);
    }
    fn status_changed(&self) {}
}

async fn device(name: &str) -> (TestHost, SyncService<TestHost>) {
    let host = TestHost::new(name);
    let svc = SyncService::new(host.clone(), new_secret_key());
    svc.apply_mode().await.unwrap();
    (host, svc)
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_then_sync_both_ways() {
    let (pc, pc_svc) = device("Desktop").await;
    let (phone, phone_svc) = device("Pixel").await;
    pc_svc.add_addr(phone_svc.addr().unwrap());
    phone_svc.add_addr(pc_svc.addr().unwrap());

    pc.store(|s| s.add_task(NewTask { title: "from desktop".into(), ..Default::default() })).unwrap();

    // Wrong code is refused; the right one pairs and syncs right away.
    let code = pc_svc.start_pairing().unwrap();
    assert!(phone_svc.pair_with(&pc_svc.endpoint_id(), "000000x").await.is_err());
    let peer = phone_svc.pair_with(&pc_svc.endpoint_id(), &code).await.unwrap();
    assert_eq!(peer.name, "Desktop");
    assert_eq!(phone.titles(), ["from desktop"]);
    assert_eq!(pc_svc.status().peers[0].name, "Pixel");
    assert!(pc_svc.status().pairing.is_none(), "window closes once used");

    // Complete on the phone → gone on the desktop.
    let id = phone.store(|s| s.list_open()).unwrap()[0].task.id.clone();
    phone.store(|s| s.complete_task(&id, Source::App)).unwrap();
    phone_svc.sync_now().await;
    assert!(pc.titles().is_empty());

    // The other direction, started by the desktop.
    pc.store(|s| s.add_task(NewTask { title: "second".into(), ..Default::default() })).unwrap();
    pc_svc.sync_now().await;
    assert_eq!(phone.titles(), ["second"]);
    assert!(phone.0 .1.load(Ordering::SeqCst) >= 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn unpaired_device_is_refused() {
    let (_, a) = device("A").await;
    let (_, b) = device("B").await;
    b.add_addr(a.addr().unwrap());
    let id = a.endpoint_id().parse().unwrap();
    let r = tokio::time::timeout(Duration::from_secs(15), b.sync_peer(id)).await.unwrap();
    assert!(r.is_err());
}
