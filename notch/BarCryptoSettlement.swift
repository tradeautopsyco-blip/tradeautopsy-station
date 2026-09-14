import Foundation

/// NFO At-expiry copy. The three-zone surface keeps this hole; crypto settlement
/// shape must not rewrite it.
enum BarNfoPayoffCopy {
    static let holeTitle = "chain unavailable"
    static let holeBody = "derived/payoff inherits market/option_chain"
}

/// NFO `derived/greeks` copy. The three-zone surface never paints Δ/Γ/Θ — chips
/// only, from the glance envelope. Legs are a local plan and do not gate this.
enum BarNfoGreeksCopy {
    static let pricingModelUnspecified = "pricing_model_unspecified"

    static func provenance(asked: Bool, status: String, ineligible: [String]) -> String {
        if ineligible.contains(pricingModelUnspecified) {
            return "derived/greeks · unavailable — pricing_model_unspecified. Named inputs market/option_chain and reference/derivative_contracts are lit. Station has no trader model (OPTIONS-PRICING.md). A Greek from memory is a lie, so this stays empty."
        }
        if asked {
            if HonestyStatus.fromWire(status) == .inheritedDark {
                return "derived/greeks · inherited dark — named inputs market/option_chain and reference/derivative_contracts (missing F&O master). A Greek from a missing master is a lie, so this stays empty."
            }
            let wire = status.trimmingCharacters(in: .whitespacesAndNewlines)
            let shown = wire.isEmpty ? "unavailable" : wire
            return "derived/greeks · \(shown) — NFO greeks stay dark until OPTIONS-PRICING.md names a trader model."
        }
        return "derived/greeks — waiting on a declared contract."
    }
}

/// NFO session / derived OHLCV copy. Kotak has no history capability; Station
/// does not compose coarser bars from a finer series.
enum BarNfoHistoryCopy {
    static let sessionHoleTitle = "no licensed series"
    static let sessionHoleBody =
        "derived/ohlcv · kotak_history_unsupported. Kotak history is unsupported; Station does not compose coarser bars from 1m."
}

/// European cash-settled P&L identity for Binance options At-expiry.
///
/// Lock: `locks/binance-com-options.md` — S is eapi `indexPrice`, not spot last,
/// not markPrice, not ModelComputed. Y is USDT **per contract**; `unit` is not applied.
enum BarCryptoSettlement {
    enum Right: Equatable {
        case call
        case put

        static func fromWire(_ raw: String) -> Right {
            let t = raw.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            if t == "PE" || t == "P" || t == "PUT" { return .put }
            return .call
        }
    }

    enum Side: Equatable {
        case buy
        case sell
    }

    struct Point: Equatable {
        var s: Double
        var pnl: Double
    }

    enum Kind: Equatable {
        case hole(reason: String)
        case lit(Lit)
    }

    struct Lit: Equatable {
        var points: [Point]
        var spotS: Double
        var openWing: Bool
        /// Long max loss is the premium paid. Short call has none (not a number).
        var longMaxLoss: Double?
        var caption: String
        var dte: Int
        var dteKnown: Bool
    }

    /// Calendar DTE: UTC date difference, floored at 0 after expiry.
    static func calendarDTE(expiryDateMs: Int64, nowMs: Int64) -> Int {
        let expiry = Date(timeIntervalSince1970: TimeInterval(expiryDateMs) / 1000)
        let now = Date(timeIntervalSince1970: TimeInterval(nowMs) / 1000)
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(secondsFromGMT: 0)!
        let startExpiry = cal.startOfDay(for: expiry)
        let startNow = cal.startOfDay(for: now)
        let days = cal.dateComponents([.day], from: startNow, to: startExpiry).day ?? 0
        return max(0, days)
    }

    /// YYMMDD second segment of `BTC-200730-9000-C` → UTC midnight ms.
    static func expiryDateMs(fromDatedContract id: String) -> Int64? {
        let parts = id.trimmingCharacters(in: .whitespacesAndNewlines).split(separator: "-")
        guard parts.count >= 3, parts[1].count == 6, parts[1].allSatisfy(\.isNumber) else {
            return nil
        }
        let raw = String(parts[1])
        guard let yy = Int(raw.prefix(2)),
              let mm = Int(raw.dropFirst(2).prefix(2)),
              let dd = Int(raw.suffix(2)),
              (1 ... 12).contains(mm),
              (1 ... 31).contains(dd)
        else { return nil }
        var comps = DateComponents()
        comps.year = 2000 + yy
        comps.month = mm
        comps.day = dd
        comps.hour = 0
        comps.minute = 0
        comps.second = 0
        var cal = Calendar(identifier: .gregorian)
        cal.timeZone = TimeZone(secondsFromGMT: 0)!
        guard let date = cal.date(from: comps) else { return nil }
        return Int64(date.timeIntervalSince1970 * 1000)
    }

