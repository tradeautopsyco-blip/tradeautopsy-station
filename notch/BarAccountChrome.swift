import Foundation

/// S8 Notch account chrome — obtain-backed pulse + ledger on the **shipping book of Start**.
/// Not Today fill-inventory (`GET /api/daemon/positions`). DualNoBlend: one book, one currency strip.
enum BarAccountChrome {
    struct Row: Equatable, Identifiable {
        var id: String
        var symbol: String
        var qty: String
        /// Venue `marginType` when the positionbook row carried it. Nil if flat or the envelope omitted it.
        var marginType: String? = nil
        /// Venue leverage string. Nil if flat — never invent `50x`.
        var leverage: String? = nil
        /// Venue `liquidationPrice`. Nil if flat.
        var liquidationPrice: String? = nil
    }

    struct Snapshot: Equatable {
        var bookId: String
        var bookLabel: String
        var freeText: String
        var fundsStatus: String
        var holdingsCount: Int
        var holdingsStatus: String
        var holdingsRows: [Row]
        var positionsCount: Int
        var positionsStatus: String
        var positionsRows: [Row]
        var ordersCount: Int
        var ordersStatus: String

        static let empty = Snapshot(
            bookId: "",
            bookLabel: "No book",
            freeText: "—",
            fundsStatus: "unavailable",
            holdingsCount: 0,
            holdingsStatus: "unavailable",
            holdingsRows: [],
            positionsCount: 0,
            positionsStatus: "unavailable",
            positionsRows: [],
            ordersCount: 0,
            ordersStatus: "unavailable"
        )
    }

    /// Start slug → shipping AccountBook only. Never NFO, never options, never USDM.
    static func shippingBookId(forStartSlug slug: String?) -> String? {
        switch normalized(slug) {
        case "kotak_neo", "kotak":
            return BarDeskTemplate.kotakCashBookId
        case "binance_com", "binance":
            return "binance-com-spot"
        default:
            return nil
        }
    }

    /// Named futures book on COM. Nil for every other class/slug — Start chrome stays spot.
    static func namedBookId(forAssetClass assetClass: BarDeclareAssetClass, startSlug: String?) -> String? {
        switch normalized(startSlug) {
        case "binance_com", "binance":
            break
        default:
            return nil
        }
        switch assetClass {
        case .usdm:
            return BarDeskTemplate.binanceComUsdmBookId
        case .coinm:
            return BarDeskTemplate.binanceComCoinmBookId
        default:
            return nil
        }
    }

    /// Pulse/ledger book for this declare class. USDM / Coin-M never fall back to Start spot.
    static func pulseBookId(forAssetClass assetClass: BarDeclareAssetClass, startSlug: String?) -> String? {
        if assetClass == .usdm || assetClass == .coinm {
            return namedBookId(forAssetClass: assetClass, startSlug: startSlug)
        }
        return shippingBookId(forStartSlug: startSlug)
    }

    static func obtainAdapterId(forStartSlug slug: String?) -> String? {
        switch normalized(slug) {
        case "kotak_neo", "kotak":
            return "kotak_neo"
        case "binance_com", "binance":
            return "binance_com"
        default:
            return nil
        }
    }

    static func obtainPath(adapter: String, bookId: String, operation: String) -> String {
        let a = adapter.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? adapter
        let b = bookId.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? bookId
        let o = operation.addingPercentEncoding(withAllowedCharacters: .urlQueryAllowed) ?? operation
        return "/api/station/obtain?adapter=\(a)&book=\(b)&operation=\(o)"
    }

    /// Compose pulse + ledger. Success envelopes whose `book_id` is not the pulse book are dropped (DualNoBlend).
    static func compose(
        shippingBookId: String,
        quoteCurrency: String,
        funds: [String: Any]?,
        holdings: [String: Any]?,
        positions: [String: Any]?,
        orders: [String: Any]?
    ) -> Snapshot {
        let fundsKept = keepIfShipping(funds, bookId: shippingBookId)
        let holdingsKept = keepIfShipping(holdings, bookId: shippingBookId)
        let positionsKept = keepIfShipping(positions, bookId: shippingBookId)
        let ordersKept = keepIfShipping(orders, bookId: shippingBookId)

        let fundsStatus = status(of: fundsKept)
        let holdingsStatus = status(of: holdingsKept)
        let positionsStatus = status(of: positionsKept)
        let ordersStatus = status(of: ordersKept)

        let assetFree = freeAsset(from: fundsKept, preferred: quoteCurrency)
        let holdingsRows = symbolRows(from: holdingsKept, qtyKeys: ["quantity", "qty"])
        let positionsRows = symbolRows(from: positionsKept, qtyKeys: ["net_qty", "qty"])
        let orderCount = intField(ordersKept, "order_count") ?? rows(of: ordersKept).count

        return Snapshot(
            bookId: shippingBookId,
            bookLabel: bookLabel(bookId: shippingBookId, quoteCurrency: quoteCurrency),
            freeText: formatFree(assetFree),
            fundsStatus: fundsStatus,
            holdingsCount: intField(holdingsKept, "holding_count") ?? holdingsRows.count,
            holdingsStatus: holdingsStatus,
            holdingsRows: holdingsRows,
            positionsCount: intField(positionsKept, "position_count") ?? positionsRows.count,
            positionsStatus: positionsStatus,
            positionsRows: positionsRows,
            ordersCount: orderCount,
            ordersStatus: ordersStatus
        )
    }

    static func ordersCountLabel(_ snap: Snapshot) -> String {
        switch snap.ordersStatus {
        case "success":
            return "\(snap.ordersCount) orders"
        case "unsupported":
            return "orders unsupported"
        default:
            return "orders unavailable"
        }
    }

