//! Print every Befikr device announcing itself on this network (mDNS),
//! paired or not, for 30 s. Diagnostics: `cargo run -p befikr-sync --example discover`.

use std::time::Duration;

use iroh::endpoint::{presets, RelayMode};
use iroh::Endpoint;
use iroh_mdns_address_lookup::{DiscoveryEvent, MdnsAddressLookup};
use n0_future::StreamExt;

#[tokio::main]
async fn main() {
    let ep = Endpoint::builder(presets::Minimal).relay_mode(RelayMode::Disabled).bind().await.unwrap();
    let mdns = MdnsAddressLookup::builder().service_name("befikr").build(ep.id()).unwrap();
    ep.address_lookup().unwrap().add(mdns.clone());
    println!("listening as {} on {:?}", ep.id().fmt_short(), ep.bound_sockets());
    let mut events = mdns.subscribe().await;
    let _ = tokio::time::timeout(Duration::from_secs(30), async {
        while let Some(ev) = events.next().await {
            if let DiscoveryEvent::Discovered { endpoint_info, .. } = ev {
                let data = endpoint_info.data.user_data().map(|u| u.to_string()).unwrap_or_default();
                println!("found {} {:?} addrs {:?}", endpoint_info.endpoint_id.fmt_short(), data, endpoint_info.data.ip_addrs().collect::<Vec<_>>());
            }
        }
    })
    .await;
}
