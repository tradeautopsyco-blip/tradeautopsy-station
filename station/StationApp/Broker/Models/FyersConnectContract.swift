import Foundation

/// ADR 0007 loopback redirect — must match Fyers app settings and agent `redirectUri` from connect/begin.
public enum FyersConnectContract {
    public static let loopbackRedirectURI =
        "http://127.0.0.1:\(AgentLoopback.port)/api/daemon/broker/fyers/callback"
}
