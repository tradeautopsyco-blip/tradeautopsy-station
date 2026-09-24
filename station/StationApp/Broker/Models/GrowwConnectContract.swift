import Foundation

/// ADR 0014 key-entry flow — agent owns the checksum mint; Swift never touches
/// the api secret beyond the entry form, never sees the session token.
///
/// There is deliberately NO loopback callback, NO consent/login URL, and NO URL
/// guard on this shape: no official Groww page names a redirect (B6 row 10/16),
/// so the 0005/0008/0009 redirect machinery does not apply. Swift posts
/// `{apiKey, apiSecret}` to the signed loopback connect route, the agent
/// Keychain-stores the credentials and mints the session, and Swift confirms
/// via the shared vault-presence route — then autostarts sync. No browser
/// opens, no URL is parsed, no shell forks.
public enum GrowwConnectContract {
    /// Signed loopback route (integrator-owned). Swift sends key-entry
    /// `{apiKey, apiSecret}` + identity; agent mints and Keychain-stores.
    public static let connectPath = "/api/daemon/broker/groww/connect"

    /// Key-entry marker: this shape never opens a browser.
    public static let requiresBrowser = false

    /// There is no callback route on this shape — pinned so a future redirect
    /// addition must consciously reopen ADR 0014 instead of slipping in.
    public static let hasLoopbackCallback = false
}
