import Foundation

/// Maps agent `POST /api/daemon/risk/preview` + local ladder fallbacks for Plan rail.
enum BarPlanRiskPreview {
    enum BudgetMode: String, CaseIterable, Sendable {
        case riskPercent = "risk_percent"
        case fixedMoney = "fixed_money"
    }

    struct Request: Equatable, Sendable {
        var bookId: String
        var sideBuy: Bool
        var budgetMode: BudgetMode
        var budgetValue: Double?
        var entry: Double?
        var stop: Double?
        var target: Double?
        var overrideQty: Double?
        var fundsLit: Bool
        var fundsBalance: Double?
        var fundsDisplayText: String?
        var priceIncrement: Double?
        var multiplier: Double?
        var unitBatchSize: Double?
        var instrumentRole: String
        var leverage: Double?
        var symbol: String
    }

    static func freeAmount(from text: String?) -> Double? {
        let raw = (text ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        guard !raw.isEmpty, raw != "—" else { return nil }
        let token = raw.split(separator: " ").last.map(String.init) ?? raw
        let cleaned = token.replacingOccurrences(of: ",", with: "")
        guard let n = Double(cleaned), n.isFinite, n > 0 else { return nil }
        return n
    }

    static func formatQty(_ n: Double) -> String {
        guard n.isFinite, n > 0 else { return "—" }
        if abs(n - n.rounded()) < 0.000_000_1 {
            return String(format: "%.0f", n)
        }
        var s = String(format: "%.8f", n)
        while s.last == "0" { s.removeLast() }
        if s.last == "." { s.removeLast() }
        return s
    }

    struct Presentation: Equatable, Sendable {
        var balanceText: String
        var riskText: String
        var feesText: String
        var rewardText: String
        var rrExFeesText: String
        var rrInFeesText: String
        var proposedSizeText: String
        var proposedSizeDashed: Bool
        var footnote: String
    }

    static func localPresentation(_ req: Request, quoteCurrency: String) -> Presentation {
        let ccy = quoteCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let balance: String = {
            guard req.fundsLit else { return "— · funds unavailable" }
            let text = req.fundsDisplayText?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            if !text.isEmpty, text != "—" { return text }
            guard let bal = req.fundsBalance, bal.isFinite, bal > 0 else { return "—" }
            return DeskMoneyFormatting.formatWhole(bal, quoteCurrency: ccy.isEmpty ? quoteCurrency : ccy)
        }()

        let risk: String = {
            guard let qty = req.overrideQty, qty > 0,
                  let entry = req.entry, let stop = req.stop
            else { return "—" }
            let loss = BarPlanLadder.maxPlannedLossINR(
                units: qty,
                entry: entry,
                stop: stop,
                sideBuy: req.sideBuy
            )
            guard let loss else { return "—" }
            return DeskMoneyFormatting.formatWhole(loss, quoteCurrency: ccy.isEmpty ? quoteCurrency : ccy)
        }()

        let rr: String = {
            guard let entry = req.entry, let stop = req.stop, let target = req.target,
                  let ratio = BarIntradayDeclareValidator.riskRewardRatio(
                      entry: entry,
                      stop: stop,
                      target: target,
                      sideBuy: req.sideBuy
                  )
            else { return "—" }
            return String(format: "%.2f", ratio)
        }()

        let reward: String = {
            guard let entry = req.entry, let target = req.target else { return "—" }
            let raw = req.sideBuy ? (target - entry) : (entry - target)
            guard raw.isFinite, raw > 0, let qty = req.overrideQty, qty > 0 else { return "—" }
            let money = raw * qty
            return DeskMoneyFormatting.formatWhole(money, quoteCurrency: ccy.isEmpty ? quoteCurrency : ccy)
        }()

        return Presentation(
            balanceText: balance,
            riskText: risk,
            feesText: "— · fee schedule unspecified",
            rewardText: reward,
            rrExFeesText: rr,
            rrInFeesText: "—",
            proposedSizeText: "—",
            proposedSizeDashed: true,
            footnote: BarPlanSizer.formulaBlockedFootnote
        )
    }

    static func presentation(
        fromAgent json: [String: Any],
        localFallback: Presentation
    ) -> Presentation {
        func text(_ key: String, dash: String = "—") -> String {
            if json[key] is NSNull { return dash }
            if let n = json[key] as? Double, n.isFinite { return String(format: "%.2f", n) }
            if let s = json[key] as? String, !s.isEmpty { return s }
            return dash
        }
        let feesReason = json["fees_reason"] as? String
        let feesText: String = {
            if feesReason == "fee_schedule_unspecified" {
                return "— · fee schedule unspecified"
            }
            return text("fees_money")
        }()
        let authored = json["authored_qty"] as? Double
        let proposedDashed = authored.map { !$0.isFinite || $0 <= 0 } ?? true
        let proposed = authored.flatMap { proposedDashed ? nil : formatQty($0) } ?? "—"
        let reason = json["authored_qty_reason"] as? String ?? json["reason"] as? String
        return Presentation(
            balanceText: localFallback.balanceText,
            riskText: text("risk_money", dash: localFallback.riskText),
            feesText: feesText,
            rewardText: localFallback.rewardText,
            rrExFeesText: text("rr_ex_fees", dash: localFallback.rrExFeesText),
            rrInFeesText: "—",
            proposedSizeText: proposed,
            proposedSizeDashed: proposedDashed,
            footnote: footnote(for: reason)
        )
    }

    static func footnote(for reason: String?) -> String {
        switch reason {
        case "ok":
            return "Fixed risk from the stop. Commission 0. This book only."
        case "funds_dark":
            return "Funds dark — no authored size."
        case "price_increment_unspecified":
            return "Tick unknown — size stays —."
        case "multiplier_unspecified":
            return "Lot unknown — size stays —."
        case "coinm_identity_unspecified":
            return "Coin-M sizing is not in the locked formula."
        case "option_premium_unspecified":
            return "Option premium risk is not in the locked formula."
        case "option_short_max_loss_unspecified":
            return "Short option max loss is not in the locked formula."
        case "fixed_money_unspecified":
            return "Fixed amount is not the locked formula. Type a risk %."
        case "margin_gate_exceeded":
            return "That size fails the margin check. No quantity applied."
        case "below_unit_batch":
            return "Risk is below one unit."
        case "profile_not_locked":
            return "This book is not on the locked sizing list."
        case "instrument_role_required":
            return "Future versus option is unknown — size stays —."
        case "risk_pct_required":
            return "Type a risk %. There is no default."
        default:
            return BarPlanSizer.formulaBlockedFootnote
        }
    }

    static func requestJSON(_ req: Request) -> [String: Any] {
        var o: [String: Any] = [
            "book_id": req.bookId,
            "side": req.sideBuy ? "BUY" : "SELL",
            "budget_mode": req.budgetMode.rawValue,
            "funds_lit": req.fundsLit,
        ]
        if let v = req.budgetValue { o["budget_value"] = v }
        if let v = req.entry { o["entry"] = v }
        if let v = req.stop { o["stop"] = v }
        if let v = req.target { o["target"] = v }
        if let v = req.overrideQty { o["override_qty"] = v }
        if let v = req.fundsBalance { o["funds_balance"] = v }
        if let v = req.priceIncrement { o["price_increment"] = v }
        if let v = req.multiplier { o["multiplier"] = v }
        if let v = req.unitBatchSize { o["unit_batch_size"] = v }
        if !req.instrumentRole.isEmpty { o["instrument_role"] = req.instrumentRole }
        if let v = req.leverage { o["leverage"] = v }
        if !req.symbol.isEmpty { o["symbol"] = req.symbol }
        return o
    }
}
