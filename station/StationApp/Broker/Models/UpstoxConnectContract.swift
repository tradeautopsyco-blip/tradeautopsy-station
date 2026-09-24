import Foundation

/// ADR 0006 loopback redirect — must match Upstox app settings and agent `redirectUri` from connect/begin.
public enum UpstoxConnectContract {
    public static let loopbackRedirectURI =
        "https://127.0.0.1:\(AgentLoopback.oauthTlsPort)/api/daemon/broker/upstox/callback"
}
