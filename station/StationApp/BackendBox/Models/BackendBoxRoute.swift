import Foundation

public enum BackendBoxRoute: String, CaseIterable, Codable, Hashable {
    case brokers = "Brokers"
    case health = "Health"
    case marketData = "Market Data"
    case aiWorkflow = "AI / Workflow"

    public var stationRoute: StationRoute {
        switch self {
        case .brokers: return .brokers
        case .health: return .health
        case .marketData: return .marketData
        case .aiWorkflow: return .aiWorkflow
        }
    }

    public var sfSymbol: String {
        switch self {
        case .brokers: return "link"
        case .health: return "heart.text.square"
        case .marketData: return "chart.line.uptrend.xyaxis"
        case .aiWorkflow: return "sparkles"
        }
    }

    public static func from(stationRoute: StationRoute) -> BackendBoxRoute? {
        switch stationRoute {
        case .brokers: return .brokers
        case .health: return .health
        case .marketData: return .marketData
        case .aiWorkflow: return .aiWorkflow
        default: return nil
        }
    }
}
