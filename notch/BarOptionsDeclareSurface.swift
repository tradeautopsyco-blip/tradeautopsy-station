import Foundation

/// Pre-trade Options uses the three-zone HTML surface. Spot / equity keep the existing form.
enum BarOptionsDeclareSurface {
    static func usesThreeZone(for asset: BarDeclareAssetClass) -> Bool {
        asset == .options
    }
}

/// Chain table is forbidden while the extract is dark. Ghost strike grids are cheating.
/// Wire `"success"` is a glance status, not `HonestyStatus`.
enum BarOptionsChainPresentation: Equatable {
    /// No underlying typed yet.
    case nothingDeclared
    /// Capability hole or refused snapshot — no rows.
    case unavailable
    /// Result set legitimately zero.
    case empty
    /// Glance `status: success` — master rows may exist; still no invented strike grid.
    case lit

    static func from(underlying: String, chainStatus: String) -> BarOptionsChainPresentation {
        let und = underlying.trimmingCharacters(in: .whitespacesAndNewlines)
        if und.isEmpty { return .nothingDeclared }
        let wire = chainStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch wire {
        case "success": return .lit
        case "empty": return .empty
        default: return .unavailable
        }
    }

    /// Strike scale / expiry conversion stay unspecified — never paint 57500.
    var showsStrikeGrid: Bool { false }
}

enum BarOptionsPremortem {
    static func question(hasLegs: Bool, declaredMaxINR: Double?) -> String {
        if hasLegs, let max = declaredMaxINR {
            return "This trade hit your declared limit of ₹\(Self.inr(max)). What happened?"
        }
        return "Write what would prove this trade wrong."
    }

    static func hint(hasLegs: Bool, declaredMaxINR: Double?) -> String {
        if hasLegs, declaredMaxINR != nil {
            return "σ rungs are dark, so this is seeded from your declared limit instead of the chain."
        }
        return "Fill the contract and size, and this question gets sharper."
    }

    private static func inr(_ n: Double) -> String {
        String(format: "%.0f", n)
    }
}

struct BarOptionsLadderRung: Equatable {
    var label: String
    var sub: String
    var amountINR: Double?
    var honesty: HonestyStatus?
}

enum BarOptionsLadderModel {
    /// Rung 1 from typed stop/entry/lots, else −declared max (HTML fallback when chain is a hole).
    static func stopRung(
        units: Double?,
        entry: Double?,
        stop: Double?,
        sideBuy: Bool,
        declaredMaxINR: Double?,
    ) -> Double? {
        if let rung = BarPlanLadder.rung1(units: units, entry: entry, stop: stop, sideBuy: sideBuy) {
            return rung
        }
        if let declaredMaxINR {
            return -abs(declaredMaxINR)
        }
        return nil
    }

    static func rungs(
        hasLegs _: Bool,
        stopText: String,
        stopRung: Double?,
        declaredMaxINR: Double?,
        horizonDays: Int,
        expiryDTE: Int?,
    ) -> [BarOptionsLadderRung] {
        let hLabel = horizonCaption(days: horizonDays, dte: expiryDTE)
        let stopSub: String
        let stopTrim = stopText.trimmingCharacters(in: .whitespacesAndNewlines)
        if !stopTrim.isEmpty {
            stopSub = "option at \(stopTrim) · typed, no market data needed"
        } else {
            stopSub = "set a stop price to light this rung"
        }
        let stopHonesty: HonestyStatus? = stopRung == nil ? .empty : nil
        return [
            BarOptionsLadderRung(
                label: "At your stop",
                sub: stopSub,
                amountINR: stopRung,
                honesty: stopHonesty,
            ),
            BarOptionsLadderRung(
                label: "1σ adverse \(hLabel)",
                sub: "σ scaled from chain IV",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "2σ / gap \(hLabel)",
                sub: "σ scaled from chain IV",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "Terminal at expiry",
                sub: "payoff needs premiums from the chain",
                amountINR: nil,
                honesty: .unavailable,
            ),
            BarOptionsLadderRung(
                label: "Distance to breach your limit",
                sub: declaredMaxINR == nil ? "set a max planned loss" : "needs Greeks to solve for the move",
                amountINR: nil,
                honesty: declaredMaxINR == nil ? .empty : .unavailable,
            ),
        ]
    }

    static func horizonCaption(days: Int, dte: Int?) -> String {
        if days == 1 { return "by close today" }
        if let dte, days == dte { return "by expiry" }
        return "over \(days) sessions"
    }
}
