import Foundation

/// Risk : reward band for intraday declare UI coloring (#116): green ≥ 2.0, red &lt; 1.5.
enum BarIntradayRiskRewardBand: Equatable, Sendable {
    /// R:R ≥ 2.0 (spec: show as green).
    case favorable
    /// R:R &lt; 1.5 (spec: show as red).
    case weak
    /// Between weak and favorable.
    case moderate
    /// Missing inputs or degenerate risk/reward geometry.
    case indeterminate
}

/// Pure validation helpers for the intraday Bar declaration flow (#116). Observable UI wires selections into these functions.
enum BarIntradayDeclareValidator {
    /// Step 1 (emotional check-in): Scale A (calm, 1 best) and Scale B (confidence, 5 best) must both be chosen — never pre-filled.
    static func canProceedFromEmotionalCheckIn(calm: Int?, confidence: Int?) -> Bool {
        guard let calm, let confidence else { return false }
        return (1 ... 5).contains(calm) && (1 ... 5).contains(confidence)
    }

    /// #121 sticky “Confirm — enter trade →” gate (unified reference mockup 4): both scales answered **and** a positive stop price.
    static func stickyPreTradeConfirmEnabled(calm: Int?, confidence: Int?, stopLossText: String) -> Bool {
        guard canProceedFromEmotionalCheckIn(calm: calm, confidence: confidence) else { return false }
        guard let sl = Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)), sl > 0 else { return false }
        return true
    }

    /// Reward ÷ risk using absolute plan distances. Returns `nil` if risk or reward is non-positive (invalid geometry).
    static func riskRewardRatio(entry: Double, stop: Double, target: Double, sideBuy: Bool) -> Double? {
        let risk: Double
        let reward: Double
        if sideBuy {
            risk = entry - stop
            reward = target - entry
        } else {
            risk = stop - entry
            reward = entry - target
        }
        guard risk > 0, reward > 0 else { return nil }
        return reward / risk
    }

    static func riskRewardBand(ratio: Double?) -> BarIntradayRiskRewardBand {
        guard let ratio else { return .indeterminate }
        if ratio >= 2.0 { return .favorable }
        if ratio < 1.5 { return .weak }
        return .moderate
    }
}
