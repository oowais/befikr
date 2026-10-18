//! P2P sync between paired devices over iroh (see PLAN.md "P2P sync — plan").
//!
//! One [`SyncService`] per app. While sync is on it runs an iroh endpoint
//! (direct when the network allows, n0's relays otherwise), accepts
//! connections from paired devices, and exchanges `changes` rows with them:
//! on start, soon after any local write (the change log is polled, so writes
//! made outside the app, e.g. from a notification, count too) and every
//! 30 s. Pairing: one device opens a short window and shows a pairing link
//! as a QR code ([`SyncService::start_pairing`]), the other scans or pastes
//! it ([`SyncService::pair_with_link`]).
//!
//! No local discovery (mDNS) and no fixed port: both failed on real phones
//! and networks (PLAN.md "Sync transport — rethink"). Devices find each other
//! by key through n0's address lookup and relays; each Hello carries the
//! sender's direct addresses, which the other side stores and tries first.
//!
//! No Tauri here: the app implements [`Host`] (store access, alarms, events).

mod errors;
mod link;
mod proto;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use befikr_core::{MergeOutcome, Peer, Store, SyncMode};
use iroh::address_lookup::MemoryLookup;
use iroh::endpoint::{presets, RelayMode};
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use serde::Serialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::Instant;

pub use errors::friendly;
pub use proto::ALPN;

const PAIRING_WINDOW: Duration = Duration::from_secs(5 * 60);
const PAIRING_ATTEMPTS: u8 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Sync with every paired device at least this often.
const PERIODIC: Duration = Duration::from_secs(30);
/// How often the change log is checked for local writes.
const POLL: Duration = Duration::from_secs(2);
/// The other device is off or Befikr isn't running there: normal, not an
/// error for the status (its "last synced" shows it).
const UNREACHABLE: &str = "device not reachable. Is Befikr open there with sync on?";

pub type Result<T> = std::result::Result<T, String>;

/// What the app provides.
pub trait Host: Send + Sync + 'static {
    /// Run `f` with the store locked (never across an await).
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> Result<R>;
    /// Remote changes were merged: apply the alarm changes, refresh the UI.
    fn merged(&self, outcome: MergeOutcome);
    /// Peers or pairing changed (for the Settings page).
    fn status_changed(&self);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub mode: SyncMode,
    /// This device's name ("" = not set yet).
    pub name: String,
    pub running: bool,
    /// A sync with some device is in progress.
    pub syncing: bool,
    pub endpoint: String,
    pub peers: Vec<Peer>,
    /// Our open pairing window: link and when it closes (UTC ms).
    pub pairing: Option<PairingInfo>,
    /// The last sync problem, worded for the user ([`friendly`]).
    pub last_error: Option<String>,
    /// The same, as raw text from the network or protocol (for "Details").
    pub last_error_detail: Option<String>,
    /// Open conflicts: our edits that lost to concurrent ones.
    pub conflicts: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub until_ms: i64,
    /// Pairing link (id, code, name, addresses, relay), shown as a QR code.
    pub link: String,
}

struct Pairing {
    code: String,
    until: Instant,
    until_ms: i64,
    attempts: u8,
}

struct Running {
    mode: SyncMode,
    ep: Endpoint,
    memory: MemoryLookup,
    tasks: Vec<JoinHandle<()>>,
}

#[derive(Default)]
struct State {
    running: Option<Running>,
    pairing: Option<Pairing>,
    /// One sync at a time per device; a second waits for the first.
    syncing: BTreeMap<EndpointId, Arc<tokio::sync::Mutex<()>>>,
    /// Syncs in progress (for the status).
    in_flight: usize,
    /// Raw error and the device it was with ("" = none).
    last_error: Option<(String, String)>,
}

struct Inner<H> {
    host: H,
    secret: SecretKey,
    /// Off only in tests: direct addresses only, no internet needed.
    relays: bool,
    state: Mutex<State>,
    nudge: Notify,
}

pub struct SyncService<H: Host> {
    inner: Arc<Inner<H>>,
}

