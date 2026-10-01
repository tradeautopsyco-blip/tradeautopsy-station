import Foundation

public struct AgentHealthWarning: Equatable {
    public enum Reason: Equatable {
        case launchTimeout
        case portCollisionNonAgent
        case crashLoopExceeded
        case runtimeDisconnected
        case killSwitchLatched
    }

    public var reason: Reason
    public var message: String
    public var logPath: URL?
    public var canRetry: Bool

    public init(reason: Reason, message: String, logPath: URL? = nil, canRetry: Bool) {
        self.reason = reason
        self.message = message
        self.logPath = logPath
        self.canRetry = canRetry
    }
}
