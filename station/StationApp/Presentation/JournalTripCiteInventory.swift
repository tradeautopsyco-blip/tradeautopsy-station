import Foundation

/// One closed Station round-trip row keyed for Journal cite (Today / inventory owner).
public struct JournalClosedTripInventoryRow: Equatable, Sendable {
    public let declarationId: String
    public let net: Double
    public let currency: String

    public init(declarationId: String, net: Double, currency: String) {
        self.declarationId = declarationId
        self.net = net
        self.currency = currency
    }
}

public enum JournalTripCiteInventory {
    /// Maps inventory rows to Journal cites. Drops rows when desk currency is known and disagrees (DualNoBlend).
    public static func citedTrips(
        rows: [JournalClosedTripInventoryRow],
        deskQuoteCurrency: String?
    ) -> [JournalCitedTrip] {
        let desk = deskQuoteCurrency?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .uppercased()
        return rows.compactMap { row in
            let decl = row.declarationId.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !decl.isEmpty, row.net.isFinite else { return nil }
            let ccy = row.currency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            guard !ccy.isEmpty else { return nil }
            if let desk, !desk.isEmpty, desk != ccy { return nil }
            return JournalCitedTrip(declarationId: decl, net: row.net, currency: ccy)
        }
    }

    /// Parses `YYYY-MM-DD` from journal week bounds (ISO timestamps tolerated).
    public static func localDayFromWeekBound(_ bound: String?) -> String? {
        guard let bound else { return nil }
        let trimmed = bound.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.count >= 10, trimmed[trimmed.index(trimmed.startIndex, offsetBy: 4)] == "-" {
            return String(trimmed.prefix(10))
        }
        return nil
    }
}
