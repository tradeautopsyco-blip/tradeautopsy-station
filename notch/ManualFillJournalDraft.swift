import Foundation

/// Builds manual-fill journal capture draft + request body (mirrors agent `journal_manual_fill.rs`).
enum ManualFillJournalDraft {
    static let sourceManual = "manual"

    struct FormDefaults: Equatable {
        var symbol: String
        var sideBuy: Bool
        var quantity: String
        var price: String
        var filledAt: Date
        var declarationId: String?
    }

    static func defaults(
        pending: BarPendingDeclaration?,
        fallbackSymbol: String,
        fallbackDeclarationId: String?
    ) -> FormDefaults {
        let symFromDeclare = pending?.symbol.trimmingCharacters(in: .whitespacesAndNewlines)
            ?? fallbackSymbol.trimmingCharacters(in: .whitespacesAndNewlines)
        let sideBuy: Bool = {
            let s = pending?.side.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
            if !s.isEmpty { return !s.contains("SELL") }
            return true
        }()
        let qty: String = {
            if let q = pending?.quantity, q > 0 { return String(Int(q)) }
            if let q = pending?.filledQty, q > 0 { return String(Int(q)) }
            return ""
        }()
        let declId = pending?.id.trimmingCharacters(in: .whitespacesAndNewlines)
        let resolvedDecl: String? = (declId?.isEmpty == false) ? declId : fallbackDeclarationId
        return FormDefaults(
            symbol: symFromDeclare,
            sideBuy: sideBuy,
            quantity: qty,
            price: "",
            filledAt: Date(),
            declarationId: resolvedDecl
        )
    }

    static func buildDraftText(
        symbol: String,
        sideBuy: Bool,
        quantity: Double,
        price: Double,
        filledAtMs: Int64,
        tradeId: String,
        declarationId: String?
    ) -> String {
        let side = sideBuy ? "BUY" : "SELL"
        let qtyText = quantity.rounded() == quantity ? String(Int(quantity)) : String(format: "%.4f", quantity)
        let human = String(format: "manual fill · %@ %@ %@ @ %.2f", symbol.uppercased(), side, qtyText, price)
        var payload: [String: Any] = [
            "journalFillSource": sourceManual,
            "v": 1,
            "symbol": symbol.uppercased(),
            "side": side,
            "quantity": quantity,
            "price": price,
            "filledAtMs": filledAtMs,
            "stationFillId": tradeId,
        ]
        if let declarationId, UUID(uuidString: declarationId) != nil {
            payload["declarationId"] = declarationId
        }
        guard
            let jsonData = try? JSONSerialization.data(withJSONObject: payload, options: []),
            let jsonLine = String(data: jsonData, encoding: .utf8)
        else {
            return human
        }
        return human + "\n" + jsonLine
    }

    static func requestBody(
        symbol: String,
        sideBuy: Bool,
        quantity: Double,
        price: Double,
        filledAtMs: Int64,
        declarationId: String?,
        idempotencyKey: String,
        consoleTradeId: String?
    ) -> [String: Any] {
        var body: [String: Any] = [
            "symbol": symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased(),
            "side": sideBuy ? "BUY" : "SELL",
            "quantity": quantity,
            "price": price,
            "filledAtMs": filledAtMs,
            "idempotencyKey": idempotencyKey,
        ]
        if let declarationId, UUID(uuidString: declarationId) != nil {
            body["preTradeDeclarationId"] = declarationId
        }
        if let consoleTradeId, UUID(uuidString: consoleTradeId) != nil {
            body["consoleTradeId"] = consoleTradeId
        }
        return body
    }

    /// Console `trades.id` rows from daemon `GET /api/daemon/journal/toolbar/recent-trades`.
    static func consoleTrades(from json: [String: Any], on day: Date = Date(), calendar: Calendar = .current) -> [ConsoleJournalTradeRow] {
        let data = json["data"] as? [String: Any]
        let rows = (data?["trades"] as? [[String: Any]]) ?? (json["trades"] as? [[String: Any]]) ?? []
        return rows.compactMap { row in
            let id = (row["trade_id"] as? String) ?? (row["id"] as? String) ?? ""
            guard UUID(uuidString: id) != nil else { return nil }
            let symbol = ((row["symbol"] as? String) ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            guard !symbol.isEmpty else { return nil }
            if let stamp = row["timestamp"] as? String, let when = parseStamp(stamp) {
                guard calendar.isDate(when, inSameDayAs: day) else { return nil }
            }
            let side = (row["side"] as? String) ?? ""
            return ConsoleJournalTradeRow(id: id, symbol: symbol, side: side)
        }
    }

    static func presentAccept(httpStatus: Int, body: [String: Any]) -> ManualFillAcceptPresentation {
        let ok = body["success"] as? Bool ?? false
        let data = body["data"] as? [String: Any]
        let errCode = ((body["error"] as? [String: Any])?["code"] as? String)?
            .trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !ok, !errCode.isEmpty {
            return ManualFillAcceptPresentation(success: nil, error: "Console \(httpStatus) · \(errCode)")
        }
        let delivery = data?["delivery"] as? String
        let dataStatus = data?["status"] as? String
        if ok, httpStatus == 202 || delivery == "local_enqueue" || dataStatus == "queued" {
            return ManualFillAcceptPresentation(
                success: "Queued locally · Console not accepted yet",
                error: nil
            )
        }
        if ok, (200 ... 299).contains(httpStatus) {
            let st = dataStatus ?? "accepted"
            let pending = (data?["pending_capture_id"] as? String).map { String($0.prefix(8)) }
            let tail = pending.map { " · \($0)…" } ?? ""
            return ManualFillAcceptPresentation(
                success: "Console accept \(httpStatus) · \(st)\(tail)",
                error: nil
            )
        }
        return ManualFillAcceptPresentation(success: nil, error: "Queue failed (\(httpStatus))")
    }

    private static func parseStamp(_ raw: String) -> Date? {
        let frac = ISO8601DateFormatter()
        frac.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        if let d = frac.date(from: raw) { return d }
        let coarse = ISO8601DateFormatter()
        coarse.formatOptions = [.withInternetDateTime]
        return coarse.date(from: raw)
    }
}

struct ConsoleJournalTradeRow: Identifiable, Equatable {
    var id: String
    var symbol: String
    var side: String
}

struct ManualFillAcceptPresentation: Equatable {
    var success: String?
    var error: String?
}
