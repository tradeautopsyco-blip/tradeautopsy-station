import Foundation
import Security

/// Loopback auth secret shared between Station and the spawned `tradeautopsy-agent`.
public enum AgentDaemonSecret {
    public static let envKey = "AGENT_DAEMON_SECRET"
    private static let legacyEnvKey = "AGENT_SECRET"

    /// Resolves the secret for this Station session.
    /// Uses an explicit env override when set; otherwise generates an ephemeral value per launch.
    public static func resolveForSession() -> String {
        if let fromEnv = ProcessInfo.processInfo.environment[envKey], !fromEnv.isEmpty {
            return fromEnv
        }
        if ProcessInfo.processInfo.environment["STATION_DEV_ATTACH"] == "1",
           let legacy = ProcessInfo.processInfo.environment[legacyEnvKey], !legacy.isEmpty {
            return legacy
        }
        return generateEphemeral()
    }

    public static func generateEphemeral() -> String {
        var bytes = [UInt8](repeating: 0, count: 32)
        let status = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
        precondition(status == errSecSuccess, "SecRandomCopyBytes failed")
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}