impl<H: Host> Clone for SyncService<H> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone() }
    }
}

/// A new random device key (32 bytes), to store and pass to [`SyncService::new`].
pub fn new_secret_key() -> [u8; 32] {
    SecretKey::generate().to_bytes()
}

impl<H: Host> SyncService<H> {
    pub fn new(host: H, secret_key: [u8; 32]) -> Self {
        Self::build(host, secret_key, true)
    }

    /// No relays or address lookup: devices reach each other only through
    /// addresses handed over directly ([`Self::add_addr`], links). For tests.
    pub fn without_relays(host: H, secret_key: [u8; 32]) -> Self {
        Self::build(host, secret_key, false)
    }

    fn build(host: H, secret_key: [u8; 32], relays: bool) -> Self {
        Self {
            inner: Arc::new(Inner {
                host,
                secret: SecretKey::from_bytes(&secret_key),
                relays,
                state: Mutex::new(State::default()),
                nudge: Notify::new(),
            }),
        }
    }

    pub fn endpoint_id(&self) -> String {
        self.inner.secret.public().to_string()
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.inner.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Start, restart or stop the endpoint to match the stored mode (call on
    /// start and after the mode changes). Needs a tokio runtime.
    pub async fn apply_mode(&self) -> Result<()> {
        let settings = self.inner.host.store(|s| s.sync_settings())?;
        let current = self.state().running.as_ref().map(|r| r.mode);
        if current == Some(settings.mode) {
            return Ok(());
        }
        self.stop().await;
        if settings.mode == SyncMode::Off {
            self.inner.host.status_changed();
            return Ok(());
        }
        let r = self.start(settings.mode).await;
        if let Err(e) = &r {
            self.state().last_error = Some((e.clone(), String::new()));
        }
        self.inner.host.status_changed();
        r
    }

    async fn stop(&self) {
        let running = {
            let mut st = self.state();
            st.pairing = None;
            st.running.take()
        };
        if let Some(r) = running {
            for t in r.tasks {
                t.abort();
            }
            r.ep.close().await;
        }
    }

    async fn start(&self, mode: SyncMode) -> Result<()> {
        let builder = if self.inner.relays {
            Endpoint::builder(presets::N0)
        } else {
            Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled)
        };
        let ep = builder
            .secret_key(self.inner.secret.clone())
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| format!("sync endpoint: {e}"))?;
        let memory = MemoryLookup::new();
        ep.address_lookup().map_err(|e| e.to_string())?.add(memory.clone());

        // Try paired devices where they last were (direct beats the relay).
        for p in self.inner.host.store(|s| s.peers()).unwrap_or_default() {
            hint(&memory, &p.endpoint, &p.addrs);
        }
        let tasks = vec![tokio::spawn(self.clone().accept_loop(ep.clone())), tokio::spawn(self.clone().sync_loop())];
        let mut st = self.state();
        st.last_error = None;
        st.running = Some(Running { mode, ep, memory, tasks });
        Ok(())
    }

    fn endpoint(&self) -> Option<Endpoint> {
        self.state().running.as_ref().map(|r| r.ep.clone())
    }

    fn me(&self) -> proto::Me {
        let addrs = self.endpoint().map(|ep| ep.addr().ip_addrs().map(|a| a.to_string()).collect()).unwrap_or_default();
        proto::Me { name: self.my_name(), addrs }
    }

    fn my_name(&self) -> String {
        let name = self.inner.host.store(|s| s.sync_settings()).map(|s| s.name).unwrap_or_default();
        if name.is_empty() {
            "Befikr device".into()
        } else {
            name
        }
    }

    /// Check for local writes now instead of at the next poll.
    pub fn nudge(&self) {
        self.inner.nudge.notify_one();
    }

    /// Our address (direct addresses + relay).
    pub fn addr(&self) -> Option<EndpointAddr> {
        self.endpoint().map(|ep| ep.addr())
    }

    /// Teach the endpoint where a device is (pairing link; tests).
    pub fn add_addr(&self, addr: EndpointAddr) {
        if let Some(r) = &self.state().running {
            r.memory.add_endpoint_info(addr);
        }
    }

