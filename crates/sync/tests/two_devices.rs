//! Two services on this machine, without relays (no internet needed):
//! they reach each other only through the addresses a pairing link carries.

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
        s.set_sync_mode(SyncMode::Relay).unwrap();
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
    let svc = SyncService::without_relays(host.clone(), new_secret_key());
    svc.apply_mode().await.unwrap();
    (host, svc)
}

fn pairing_link(svc: &SyncService<TestHost>) -> String {
    svc.start_pairing().unwrap();
    svc.status().pairing.unwrap().link
}

/// The same link with a different code.
fn wrong_code(link: &str) -> String {
    let at = link.find("code=").unwrap() + 5;
    let code = &link[at..at + 6];
    let other = if code == "000000" { "111111" } else { "000000" };
    link.replacen(code, other, 1)
}

#[tokio::test(flavor = "multi_thread")]
async fn pair_from_link_then_sync_both_ways() {
    let (pc, pc_svc) = device("Desktop").await;
    let (phone, phone_svc) = device("Pixel").await;
    pc.store(|s| s.add_task(NewTask { title: "from desktop".into(), ..Default::default() })).unwrap();

    // The phone learns nothing except what the QR code carries.
    let link = pairing_link(&pc_svc);
    assert!(link.starts_with("befikr://pair?") && link.contains("name=Desktop"), "{link}");
    assert!(phone_svc.pair_with_link("hello").await.is_err());
    assert!(phone_svc.pair_with_link(&wrong_code(&link)).await.is_err());
    let peer = phone_svc.pair_with_link(&link).await.unwrap();
    assert_eq!(peer.name, "Desktop");
    assert_eq!(phone.titles(), ["from desktop"]);
    assert_eq!(pc_svc.status().peers[0].name, "Pixel");
    assert!(pc_svc.status().pairing.is_none(), "window closes once used");
    // Both remember where the other was.
    assert!(!phone.store(|s| s.peers()).unwrap()[0].addrs.is_empty());
    assert!(!pc.store(|s| s.peers()).unwrap()[0].addrs.is_empty());

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

#[tokio::test(flavor = "multi_thread")]
async fn too_many_wrong_codes_close_the_window() {
    let (_, pc_svc) = device("Desktop").await;
    let (_, phone_svc) = device("Pixel").await;
    let link = pairing_link(&pc_svc);
    for _ in 0..5 {
        assert!(phone_svc.pair_with_link(&wrong_code(&link)).await.is_err());
    }
    assert!(pc_svc.status().pairing.is_none());
    assert!(phone_svc.pair_with_link(&link).await.is_err(), "right code, but too late");
}

#[tokio::test(flavor = "multi_thread")]
async fn sync_once_syncs_then_stops() {
    let (pc, pc_svc) = device("Desktop").await;
    let (phone, phone_svc) = device("Pixel").await;
    phone_svc.pair_with_link(&pairing_link(&pc_svc)).await.unwrap();

    pc.store(|s| s.add_task(NewTask { title: "added on the desktop".into(), ..Default::default() })).unwrap();
    phone_svc.sync_once().await.unwrap();
    assert_eq!(phone.titles(), ["added on the desktop"]);
    assert!(!phone_svc.status().running, "endpoint closed after the round");
    assert!(phone.0 .1.load(Ordering::SeqCst) >= 1, "merge reported to the host (alarms)");
}
