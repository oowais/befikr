//! P2P sync between paired devices over iroh (see PLAN.md "P2P sync — plan").
//!
//! One [`SyncService`] per app. While sync is on it runs an iroh endpoint
//! (same network only, or with n0 relays as fallback), finds other Befikr
//! devices with mDNS, accepts connections from paired devices, and exchanges
//! `changes` rows with them: on start, when a paired device shows up, soon
//! after any local write (the change log is polled, so writes made outside
//! the app, e.g. from a notification, count too) and every minute. Pairing: one device
//! opens a short window with a 6-digit code ([`SyncService::start_pairing`]),
//! the other connects and sends the code ([`SyncService::pair_with`]).
//!
//! No Tauri here: the app implements [`Host`] (store access, alarms, events).

mod proto;

use std::collections::{BTreeMap, HashSet};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use befikr_core::{MergeOutcome, Peer, Store, SyncMode};
use iroh::address_lookup::{MemoryLookup, UserData};
use iroh::endpoint::{presets, RelayMode};
use iroh::{Endpoint, EndpointAddr, EndpointId, SecretKey};
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use n0_future::StreamExt;
use serde::Serialize;
use tokio::sync::Notify;
use tokio::task::JoinHandle;
use tokio::time::Instant;

pub use proto::ALPN;

/// mDNS service name: only Befikr devices see each other.
const SERVICE: &str = "befikr";
const PAIRING_WINDOW: Duration = Duration::from_secs(5 * 60);
const PAIRING_ATTEMPTS: u8 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const PERIODIC: Duration = Duration::from_secs(60);
/// How often the change log is checked for local writes.
const POLL: Duration = Duration::from_secs(2);
/// A device seen again via mDNS within this time doesn't trigger another sync.
const SEEN_COOLDOWN: Duration = Duration::from_secs(20);

pub type Result<T> = std::result::Result<T, String>;