    pub fn status(&self) -> Status {
        let settings = self.inner.host.store(|s| s.sync_settings()).ok();
        let peers = self.inner.host.store(|s| s.peers()).unwrap_or_default();
        let conflicts = self.inner.host.store(|s| s.conflicts()).map(|c| c.len()).unwrap_or(0);
        let my_name = self.my_name();
        let st = self.state();
        Status {
            mode: settings.as_ref().map_or(SyncMode::Off, |s| s.mode),
            name: settings.map(|s| s.name).unwrap_or_default(),
            running: st.running.is_some(),
            syncing: st.in_flight > 0,
            endpoint: self.endpoint_id(),
            peers,
            pairing: st.pairing.as_ref().map(|p| PairingInfo {
                until_ms: p.until_ms,
                link: st.running.as_ref().map(|r| link::encode(&r.ep.addr(), &p.code, &my_name)).unwrap_or_default(),
            }),
            last_error: st.last_error.as_ref().map(|(raw, device)| friendly(raw, device)),
            last_error_detail: st.last_error.as_ref().map(|(raw, _)| raw.clone()),
            conflicts,
        }
    }

    /// Open a pairing window; its link is in [`Self::status`].
    pub fn start_pairing(&self) -> Result<()> {
        if self.endpoint().is_none() {
            return Err("Turn sync on first.".into());
        }
        let code = format!("{:06}", rand::random_range(0..1_000_000u32));
        let now_ms = self.inner.host.store(|s| Ok(s.now()))?;
        self.state().pairing = Some(Pairing {
            code,
            until: Instant::now() + PAIRING_WINDOW,
            until_ms: now_ms + PAIRING_WINDOW.as_millis() as i64,
            attempts: 0,
        });
        self.inner.host.status_changed();
        Ok(())
    }

    pub fn stop_pairing(&self) {
        self.state().pairing = None;
        self.inner.host.status_changed();
    }

    /// Pair from a pairing link (scanned QR code or pasted): it carries the
    /// device's id, addresses, relay and code. Then sync with it.
    /// Errors are worded for the user.
    pub async fn pair_with_link(&self, link: &str) -> Result<Peer> {
        let link = link::decode(link).map_err(|e| friendly(&e, ""))?;
        let name = link.name.clone();
        self.pair(link).await.map_err(|e| friendly(&e, &name))
    }

    async fn pair(&self, link: link::PairLink) -> Result<Peer> {
        let ep = self.endpoint().ok_or("turn sync on first")?;
        let id = link.addr.id;
        self.add_addr(link.addr);
        let conn = connect(&ep, id).await?;
        let peer = proto::pair(&self.inner.host, &conn, &self.me(), &link.code).await;
        conn.close(0u32.into(), b"done");
        let peer = peer?;
        self.inner.host.status_changed();
        let _ = self.sync_peer(id).await;
        Ok(peer)
    }

    /// One sync round with every paired device, then stop: for background
    /// sync with no app running (Android WorkManager). Nothing when sync is
    /// off. Errors: the last sync error, if any device failed.
    pub async fn sync_once(&self) -> Result<()> {
        self.apply_mode().await?;
        if self.endpoint().is_none() {
            return Ok(());
        }
        self.sync_now().await;
        let error = self.state().last_error.clone().map(|(raw, _)| raw);
        self.stop().await;
        error.map_or(Ok(()), Err)
    }

    pub async fn unpair(&self, endpoint: &str) -> Result<()> {
        self.inner.host.store(|s| s.remove_peer(endpoint))?;
        self.inner.host.status_changed();
        Ok(())
    }

