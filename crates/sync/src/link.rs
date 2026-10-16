//! Pairing link, shown as a QR code next to the pairing code:
//! `befikr://pair?id=<device id>&code=<6 digits>&name=<device name>&ip=<ip:port>,…&relay=<url>`.
//! Everything the other device needs to reach this one without mDNS or
//! typing: its id (public key), where it listens, its relay (Anywhere mode),
//! and the code that proves the user is holding both devices.

use iroh::{EndpointAddr, EndpointId, RelayUrl, TransportAddr};

pub const PREFIX: &str = "befikr://pair?";

/// A parsed pairing link.
#[derive(Debug, Clone, PartialEq)]
pub struct PairLink {
    pub addr: EndpointAddr,
    pub code: String,
    pub name: String,
}

pub fn encode(addr: &EndpointAddr, code: &str, name: &str) -> String {
    let mut s = format!("{PREFIX}id={}&code={code}&name={}", addr.id, escape(name));
    // Link-local IPv6 needs a scope id the other device can't know: skip it.
    let ips: Vec<String> = addr
        .ip_addrs()
        .filter(|a| !matches!(a.ip(), std::net::IpAddr::V6(v6) if v6.is_unicast_link_local()))
        .map(|a| a.to_string())
        .collect();
    if !ips.is_empty() {
        s += &format!("&ip={}", escape(&ips.join(",")));
    }
    if let Some(relay) = addr.relay_urls().next() {
        s += &format!("&relay={}", escape(relay.as_str()));
    }
    s
}

pub fn decode(link: &str) -> Result<PairLink, String> {
    const BAD: &str = "not a Befikr pairing link";
    let query = link.trim().strip_prefix(PREFIX).ok_or(BAD)?;
    let (mut id, mut code, mut name, mut ips, mut relay) = (None, None, String::new(), Vec::new(), None);
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        let v = unescape(v);
        match k {
            "id" => id = Some(v.parse::<EndpointId>().map_err(|_| BAD)?),
            "code" => code = Some(v),
            "name" => name = v,
            "ip" => ips = v.split(',').filter_map(|a| a.parse().ok()).collect(),
            "relay" => relay = v.parse::<RelayUrl>().ok(),
            _ => {}
        }
    }
    let (id, code) = (id.ok_or(BAD)?, code.ok_or(BAD)?);
    if code.len() != 6 || !code.bytes().all(|b| b.is_ascii_digit()) {
        return Err(BAD.into());
    }
    let mut addr = EndpointAddr::new(id).with_addrs(ips.into_iter().map(TransportAddr::Ip));
    if let Some(relay) = relay {
        addr = addr.with_relay_url(relay);
    }
    Ok(PairLink { addr, code, name })
}

fn escape(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b':' | b',' | b'[' | b']' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%' && i + 2 < b.len())
            .then(|| std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()))
            .flatten();
        match hex {
            Some(v) => {
                out.push(v);
                i += 3;
            }
            None => {
                out.push(if b[i] == b'+' { b' ' } else { b[i] });
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let id = iroh::SecretKey::generate().public();
        let addr = EndpointAddr::new(id)
            .with_ip_addr("192.168.1.20:47474".parse().unwrap())
            .with_ip_addr("[fd00::1]:47474".parse().unwrap())
            .with_relay_url("https://euw1-1.relay.n0.iroh.link./".parse().unwrap());
        let link = encode(&addr, "012345", "Sam's Pixel & co");
        let back = decode(&link).unwrap();
        assert_eq!(back, PairLink { addr, code: "012345".into(), name: "Sam's Pixel & co".into() });
    }

    #[test]
    fn rejects_other_text() {
        assert!(decode("https://example.com").is_err());
        let id = iroh::SecretKey::generate().public();
        assert!(decode(&format!("{PREFIX}id={id}&code=12")).is_err());
        assert!(decode(&format!("{PREFIX}id=nope&code=123456")).is_err());
        assert!(decode(&format!("  {PREFIX}id={id}&code=123456\n")).is_ok());
    }
}
