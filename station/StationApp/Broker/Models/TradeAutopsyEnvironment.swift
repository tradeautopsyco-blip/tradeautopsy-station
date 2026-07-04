import Foundation

public enum TradeAutopsyEnvironment: String, Equatable, Sendable, CaseIterable, Codable {
    case dev
    case staging
    case prod

    public var displayName: String {
        switch self {
        case .dev: return "Dev"
        case .staging: return "Staging"
        case .prod: return "Prod"
        }
    }
}

public enum InternalBuildGate {
    public static func environmentSwitchingEnabled(
        processInfo: [String: String] = ProcessInfo.processInfo.environment
    ) -> Bool {
        #if DEBUG
        return true
        #else
        return processInfo["STATION_INTERNAL"] == "1"
        #endif
    }
}