    /// Typed USDT premium — same precision rules as Last. Do not `%.2f` a sub-cent
    /// premium to `0.00`.
    static func parsePremium(_ text: String) -> Double? {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty, let value = Double(t), value.isFinite, value >= 0 else { return nil }
        return value
    }

    static func formatUsdt(_ value: Double) -> String {
        let magnitude = abs(value)
        var places = 2
        if magnitude > 0, magnitude < 1 {
            places = min(8, max(2, 3 - Int(floor(log10(magnitude)))))
        }
        var text = String(format: "%.\(places)f", magnitude)
        while places > 2, text.hasSuffix("0") {
            text.removeLast()
            places -= 1
        }
        if magnitude > 0, Double(text) == 0 {
            text = String(format: "%.2e", magnitude)
        }
        let signed = value < 0 ? "-\(text)" : text
        return "\(signed) USDT"
    }

    /// Per-contract European cash identity. Sign from buy/sell.
    static func pnlPerContract(
        s: Double,
        strike: Double,
        right: Right,
        side: Side,
        premium: Double
    ) -> Double {
        let intrinsic: Double
        switch right {
        case .call: intrinsic = max(s - strike, 0)
        case .put: intrinsic = max(strike - s, 0)
        }
        switch side {
        case .buy: return intrinsic - premium
        case .sell: return premium - intrinsic
        }
    }

    static func polyline(
        strike: Double,
        right: Right,
        side: Side,
        premium: Double,
        spotS: Double
    ) -> (points: [Point], openWing: Bool) {
        let span = max(abs(spotS - strike), max(strike, 1) * 0.25, 1)
        let lo = max(0, min(spotS, strike) - span)
        let hi = max(spotS, strike) + span
        var xs: [Double] = [lo, strike, spotS, hi]
        if right == .call {
            xs.append(strike * 0.5)
            xs.append(strike + span * 2)
        } else {
            xs.append(0)
            xs.append(strike * 1.5)
        }
        let unique = Array(Set(xs.map { ($0 * 1e9).rounded() / 1e9 })).sorted()
        let points = unique.map { x in
            Point(s: x, pnl: pnlPerContract(s: x, strike: strike, right: right, side: side, premium: premium))
        }
        let openWing = side == .sell && right == .call
        return (points, openWing)
    }

    struct Inputs: Equatable {
        var datedContractBound: Bool
        var strikeText: String
        var right: Right
        var side: Side
        var premiumText: String
        var contractCount: Int
        /// Lock-named `indexPrice` string. Nil / empty = missing. Never lastPrice.
        var indexPriceText: String?
        var expiryDateMs: Int64?
        var nowMs: Int64
    }

    static func evaluate(_ input: Inputs) -> Kind {
        let indexRaw = input.indexPriceText?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        let s = Double(indexRaw)
        let strike = Double(input.strikeText.trimmingCharacters(in: .whitespacesAndNewlines))
        let premium = parsePremium(input.premiumText)
        let ready = input.datedContractBound
            && input.contractCount > 0
            && strike != nil && (strike ?? 0).isFinite
            && premium != nil
            && s != nil && (s ?? .nan).isFinite && !(indexRaw.isEmpty)
        let dte: Int
        let dteKnown: Bool
        if let ms = input.expiryDateMs {
            dte = calendarDTE(expiryDateMs: ms, nowMs: input.nowMs)
            dteKnown = true
        } else {
            dte = 0
            dteKnown = false
        }
        guard ready, let strike, let premium, let s else {
            if indexRaw.isEmpty || s == nil {
                return .hole(reason: holeIndexMissing)
            }
            return .hole(reason: holeInputsMissing)
        }
        let (points, openWing) = polyline(
            strike: strike, right: input.right, side: input.side, premium: premium, spotS: s
        )
        let longMaxLoss: Double? = input.side == .buy ? premium : nil
        return .lit(Lit(
            points: points,
            spotS: s,
            openWing: openWing,
            longMaxLoss: longMaxLoss,
            caption: litCaption,
            dte: dte,
            dteKnown: dteKnown
        ))
    }

    static let holeTitle = "payoff unavailable"
    static let holeIndexMissing =
        "S from GET /eapi/v1/index · unavailable — not spot last, not markPrice"
    static let holeInputsMissing = "needs strike, right, side, premium, contracts, and index S"
    static let litCaption =
        "settlement identity · S from GET /eapi/v1/index · not ModelComputed · per contract · unit is not applied"
}