/// What the app provides.
pub trait Host: Send + Sync + 'static {
    /// Run `f` with the store locked (never across an await).
    fn store<R>(&self, f: impl FnOnce(&mut Store) -> befikr_core::Result<R>) -> Result<R>;
    /// Remote changes were merged: apply the alarm changes, refresh the UI.
    fn merged(&self, outcome: MergeOutcome);
    /// Peers, nearby devices or pairing changed (for the Settings page).
    fn status_changed(&self);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub mode: SyncMode,
    /// This device's name ("" = not set yet).
    pub name: String,
    pub running: bool,
    pub endpoint: String,
    pub peers: Vec<Peer>,
    /// Befikr devices on this network that are waiting to pair.
    pub nearby: Vec<Nearby>,
    /// Our open pairing window: code and when it closes (UTC ms).
    pub pairing: Option<PairingInfo>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Nearby {
    pub endpoint: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingInfo {
    pub code: String,
    pub until_ms: i64,
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
    /// Discovered devices: name and whether they're open for pairing.
    seen: BTreeMap<EndpointId, (String, bool)>,
    last_seen_sync: BTreeMap<EndpointId, Instant>,
    /// One sync at a time per device; a second waits for the first.
    syncing: BTreeMap<EndpointId, Arc<tokio::sync::Mutex<()>>>,
    last_error: Option<String>,
}

struct Inner<H> {
    host: H,
    secret: SecretKey,
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
        Self {
            inner: Arc::new(Inner {
                host,
                secret: SecretKey::from_bytes(&secret_key),
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
            self.state().last_error = Some(e.clone());
        }
        self.inner.host.status_changed();
        r
    }

    async fn stop(&self) {
        let running = {
            let mut st = self.state();
            st.seen.clear();
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
        let builder = match mode {
            SyncMode::Lan => Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled),
            _ => Endpoint::builder(presets::N0),
        };
        let ep = builder
            .secret_key(self.inner.secret.clone())
            .alpns(vec![ALPN.to_vec()])
            .bind()
            .await
            .map_err(|e| format!("sync endpoint: {e}"))?;
        let lookups = ep.address_lookup().map_err(|e| e.to_string())?;
        let memory = MemoryLookup::new();
        lookups.add(memory.clone());
        let mdns = MdnsAddressLookup::builder()
            .service_name(SERVICE)
            .build(ep.id())
            .map_err(|e| format!("local discovery: {e}"))?;
        lookups.add(mdns.clone());

        let tasks = vec![
            tokio::spawn(self.clone().accept_loop(ep.clone())),
            tokio::spawn(self.clone().discovery_loop(mdns)),
            tokio::spawn(self.clone().sync_loop()),
        ];
        {
            let mut st = self.state();
            st.last_error = None;
            st.running = Some(Running { mode, ep, memory, tasks });
        }
        self.advertise();
        Ok(())
    }

    fn endpoint(&self) -> Option<Endpoint> {
        self.state().running.as_ref().map(|r| r.ep.clone())
    }

    /// Our name and pairing flag, for other devices' "nearby" lists.
    fn advertise(&self) {
        let name = self.my_name();
        let st = self.state();
        let Some(r) = &st.running else { return };
        let flag = if st.pairing.is_some() { '1' } else { '0' };
        if let Ok(data) = UserData::try_from(format!("{flag}{name}")) {
            r.ep.set_user_data_for_address_lookup(Some(data));
        }
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

    /// Our address (direct addresses + relay), e.g. for a pairing link.
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
        let st = self.state();
        let paired: HashSet<&str> = peers.iter().map(|p| p.endpoint.as_str()).collect();
        Status {
            mode: settings.as_ref().map_or(SyncMode::Off, |s| s.mode),
            name: settings.map(|s| s.name).unwrap_or_default(),
            running: st.running.is_some(),
            endpoint: self.endpoint_id(),
            nearby: st
                .seen
                .iter()
                .filter(|(id, (_, pairing))| *pairing && !paired.contains(id.to_string().as_str()))
                .map(|(id, (name, _))| Nearby { endpoint: id.to_string(), name: name.clone() })
                .collect(),
            peers,
            pairing: st.pairing.as_ref().map(|p| PairingInfo { code: p.code.clone(), until_ms: p.until_ms }),
            last_error: st.last_error.clone(),
        }
    }

    /// Open a pairing window and return its code.
    pub fn start_pairing(&self) -> Result<String> {
        if self.endpoint().is_none() {
            return Err("turn sync on first".into());
        }
        let code = format!("{:06}", rand::random_range(0..1_000_000u32));
        let now_ms = self.inner.host.store(|s| Ok(s.now()))?;
        self.state().pairing = Some(Pairing {
            code: code.clone(),
            until: Instant::now() + PAIRING_WINDOW,
            until_ms: now_ms + PAIRING_WINDOW.as_millis() as i64,
            attempts: 0,
        });
        self.advertise();
        self.inner.host.status_changed();
        Ok(code)
    }

    pub fn stop_pairing(&self) {
        self.state().pairing = None;
        self.advertise();
        self.inner.host.status_changed();
    }

    /// Pair with a device that shows `code`, then sync with it.
    pub async fn pair_with(&self, endpoint: &str, code: &str) -> Result<Peer> {
        let ep = self.endpoint().ok_or("turn sync on first")?;
        let id: EndpointId = endpoint.parse().map_err(|_| "not a device id".to_string())?;
        let conn = connect(&ep, id).await?;
        let peer = proto::pair(&self.inner.host, &conn, &self.my_name(), code.trim()).await;
        conn.close(0u32.into(), b"done");
        let peer = peer?;
        self.inner.host.status_changed();
        let _ = self.sync_peer(id).await;
        Ok(peer)
    }

    pub async fn unpair(&self, endpoint: &str) -> Result<()> {
        self.inner.host.store(|s| s.remove_peer(endpoint))?;
        self.inner.host.status_changed();
        Ok(())
    }

    /// Sync with every paired device that can be reached.
    pub async fn sync_now(&self) {
        let peers = self.inner.host.store(|s| s.peers()).unwrap_or_default();
        let jobs = peers.iter().filter_map(|p| p.endpoint.parse::<EndpointId>().ok()).map(|id| {
            let me = self.clone();
            tokio::spawn(async move { (id, me.sync_peer(id).await) })
        });
        let mut error = None;
        for j in jobs.collect::<Vec<_>>() {
            if let Ok((id, Err(e))) = j.await {
                eprintln!("sync with {}: {e}", id.fmt_short());
                error = Some(e);
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
        let conn = connect(&ep, id).await?;
        let r = proto::sync_as_client(&self.inner.host, &conn, &self.my_name()).await;
        conn.close(0u32.into(), b"done");
        r
    }

    async fn accept_loop(self, ep: Endpoint) {
        while let Some(incoming) = ep.accept().await {
            let me = self.clone();
            tokio::spawn(async move {
                let Ok(conn) = incoming.await else { return };
                let name = me.my_name();
                let r = proto::serve(&me.inner.host, &conn, &name, |code| me.check_code(code)).await;
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

    async fn discovery_loop(self, mdns: MdnsAddressLookup) {
        let mut events = mdns.subscribe().await;
        while let Some(ev) = events.next().await {
            match ev {
                DiscoveryEvent::Discovered { endpoint_info, .. } => {
                    let id = endpoint_info.endpoint_id;
                    let Some(data) = endpoint_info.data.user_data() else { continue };
                    let data: &str = data.as_ref();
                    let (flag, name) = data.split_at(data.char_indices().nth(1).map_or(data.len(), |(i, _)| i));
                    let changed = {
                        let mut st = self.state();
                        let entry = (name.to_string(), flag == "1");
                        st.seen.insert(id, entry.clone()) != Some(entry)
                    };
                    if changed {
                        self.inner.host.status_changed();
                    }
                    let paired = self.inner.host.store(|s| s.peer(&id.to_string())).ok().flatten().is_some();
                    let due = {
                        let mut st = self.state();
                        let due = st.last_seen_sync.get(&id).is_none_or(|t| t.elapsed() > SEEN_COOLDOWN);
                        if due {
                            st.last_seen_sync.insert(id, Instant::now());
                        }
                        due
                    };
                    if paired && due {
                        let me = self.clone();
                        tokio::spawn(async move {
                            let _ = me.sync_peer(id).await;
                            me.inner.host.status_changed();
                        });
                    }
                }
                DiscoveryEvent::Expired { endpoint_id } => {
                    self.state().seen.remove(&endpoint_id);
                    self.inner.host.status_changed();
                }
                _ => {}
            }
        }
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

async fn connect(ep: &Endpoint, id: EndpointId) -> Result<iroh::endpoint::Connection> {
    tokio::time::timeout(CONNECT_TIMEOUT, ep.connect(id, ALPN))
        .await
        .map_err(|_| "device not reachable".to_string())?
        .map_err(|e| format!("connect: {e}"))
}
