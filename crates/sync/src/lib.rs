//! P2P sync between paired devices over iroh (see PLAN.md "P2P sync — plan").
//!
//! One [`SyncService`] per app. While sync is on it runs an iroh endpoint
//! (same network only, or with n0 relays as fallback), finds other Befikr
//! devices with mDNS, accepts connections from paired devices, and exchanges
//! `changes` rows with them: on start, when a paired device shows up, soon
//! after any local write (the change log is polled, so writes made outside
//! the app, e.g. from a notification, count too) and every minute. Pairing: one device
//! opens a short window with a 6-digit code ([`SyncService::start_pairing`]),
//! the other connects and sends the code ([`SyncService::pair_with`], or
//! [`SyncService::pair_with_address`] when mDNS can't see it, or
//! [`SyncService::pair_with_link`] from the QR code / link next to the code).
//!
//! mDNS is flaky (Android, some routers), so like Syncthing we don't rely on
//! it: the endpoint listens on a fixed port ([`PORT`], one firewall rule),
//! a tiny TCP responder on the same port tells who's here (for pairing by
//! address), and each Hello carries the sender's addresses, which the other
//! side stores and dials directly next time.
//!
//! No Tauri here: the app implements [`Host`] (store access, alarms, events).

mod link;
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

/// Sync listens on this UDP port (QUIC) and TCP port (who's-here lookup).
/// Falls back to a random UDP port if taken.
pub const PORT: u16 = 47474;

