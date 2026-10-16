//! Wire protocol: one bidirectional QUIC stream per sync or pairing,
//! messages as length-prefixed JSON.
//!
//! Sync (both sides paired): client sends Hello + Want, server checks the
//! client is paired and answers Hello + Want, then both send Changes at the
//! same time (so large batches can't deadlock on flow control), merge what
//! they receive and confirm with Done: when either side returns, both have
//! merged.
//!
//! Pairing: client sends Hello + Pair{code}; server answers Hello + Paired
//! (both save each other as peers) or Denied.

use befikr_core::{Peer, SyncChange};
use iroh::endpoint::{Connection, RecvStream, SendStream};
use serde::{Deserialize, Serialize};

use crate::{Host, Result};

pub const ALPN: &[u8] = b"befikr/sync/1";
const VERSION: u32 = 1;
const MAX_FRAME: usize = 64 * 1024 * 1024;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "camelCase")]
enum Msg {
    /// `addrs`: where the sender can be reached ("ip:port"), remembered by
    /// the other side and tried before the relay next time.
    Hello {
        version: u32,
        device: String,
        name: String,
        #[serde(default)]
        addrs: Vec<String>,
    },
    Pair { code: String },
    Paired,
    Denied { reason: String },
    /// Send me your changes after this seq of yours.
    Want { after: i64 },
    /// `last`: your newest seq at the time; ask for after it next time.
    Changes { changes: Vec<SyncChange>, last: i64 },
    /// Merged what you sent.
    Done,
}

fn remember<H: Host>(host: &H, endpoint: &str, addrs: &[String]) {
    if !addrs.is_empty() {
        let _ = host.store(|s| s.set_peer_addrs(endpoint, addrs));
    }
}

pub enum Served {
    Paired,
    Synced,
}

async fn send(s: &mut SendStream, msg: &Msg) -> Result<()> {
    let bytes = serde_json::to_vec(msg).map_err(|e| e.to_string())?;
    s.write_all(&(bytes.len() as u32).to_be_bytes()).await.map_err(|e| format!("send: {e}"))?;
    s.write_all(&bytes).await.map_err(|e| format!("send: {e}"))?;
    Ok(())
}

async fn recv(r: &mut RecvStream) -> Result<Msg> {
    let mut len = [0u8; 4];
    r.read_exact(&mut len).await.map_err(|e| format!("receive: {e}"))?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err("message too large".into());
    }
    let mut buf = vec![0u8; len];
    r.read_exact(&mut buf).await.map_err(|e| format!("receive: {e}"))?;
    serde_json::from_slice(&buf).map_err(|e| format!("bad message: {e}"))
}

fn hello<H: Host>(host: &H, me: &Me) -> Result<Msg> {
    Ok(Msg::Hello {
        version: VERSION,
        device: host.store(|s| s.device_id())?,
        name: me.name.clone(),
        addrs: me.addrs.clone(),
    })
}

/// Our side of a Hello: name and reachable addresses.
pub struct Me {
    pub name: String,
    pub addrs: Vec<String>,
}

fn expect_hello(msg: Msg) -> Result<(String, String, Vec<String>)> {
    match msg {
        Msg::Hello { version, device, name, addrs } if version == VERSION => Ok((device, name, addrs)),
        Msg::Hello { .. } => Err("other device runs a different Befikr version; update both".into()),
        Msg::Denied { reason } => Err(reason),
        _ => Err("unexpected message".into()),
    }
}

