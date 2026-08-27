import Foundation

/// Pre-trade asset class — orthogonal to Intraday / Swing / Scalper style.
enum BarDeclareAssetClass: String, CaseIterable, Identifiable {
    case spot
    case equity
    case options

    var id: String { rawValue }

    var label: String {
        switch self {
        case .spot: return "Spot"
        case .equity: return "Equity"
        case .options: return "Options"
        }
    }
}

/// One Station extract hole on the declare form.
enum BarDeskInstrumentKind: String, Equatable {
    case last
    case history
    case chain
    case openInterest

    var title: String {
        switch self {
        case .last: return "Last"
        case .history: return "History"
        case .chain: return "Chain"
        case .openInterest: return "OI"
        }
    }
}

/// Kotak TickBook identity — `nse_cm|2885`, never a Binance pair or `"NSE"`.
enum InstrumentTickBookId {
    static let kotakCashSegments: Set<String> = ["nse_cm", "bse_cm"]

    /// Cash `segment|token` for Kotak REST/TickBook. Nil for Binance (token 0 / SPOT).
    static func make(segment: String?, instrumentToken: Int64?) -> String? {
        guard let token = instrumentToken, token > 0 else { return nil }
        let seg = (segment ?? "")
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        guard kotakCashSegments.contains(seg) else { return nil }
        return "\(seg)|\(token)"
    }

    /// Encode `|` so `/api/station/quote?instrument=` survives URL parsing.
    static func queryEncode(_ raw: String) -> String {
        var allowed = CharacterSet.urlQueryAllowed
        allowed.remove(charactersIn: "|&+")
        return raw.addingPercentEncoding(withAllowedCharacters: allowed) ?? raw
    }
}

/// Quote vs account health from `GET /api/daemon/broker/sync-state` `capabilities`.
enum DeskCapabilityChrome {
    /// Account proof is funds/fills — never copied from quote.
    static func accountStatus(funds: String, fills: String) -> String {
        let fundsN = funds.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if !fundsN.isEmpty, fundsN != "unavailable" {
            return fundsN
        }
        let fillsN = fills.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if !fillsN.isEmpty {
            return fillsN
        }
        return "unavailable"
    }

    static func dotName(forStatus status: String) -> String {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "fresh":
            return "teal"
        case "stale", "unknown", "loading":
            return "amber"
        default:
            return "red"
        }
    }

    static func pillLabel(kind: String, status: String) -> String {
        "\(kind) · \(status)"
    }

    static func emptySearchHint(masterStatus: String?, connectedInstrumentDesk: Bool) -> String? {
        switch masterStatus?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "loading":
            return "Loading instruments…"
        case "unavailable" where connectedInstrumentDesk:
            return "Catalog failed"
        case "stale" where connectedInstrumentDesk:
            return "Catalog failed"
        default:
            return nil
        }
    }

    static func showsRetryInstruments(status: String) -> Bool {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "unavailable", "stale":
            return true
        default:
            return false
        }
    }
}

enum DeskCatalogAllowlist {
    static func isAllowedExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "kotak_neo", "kotak", "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    static func isBinanceDesk(_ slug: String?) -> Bool {
        switch slug.map(normalized) {
        case "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    static func filterSymbols(_ symbols: [InstrumentResult], deskSlug: String?) -> [InstrumentResult] {
        let allowed = symbols.filter { isAllowedExchange($0.exchange) }
        if BarDeskTemplate.isKotakNeoDesk(slug: deskSlug) {
            return allowed.filter { isKotakExchange($0.exchange) }
        }
        if isBinanceDesk(deskSlug) {
            return allowed.filter { isBinanceExchange($0.exchange) }
        }
        return allowed
    }

    static func refusesKotakSelection(_ result: InstrumentResult, deskSlug: String?) -> Bool {
        guard BarDeskTemplate.isKotakNeoDesk(slug: deskSlug) else { return false }
        return !isKotakExchange(result.exchange)
    }

    private static func isKotakExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "kotak_neo", "kotak":
            return true
        default:
            return false
        }
    }

    private static func isBinanceExchange(_ exchange: String) -> Bool {
        switch normalized(exchange) {
        case "binance_com", "binance":
            return true
        default:
            return false
        }
    }

    private static func normalized(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
    }
}

enum BarDeskTemplate {
    /// Widgets for this class. Chain/OI stay on the options template even while extracts are unavailable.
    static func glanceKinds(for asset: BarDeclareAssetClass) -> [BarDeskInstrumentKind] {
        switch asset {
        case .spot, .equity:
            return [.last, .history]
        case .options:
            return [.last, .history, .chain, .openInterest]
        }
    }

    static func historyDetail(status: String, ineligible: [String]) -> String {
        if ineligible.contains("rights_forbid_canonical") {
            return "yahoo-shaped · rights_forbid_canonical"
        }
        if status == "unsupported" {
            return "unsupported"
        }
        if status == "unavailable" {
            return "no licensed series"
        }
        return status
    }

    /// One licensed History row. Yahoo is never the product line (COM or Kotak).
    static func historyGlanceLine(
        licensedStatus: String,
        licensedIneligible: [String],
        yahooStatus: String,
        yahooIneligible: [String],
        stitchYahoo: Bool
    ) -> String {
        let licensed = historyDetail(status: licensedStatus, ineligible: licensedIneligible)
        guard stitchYahoo else { return licensed }
        let yahoo = historyDetail(status: yahooStatus, ineligible: yahooIneligible)
        return "\(licensed) · \(yahoo)"
    }

    static func isKotakNeoDesk(slug: String?) -> Bool {
        switch slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "kotak_neo", "kotak":
            return true
        default:
            return false
        }
    }
}