/// mDNS service name: only Befikr devices see each other.
const SERVICE: &str = "befikr";
const PAIRING_WINDOW: Duration = Duration::from_secs(5 * 60);
const PAIRING_ATTEMPTS: u8 = 5;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// Also how fast changes reach a device that can only be dialled from the
/// other side (a computer whose firewall drops incoming connections).
const PERIODIC: Duration = Duration::from_secs(30);
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
    /// A sync with some device is in progress.
    pub syncing: bool,
    pub endpoint: String,
    pub peers: Vec<Peer>,
    /// Befikr devices on this network that are waiting to pair.
    pub nearby: Vec<Nearby>,
    /// Our open pairing window: code and when it closes (UTC ms).
    pub pairing: Option<PairingInfo>,
    /// This device's LAN addresses, to type on the other device when it
    /// doesn't show up under Nearby ("192.168.1.20", or "ip:port" off PORT).
    pub addresses: Vec<String>,
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
    /// Discovered devices: name and whether they're open for pairing.
    seen: BTreeMap<EndpointId, (String, bool)>,
    last_seen_sync: BTreeMap<EndpointId, Instant>,
    /// One sync at a time per device; a second waits for the first.
    syncing: BTreeMap<EndpointId, Arc<tokio::sync::Mutex<()>>>,
    /// Syncs in progress (for the status).
    in_flight: usize,
    /// Lasting problem with this device's setup (shown when there's no
    /// fresher error), e.g. mDNS unavailable.
    warning: Option<String>,
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
        let builder = || {
            let b = match mode {
                SyncMode::Lan => Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled),
                _ => Endpoint::builder(presets::N0),
            };
            b.secret_key(self.inner.secret.clone()).alpns(vec![ALPN.to_vec()])
        };
        let fixed = builder()
            .bind_addr(format!("0.0.0.0:{PORT}"))
            .and_then(|b| b.bind_addr(format!("[::]:{PORT}")))
            .map_err(|e| e.to_string());
        let ep = match fixed {
            Ok(b) => match b.bind().await {
                Ok(ep) => ep,
                // Port taken (another copy running): any port still works via
                // stored addresses and mDNS.
                Err(_) => builder().bind().await.map_err(|e| format!("sync endpoint: {e}"))?,
            },
            Err(_) => builder().bind().await.map_err(|e| format!("sync endpoint: {e}"))?,
        };
        let lookups = ep.address_lookup().map_err(|e| e.to_string())?;
        let memory = MemoryLookup::new();
        lookups.add(memory.clone());
        // Nearby discovery is a bonus: if mDNS can't start (some Android
        // versions, odd networks), sync still runs via addresses.
        let mdns = MdnsAddressLookup::builder().service_name(SERVICE).build(ep.id());
        let warning = match &mdns {
            Ok(m) => {
                lookups.add(m.clone());
                None
            }
            Err(e) => Some(format!("Nearby devices unavailable ({e}); pair by address instead")),
        };

        // Dial paired devices where they last were.
        for p in self.inner.host.store(|s| s.peers()).unwrap_or_default() {
            hint(&memory, &p.endpoint, &p.addrs);
        }
        let mut tasks = vec![
            tokio::spawn(self.clone().id_server(ep.clone())),
            tokio::spawn(self.clone().accept_loop(ep.clone())),
            tokio::spawn(self.clone().sync_loop()),
        ];
        if let Ok(m) = mdns {
            tasks.push(tokio::spawn(self.clone().discovery_loop(m)));
        }
        {
            let mut st = self.state();
            st.last_error = None;
            st.warning = warning;
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
        let my_name = self.my_name();
        let st = self.state();
        let paired: HashSet<&str> = peers.iter().map(|p| p.endpoint.as_str()).collect();
        Status {
            mode: settings.as_ref().map_or(SyncMode::Off, |s| s.mode),
            name: settings.map(|s| s.name).unwrap_or_default(),
            running: st.running.is_some(),
            syncing: st.in_flight > 0,
            endpoint: self.endpoint_id(),
            nearby: st
                .seen
                .iter()
                .filter(|(id, (_, pairing))| *pairing && !paired.contains(id.to_string().as_str()))
                .map(|(id, (name, _))| Nearby { endpoint: id.to_string(), name: name.clone() })
                .collect(),
            peers,
            pairing: st.pairing.as_ref().map(|p| PairingInfo {
                code: p.code.clone(),
                until_ms: p.until_ms,
                link: st.running.as_ref().map(|r| link::encode(&r.ep.addr(), &p.code, &my_name)).unwrap_or_default(),
            }),
            last_error: st.last_error.clone().or_else(|| st.warning.clone()),
            addresses: st.running.as_ref().map(|r| lan_addresses(&r.ep)).unwrap_or_default(),
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
        let peer = proto::pair(&self.inner.host, &conn, &self.me(), code.trim()).await;
        conn.close(0u32.into(), b"done");
        let peer = peer?;
        self.inner.host.status_changed();
        let _ = self.sync_peer(id).await;
        Ok(peer)
    }

    /// Pair with a device by its address ("192.168.1.20" or "ip:port"), for
    /// when it doesn't show up under Nearby. Its id comes from the TCP
    /// responder on that address; the code (shown on that device) proves it.
    pub async fn pair_with_address(&self, address: &str, code: &str) -> Result<Peer> {
        let address = address.trim();
        let target = if address.parse::<std::net::IpAddr>().is_ok() {
            match address.parse::<std::net::Ipv6Addr>() {
                Ok(v6) => std::net::SocketAddr::from((v6, PORT)),
                Err(_) => format!("{address}:{PORT}").parse().map_err(|_| "not an address".to_string())?,
            }
        } else {
            address.parse::<std::net::SocketAddr>().map_err(|_| "type an address like 192.168.1.20".to_string())?
        };
        let who = tokio::time::timeout(CONNECT_TIMEOUT, who_is(target))
            .await
            .map_err(|_| "No Befikr at that address. Is sync on there?".to_string())??;
        let id: EndpointId = who.endpoint.parse().map_err(|_| "bad reply".to_string())?;
        if let Some(r) = &self.state().running {
            r.memory.add_endpoint_info(EndpointAddr::new(id).with_ip_addr((target.ip(), who.port).into()));
        }
        self.pair_with(&who.endpoint, code).await
    }

    /// Pair from a pairing link (scanned QR code or pasted): it carries the
    /// device's id, addresses, relay and code, so no mDNS or typing needed.
    pub async fn pair_with_link(&self, link: &str) -> Result<Peer> {
        let link = link::decode(link)?;
        let id = link.addr.id.to_string();
        self.add_addr(link.addr);
        self.pair_with(&id, &link.code).await
    }

    /// Answer "who's here" on TCP [`PORT`]: our id and QUIC port, one JSON
    /// line. Public information (the id is a public key); pairing still
    /// needs the code.
    async fn id_server(self, ep: Endpoint) {
        let listener = match tokio::net::TcpListener::bind(("::", PORT)).await {
            Ok(l) => l,
            Err(_) => match tokio::net::TcpListener::bind(("0.0.0.0", PORT)).await {
                Ok(l) => l,
                Err(e) => {
                    self.state().warning = Some(format!("Pairing by address unavailable on this device: {e}"));
                    self.inner.host.status_changed();
                    return;
                }
            },
        };
        let port = ep.bound_sockets().first().map_or(PORT, |a| a.port());
        let line = format!("{}\n", serde_json::json!({ "endpoint": ep.id().to_string(), "port": port }));
        while let Ok((mut stream, _)) = listener.accept().await {
            let line = line.clone();
            tokio::spawn(async move {
                use tokio::io::AsyncWriteExt;
                let _ = stream.write_all(line.as_bytes()).await;
            });
        }
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
        // A device we can't reach but that reached us lately (its firewall
        // only lets it dial out) is fine: no error for that one.
        let now = self.inner.host.store(|s| Ok(s.now())).unwrap_or(0);
        let fresh = |id: &EndpointId| {
            let id = id.to_string();
            let synced = self.inner.host.store(|s| s.peer(&id)).ok().flatten().and_then(|p| p.last_sync_at);
            synced.is_some_and(|t| now - t < 3 * PERIODIC.as_millis() as i64)
        };
        let mut error = None;
        for j in jobs.collect::<Vec<_>>() {
            if let Ok((id, Err(e))) = j.await {
                eprintln!("sync with {}: {e}", id.fmt_short());
                if !fresh(&id) {
                    error = Some(e);
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

    /// Look for devices again: restart the endpoint, so discovery starts
    /// over and this device announces itself afresh.
    pub async fn rescan(&self) -> Result<()> {
        let Some(mode) = self.state().running.as_ref().map(|r| r.mode) else {
            return Err("sync is off".into());
        };
        let pairing = self.state().pairing.take();
        self.stop().await;
        let r = self.start(mode).await;
        if let Err(e) = &r {
            self.state().last_error = Some(e.clone());
        } else if pairing.is_some() {
            self.state().pairing = pairing;
            self.advertise();
        }
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

#[derive(serde::Deserialize)]
struct WhoIs {
    endpoint: String,
    port: u16,
}

async fn who_is(target: std::net::SocketAddr) -> Result<WhoIs> {
    use tokio::io::AsyncReadExt;
    let mut stream = tokio::net::TcpStream::connect(target)
        .await
        .map_err(|_| "No Befikr at that address. Is sync on there? On a computer, pair from the computer instead: show the code on the other device and type its address on the computer.".to_string())?;
    let mut buf = Vec::new();
    let _ = (&mut stream).take(1024).read_to_end(&mut buf).await;
    serde_json::from_slice(buf.trim_ascii()).map_err(|_| "something else answered at that address".to_string())
}

/// Tell the endpoint where a paired device was last seen.
fn hint(memory: &MemoryLookup, endpoint: &str, addrs: &[String]) {
    let Ok(id) = endpoint.parse::<EndpointId>() else { return };
    let ips: Vec<std::net::SocketAddr> = addrs.iter().filter_map(|a| a.parse().ok()).collect();
    if !ips.is_empty() {
        memory.add_endpoint_info(EndpointAddr::new(id).with_addrs(ips.into_iter().map(iroh::TransportAddr::Ip)));
    }
}

/// Private IPv4 addresses (home network), as typed on another device.
fn lan_addresses(ep: &Endpoint) -> Vec<String> {
    let mut out: Vec<String> = ep
        .addr()
        .ip_addrs()
        .filter_map(|a| match a.ip() {
            std::net::IpAddr::V4(v4) if v4.is_private() => {
                Some(if a.port() == PORT { v4.to_string() } else { a.to_string() })
            }
            _ => None,
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

async fn connect(ep: &Endpoint, id: EndpointId) -> Result<iroh::endpoint::Connection> {
    tokio::time::timeout(CONNECT_TIMEOUT, ep.connect(id, ALPN))
        .await
        .map_err(|_| "device not reachable".to_string())?
        .map_err(|e| format!("connect: {e}"))
}