/// Exchange changes once Hello/Want are done: send ours after `their_want`,
/// receive theirs, merge, remember how far we got.
async fn exchange<H: Host>(
    host: &H,
    send_s: &mut SendStream,
    recv_s: &mut RecvStream,
    peer: &Peer,
    their_want: i64,
    their_name: &str,
) -> Result<()> {
    // Normally skip what they made themselves. Asking from 0 means starting
    // over (new pairing, restored backup): then their own changes count too,
    // since a restore may have lost them.
    let skip = (their_want > 0).then_some(peer.device.as_str());
    let (last, changes) = host.store(|s| {
        let last = s.last_seq()?;
        Ok((last, s.changes_since(their_want, u32::MAX, skip)?))
    })?;
    let ours = Msg::Changes { changes, last };
    let (sent, received) = tokio::join!(send(send_s, &ours), recv(recv_s));
    sent?;
    let Msg::Changes { changes, last } = received? else { return Err("unexpected message".into()) };
    let outcome = host.store(|s| s.merge(&changes))?;
    host.store(|s| s.peer_synced(&peer.endpoint, last, their_name))?;
    if outcome.applied > 0 {
        host.merged(outcome);
    }
    send(send_s, &Msg::Done).await?;
    let _ = send_s.finish();
    match recv(recv_s).await? {
        Msg::Done => Ok(()),
        _ => Err("unexpected message".into()),
    }
}

pub async fn sync_as_client<H: Host>(host: &H, conn: &Connection, me: &Me) -> Result<()> {
    let remote = conn.remote_id().to_string();
    let peer = host.store(|s| s.peer(&remote))?.ok_or("not paired")?;
    let (mut tx, mut rx) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send(&mut tx, &hello(host, me)?).await?;
    send(&mut tx, &Msg::Want { after: peer.their_seq }).await?;
    let (_, their_name, addrs) = expect_hello(recv(&mut rx).await?)?;
    remember(host, &remote, &addrs);
    let Msg::Want { after } = recv(&mut rx).await? else { return Err("unexpected message".into()) };
    exchange(host, &mut tx, &mut rx, &peer, after, &their_name).await
}

pub async fn pair<H: Host>(host: &H, conn: &Connection, me: &Me, code: &str) -> Result<Peer> {
    let remote = conn.remote_id().to_string();
    let (mut tx, mut rx) = conn.open_bi().await.map_err(|e| e.to_string())?;
    send(&mut tx, &hello(host, me)?).await?;
    send(&mut tx, &Msg::Pair { code: code.to_string() }).await?;
    let (device, their_name, addrs) = expect_hello(recv(&mut rx).await?)?;
    match recv(&mut rx).await? {
        Msg::Paired => {}
        Msg::Denied { reason } => return Err(reason),
        _ => return Err("unexpected message".into()),
    }
    let _ = tx.finish();
    host.store(|s| s.add_peer(&remote, &device, &their_name))?;
    remember(host, &remote, &addrs);
    host.store(|s| s.peer(&remote))?.ok_or_else(|| "pairing not saved".into())
}

/// Answer an incoming connection. `check_code` validates a pairing code
/// against our open pairing window.
pub async fn serve<H: Host>(
    host: &H,
    conn: &Connection,
    me: &Me,
    check_code: impl FnOnce(&str) -> bool,
) -> Result<Served> {
    let remote = conn.remote_id().to_string();
    let (mut tx, mut rx) = conn.accept_bi().await.map_err(|e| e.to_string())?;
    let (device, their_name, addrs) = expect_hello(recv(&mut rx).await?)?;
    match recv(&mut rx).await? {
        Msg::Pair { code } => {
            if !check_code(&code) {
                send(&mut tx, &Msg::Denied { reason: "wrong or expired code".into() }).await?;
                let _ = tx.finish();
                return Err("pairing refused".into());
            }
            host.store(|s| s.add_peer(&remote, &device, &their_name))?;
            remember(host, &remote, &addrs);
            send(&mut tx, &hello(host, me)?).await?;
            send(&mut tx, &Msg::Paired).await?;
            let _ = tx.finish();
            Ok(Served::Paired)
        }
        Msg::Want { after } => {
            let Some(peer) = host.store(|s| s.peer(&remote))? else {
                send(&mut tx, &Msg::Denied { reason: "not paired with this device".into() }).await?;
                let _ = tx.finish();
                return Err("not paired".into());
            };
            remember(host, &remote, &addrs);
            send(&mut tx, &hello(host, me)?).await?;
            send(&mut tx, &Msg::Want { after: peer.their_seq }).await?;
            exchange(host, &mut tx, &mut rx, &peer, after, &their_name).await?;
            Ok(Served::Synced)
        }
        _ => Err("unexpected message".into()),
    }
}
