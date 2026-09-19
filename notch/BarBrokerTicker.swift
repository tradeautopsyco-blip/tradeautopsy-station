import Foundation

/// NSE wire ticker for Bar declare / place_sl — never Kotak company display names.
enum BarBrokerTicker {
    /// Prefer `trdSym`, then `tradingSymbol`, then `raw`. Returns uppercased token (e.g. RELIANCE).
    static func normalize(
        raw: String?,
        tradingSymbol: String? = nil,
        trdSym: String? = nil,
    ) -> String? {
        for candidate in [trdSym, tradingSymbol, raw] {
            guard let c = candidate else { continue }
            if let t = normalizeSingle(c) { return t }
        }
        return nil
    }

    /// Normalize a position/holding JSON row from `GET /api/daemon/positions`.
    static func fromPositionRow(_ row: [String: Any]) -> String? {
        normalize(
            raw: row["symbol"] as? String,
            tradingSymbol: row["tradingSymbol"] as? String,
            trdSym: row["trdSym"] as? String,
        )
    }

    private static func normalizeSingle(_ input: String) -> String? {
        var s = input.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        if s.isEmpty { return nil }

        if let eqRange = s.range(of: "-EQ", options: [.caseInsensitive, .backwards]) {
            s = String(s[..<eqRange.lowerBound])
        }
        s = s.trimmingCharacters(in: .whitespacesAndNewlines)
        if s.isEmpty { return nil }

        // Kotak holdings often ship instrument/company names — not matchable to fills.
        if s.contains(" ") { return nil }

        guard s.count >= 2, s.count <= 24 else { return nil }

        var allowed = CharacterSet.alphanumerics
        allowed.insert(charactersIn: "&.-_")
        guard s.unicodeScalars.allSatisfy({ allowed.contains($0) }) else { return nil }

        return s
    }
}