    /// Sync with every paired device that can be reached.
    pub async fn sync_now(&self) {
        let peers = self.inner.host.store(|s| s.peers()).unwrap_or_default();
        let jobs = peers.iter().filter_map(|p| Some((p.endpoint.parse::<EndpointId>().ok()?, p.name.clone()))).map(|(id, name)| {
            let me = self.clone();
            tokio::spawn(async move { (id, name, me.sync_peer(id).await) })
        });
        // An unreachable device, or one that failed but synced lately, is
        // no error.
        let now = self.inner.host.store(|s| Ok(s.now())).unwrap_or(0);
        let fresh = |id: &EndpointId| {
            let id = id.to_string();
            let synced = self.inner.host.store(|s| s.peer(&id)).ok().flatten().and_then(|p| p.last_sync_at);
            synced.is_some_and(|t| now - t < 3 * PERIODIC.as_millis() as i64)
        };
        let mut error = None;
        for j in jobs.collect::<Vec<_>>() {
            if let Ok((id, name, Err(e))) = j.await {
                eprintln!("sync with {}: {e}", id.fmt_short());
                if e != UNREACHABLE && !fresh(&id) {
                    error = Some((e, name));
                }
            }
        }
        self.state().last_error = error;
        self.inner.host.status_changed();
    }

    /// One sync with one paired device (after any that's still running).
    pub async fn sync_peer(&self, id: EndpointId) -> Result<()> {
        let ep = self.endpoint().ok_or("sync is off")?;
        let lock = self.state().syncing.entry(id).or_default().clone();
        let _one_at_a_time = lock.lock().await;
        self.state().in_flight += 1;
        self.inner.host.status_changed();
        let r = async {
            let conn = connect(&ep, id).await?;
            let r = proto::sync_as_client(&self.inner.host, &conn, &self.me()).await;
            conn.close(0u32.into(), b"done");
            r
        }
        .await;
        self.state().in_flight -= 1;
        self.inner.host.status_changed();
        r
    }

    async fn accept_loop(self, ep: Endpoint) {
        while let Some(incoming) = ep.accept().await {
            let me = self.clone();
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                let r = proto::serve(&me.inner.host, &conn, &me.me(), |code| me.check_code(code)).await;
                match r {
                    Ok(proto::Served::Paired) => {
                        me.stop_pairing();
                    }
                    Ok(proto::Served::Synced) => me.inner.host.status_changed(),
                    Err(e) => eprintln!("sync from {}: {e}", conn.remote_id().fmt_short()),
                }
                let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
            });
        }
    }

    /// Pairing code check for an incoming pairing request.
    fn check_code(&self, code: &str) -> bool {
        let mut st = self.state();
        let Some(p) = st.pairing.as_mut() else { return false };
        if Instant::now() > p.until {
            st.pairing = None;
            return false;
        }
        if p.code == code {
            return true;
        }
        p.attempts += 1;
        if p.attempts >= PAIRING_ATTEMPTS {
            st.pairing = None;
        }
        false
    }

    async fn sync_loop(self) {
        let last_seq = || self.inner.host.store(|s| s.last_seq()).unwrap_or(0);
        let mut synced_seq = last_seq();
        let mut synced_at = Instant::now();
        self.sync_now().await;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(POLL) => {}
                _ = self.inner.nudge.notified() => {}
            }
            let seq = last_seq();
            if seq != synced_seq || synced_at.elapsed() >= PERIODIC {
                // A merge raises the seq too: one more (empty) round follows.
                synced_seq = seq;
                synced_at = Instant::now();
                self.sync_now().await;
            }
        }
    }
}

/// Tell the endpoint where a paired device was last seen.
fn hint(memory: &MemoryLookup, endpoint: &str, addrs: &[String]) {
    let Ok(id) = endpoint.parse::<EndpointId>() else { return };
    let ips: Vec<std::net::SocketAddr> = addrs.iter().filter_map(|a| a.parse().ok()).collect();
    if !ips.is_empty() {
        memory.add_endpoint_info(EndpointAddr::new(id).with_addrs(ips.into_iter().map(iroh::TransportAddr::Ip)));
    }
}

async fn connect(ep: &Endpoint, id: EndpointId) -> Result<iroh::endpoint::Connection> {
    tokio::time::timeout(CONNECT_TIMEOUT, ep.connect(id, ALPN))
        .await
        .map_err(|_| UNREACHABLE.to_string())?
        .map_err(|e| format!("connect: {e}"))
}
