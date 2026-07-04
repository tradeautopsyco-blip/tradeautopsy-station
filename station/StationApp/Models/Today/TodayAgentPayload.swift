import Foundation

public struct TodayAgentPayload: Decodable, Equatable, Sendable {
    public let localDate: String
    public let performanceBasisNotTax: Bool
    public let degradedReason: String?
    public let learningBaseline: Bool
    public let hero: TodayHeroPayload
    public let topSignals: [TodaySignalPayload]
    public let trades: [TodayTradeRowPayload]
    public let openPositionCount: Int
}

public struct TodayHeroPayload: Decodable, Equatable, Sendable {
    public let pnlTodayUsd: Double?
    public let tradesToday: Int?
    public let winRate: Double?
}

public struct TodaySignalPayload: Decodable, Equatable, Sendable {
    public let kind: String
    public let severity: String
    public let name: String
    public let description: String
}

public struct TodayTradeRowPayload: Decodable, Equatable, Sendable {
    public let closedAt: String
    public let symbol: String
    public let avgEntry: Double
    public let avgExit: Double
    public let qty: Double
    public let netPnlUsd: Double?
    public let primaryFlag: String
    public let flagSeverity: String
    public let dataQualityFlags: [String]
}