    static func listStatusLine(kind: String, status: String) -> String? {
        switch status {
        case "success":
            return nil
        case "unsupported":
            return "\(kind) unsupported"
        default:
            return "\(kind) unavailable"
        }
    }

    // MARK: - private

    private static func normalized(_ slug: String?) -> String {
        slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
    }

    private static func keepIfShipping(_ envelope: [String: Any]?, bookId: String) -> [String: Any]? {
        guard let envelope else { return nil }
        guard let got = envelope["book_id"] as? String, !got.isEmpty else { return envelope }
        return got == bookId ? envelope : nil
    }

    private static func status(of envelope: [String: Any]?) -> String {
        let raw = (envelope?["status"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased()
        if let raw, !raw.isEmpty { return raw }
        return "unavailable"
    }

    private static func data(of envelope: [String: Any]?) -> [String: Any]? {
        envelope?["data"] as? [String: Any]
    }

    /// USDM income slot when the funds envelope named it. Missing is none — never invent `0`.
    static func realizedPnl(from funds: [String: Any]?) -> Double? {
        let d = data(of: funds)
        if let n = d?["realized_pnl"] as? Double { return n }
        if let n = d?["realized_pnl"] as? Int { return Double(n) }
        return nil
    }

    private static func intField(_ envelope: [String: Any]?, _ key: String) -> Int? {
        let d = data(of: envelope)
        if let n = d?[key] as? Int { return n }
        if let n = d?[key] as? Double { return Int(n) }
        return nil
    }

    private static func rows(of envelope: [String: Any]?) -> [[String: Any]] {
        (data(of: envelope)?["rows"] as? [[String: Any]]) ?? []
    }

    private static func freeAsset(from funds: [String: Any]?, preferred: String) -> (asset: String, free: Double)? {
        let holdings = (data(of: funds)?["holdings"] as? [[String: Any]]) ?? []
        guard !holdings.isEmpty else { return nil }
        let want = preferred.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let picked = holdings.first { row in
            (row["asset"] as? String)?.uppercased() == want
        } ?? holdings[0]
        let asset = (picked["asset"] as? String)?.trimmingCharacters(in: .whitespacesAndNewlines) ?? want
        let free: Double
        if let n = picked["free"] as? Double {
            free = n
        } else if let n = picked["free"] as? Int {
            free = Double(n)
        } else {
            return nil
        }
        return (asset, free)
    }

    private static func formatFree(_ pair: (asset: String, free: Double)?) -> String {
        guard let pair else { return "—" }
        let f = NumberFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.numberStyle = .decimal
        f.minimumFractionDigits = 2
        f.maximumFractionDigits = 2
        let n = f.string(from: NSNumber(value: pair.free)) ?? String(format: "%.2f", pair.free)
        return "\(pair.asset) \(n)"
    }

    private static func bookLabel(bookId: String, quoteCurrency: String) -> String {
        let kind: String
        switch bookId {
        case BarDeskTemplate.kotakCashBookId:
            kind = "Cash"
        case "binance-com-spot":
            kind = "Spot"
        case BarDeskTemplate.binanceComUsdmBookId:
            kind = "USDM"
        case BarDeskTemplate.binanceComCoinmBookId:
            kind = "Coin-M"
        default:
            kind = bookId
        }
        let ccy = quoteCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        if ccy.isEmpty { return kind }
        return "\(kind) · \(ccy)"
    }

    private static func symbolRows(from envelope: [String: Any]?, qtyKeys: [String]) -> [Row] {
        rows(of: envelope).enumerated().compactMap { idx, row in
            let symbol = (row["symbol"] as? String)?
                .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            guard !symbol.isEmpty else { return nil }
            let qty = qtyKeys.compactMap { key -> String? in
                if let n = row[key] as? Double { return formatQty(n) }
                if let n = row[key] as? Int { return formatQty(Double(n)) }
                if let s = row[key] as? String, !s.isEmpty { return s }
                return nil
            }.first ?? "—"
            return Row(
                id: "\(symbol)#\(idx)",
                symbol: symbol,
                qty: qty,
                marginType: stringField(row, ["marginType", "margin_type"]),
                leverage: stringField(row, ["leverage"]),
                liquidationPrice: stringField(row, ["liquidationPrice", "liquidation_price"])
            )
        }
    }

    /// Cross / x / liq from a named-book position row. All nil when flat or the envelope omitted the fields.
    static func futuresMargin(from snap: Snapshot, symbol: String) -> (mode: String?, leverage: String?, liq: String?) {
        guard snap.positionsStatus == "success", snap.positionsCount > 0 else {
            return (nil, nil, nil)
        }
        let want = symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let row = snap.positionsRows.first { $0.symbol.uppercased() == want } ?? snap.positionsRows.first
        guard let row else { return (nil, nil, nil) }
        return (row.marginType, row.leverage, row.liquidationPrice)
    }

    private static func stringField(_ row: [String: Any], _ keys: [String]) -> String? {
        for key in keys {
            if let s = row[key] as? String {
                let t = s.trimmingCharacters(in: .whitespacesAndNewlines)
                if !t.isEmpty { return t }
            }
            if let n = row[key] as? Int { return String(n) }
            if let n = row[key] as? Double {
                if n == n.rounded() { return String(Int(n.rounded())) }
                return String(n)
            }
        }
        return nil
    }

    private static func formatQty(_ n: Double) -> String {
        if n == n.rounded() {
            return String(Int(n.rounded()))
        }
        let f = NumberFormatter()
        f.locale = Locale(identifier: "en_US_POSIX")
        f.numberStyle = .decimal
        f.minimumFractionDigits = 0
        f.maximumFractionDigits = 8
        return f.string(from: NSNumber(value: n)) ?? String(n)
    }
}
