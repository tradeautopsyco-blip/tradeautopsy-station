import Foundation

/// ADR 0005 loopback redirect — must match Kite app settings and agent `redirectUri` from connect/begin.
public enum ZerodhaKiteConnectContract {
    public static let loopbackRedirectURI =
        "http://127.0.0.1:\(AgentLoopback.port)/api/daemon/broker/zerodha/callback"
}
