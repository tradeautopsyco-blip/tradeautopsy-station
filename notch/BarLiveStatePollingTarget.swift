import Foundation

/// Extract target for PLAN live-state polling. Agent loopback only — never hosted Console.
enum BarLiveStatePollingTarget {
    static let path = "/api/daemon/bar/live-state"

    static func url(agentBase: String) -> URL? {
        URL(string: agentBase + path)
    }

    static func isHostedConsoleLiveState(_ url: URL) -> Bool {
        url.path == "/api/bar/v1/live-state"
    }
}
