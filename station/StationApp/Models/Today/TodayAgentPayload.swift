import Foundation
import Notch

public struct TodayAgentPayload: Decodable, Equatable, Sendable {
    public let localDate: String
    public let performanceBasisNotTax: Bool
    public let degradedReason: String?
    public let learningBaseline: Bool
    public let hero: TodayHeroPayload
    public let topSignals: [TodaySignalPayload]
    public let trades: [TodayTradeRowPayload]
    public let openPositionCount: Int
    /// Active sync desk honesty (R7). Optional for backward-compatible decode.
    public let brokerSlug: String?
    public let quoteCurrency: String?
    public let calcProfileId: String?

    public init(
        localDate: String,
        performanceBasisNotTax: Bool,
        degradedReason: String?,
        learningBaseline: Bool,
        hero: TodayHeroPayload,
        topSignals: [TodaySignalPayload],
        trades: [TodayTradeRowPayload],
        openPositionCount: Int,
        brokerSlug: String? = nil,
        quoteCurrency: String? = nil,
        calcProfileId: String? = nil
    ) {
        self.localDate = localDate
        self.performanceBasisNotTax = performanceBasisNotTax
        self.degradedReason = degradedReason
        self.learningBaseline = learningBaseline
        self.hero = hero
        self.topSignals = topSignals
        self.trades = trades
        self.openPositionCount = openPositionCount
        self.brokerSlug = brokerSlug
        self.quoteCurrency = quoteCurrency
        self.calcProfileId = calcProfileId
    }

    /// Effective quote currency: payload field, else catalog slug map.
    public var deskQuoteCurrency: String? {
        if let quoteCurrency, !quoteCurrency.isEmpty { return quoteCurrency.uppercased() }
        return DeskMoneyFormatting.quoteCurrency(forBrokerSlug: brokerSlug)
    }
}

public struct TodayHeroPayload: Decodable, Equatable, Sendable {
    public let pnlTodayUsd: Double?
    public let tradesToday: Int?
    public let winRate: Double?

    public init(pnlTodayUsd: Double?, tradesToday: Int?, winRate: Double?) {
        self.pnlTodayUsd = pnlTodayUsd
        self.tradesToday = tradesToday
        self.winRate = winRate
    }
}

public struct TodaySignalPayload: Decodable, Equatable, Sendable {
    public let kind: String
    public let severity: String
    public let name: String
    public let description: String

    public init(kind: String, severity: String, name: String, description: String) {
        self.kind = kind
        self.severity = severity
        self.name = name
        self.description = description
    }
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

    public init(
        closedAt: String,
        symbol: String,
        avgEntry: Double,
        avgExit: Double,
        qty: Double,
        netPnlUsd: Double?,
        primaryFlag: String,
        flagSeverity: String,
        dataQualityFlags: [String]
    ) {
        self.closedAt = closedAt
        self.symbol = symbol
        self.avgEntry = avgEntry
        self.avgExit = avgExit
        self.qty = qty
        self.netPnlUsd = netPnlUsd
        self.primaryFlag = primaryFlag
        self.flagSeverity = flagSeverity
        self.dataQualityFlags = dataQualityFlags
    }
}
