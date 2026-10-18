//! Sync and pairing errors as the user reads them. The raw text (iroh,
//! protocol) stays available as the detail; this says what happened and
//! what to do, naming the other device.

use crate::UNREACHABLE;

/// `device`: the other device's name ("" when unknown).
pub fn friendly(raw: &str, device: &str) -> String {
    let (d, mid) = if device.is_empty() { ("The other device", "the other device") } else { (device, device) };
    let has = |s: &str| raw.contains(s);
    if raw == UNREACHABLE {
        format!("{d} isn't reachable. Open Befikr there.")
    } else if has("different Befikr version") {
        format!("{d} runs a different version. Update Befikr on both.")
    } else if has("not paired") {
        format!("{d} no longer knows this device. Remove it here and pair again.")
    } else if has("wrong or expired code") || has("pairing refused") {
        "That code is wrong or has expired. Show a new one on the other device.".into()
    } else if has("pairing link") {
        "That isn’t a Befikr pairing link.".into()
    } else if has("turn sync on first") || has("sync is off") {
        "Turn sync on first.".into()
    } else if has("sync endpoint") {
        "Sync couldn't start. Check the internet connection; it retries on its own.".into()
    } else if has("unexpected message") || has("bad message") || has("message too large") {
        format!("Sync with {mid} failed. Try again; if it keeps failing, update both.")
    } else {
        // connect / send / receive: timeouts and dropped connections.
        format!("Couldn't reach {mid}. Make sure both are online; it retries every minute.")
    }
}

#[cfg(test)]
mod tests {
    use super::friendly;

    #[test]
    fn messages() {
        let cases = [
            ("connect: timed out", "Couldn't reach Pixel 9. Make sure both are online; it retries every minute."),
            ("receive: connection lost", "Couldn't reach Pixel 9. Make sure both are online; it retries every minute."),
            (crate::UNREACHABLE, "Pixel 9 isn't reachable. Open Befikr there."),
            ("other device runs a different Befikr version; update both", "Pixel 9 runs a different version. Update Befikr on both."),
            ("not paired with this device", "Pixel 9 no longer knows this device. Remove it here and pair again."),
            ("wrong or expired code", "That code is wrong or has expired. Show a new one on the other device."),
            ("unexpected message", "Sync with Pixel 9 failed. Try again; if it keeps failing, update both."),
            ("sync endpoint: no network", "Sync couldn't start. Check the internet connection; it retries on its own."),
            ("turn sync on first", "Turn sync on first."),
        ];
        for (raw, want) in cases {
            assert_eq!(friendly(raw, "Pixel 9"), want, "{raw}");
        }
        assert!(friendly("connect: x", "").starts_with("Couldn't reach the other device"));
    }
}
