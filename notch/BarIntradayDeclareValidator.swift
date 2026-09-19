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

/// Inputs for the intraday/scalper pre-trade confirm gate (#121).
struct BarIntradayDeclarationSubmitInput: Equatable, Sendable {
    var blocksDeclarationSubmit: Bool
    var protectiveSlConsent: Bool
    var calm: Int
    var confidence: Int
    var stopLossText: String
    var symbolRaw: String
    var quantityText: String
    var setupType: String
    var invalidationTypeRaw: String
    var invalidationCondition: String
    var declarationKindWire: String
    var scalperSessionId: String
    /// Options lots — when set, satisfies the quantity confirm gate (lots is the unit until lot size exists).
    var lotsText: String = ""
    var isOptions: Bool = false
    var isUsdm: Bool = false
    var optionLegCount: Int = 0
    var maxPlannedLossText: String = ""
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

    /// Mirrors `BarDeclarationFlowView.buildJsonBody()` guards — drives disabled confirm + inline hint.
    static func submitReadiness(_ input: BarIntradayDeclarationSubmitInput) -> (ready: Bool, hint: String?) {
        if input.blocksDeclarationSubmit {
            return (false, "Circuit active — finish or clear the web Bar intervention before declaring.")
        }
        if !input.isOptions, !input.isUsdm, !input.protectiveSlConsent {
            return (false, "Turn on auto-place stop loss in Step 4.")
        }
        let calmOpt: Int? = (1 ... 5).contains(input.calm) ? input.calm : nil
        let confOpt: Int? = (1 ... 5).contains(input.confidence) ? input.confidence : nil
        guard canProceedFromEmotionalCheckIn(calm: calmOpt, confidence: confOpt) else {
            return (false, "Choose psychological calm and confidence in Step 1.")
        }
        guard stickyPreTradeConfirmEnabled(calm: calmOpt, confidence: confOpt, stopLossText: input.stopLossText) else {
            return (false, "Enter a stop loss price in Step 2.")
        }
        let symTrim = input.symbolRaw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard BarBrokerTicker.normalize(raw: input.symbolRaw) != nil else {
            if symTrim.isEmpty {
                return (false, "Enter a symbol in Step 2 (e.g. RELIANCE).")
            }
            return (false, "Symbol must be a broker ticker (e.g. RELIANCE), not a company name.")
        }
        if input.isOptions {
            if input.optionLegCount < 1 {
                return (false, "Add a leg.")
            }
            guard let max = Double(input.maxPlannedLossText.trimmingCharacters(in: .whitespacesAndNewlines)),
                  max > 0
            else {
                return (false, "Enter max planned loss.")
            }
            guard !input.invalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                return (false, "Write what would prove this trade wrong.")
            }
            return (true, nil)
        }
        if !lotsSatisfyQuantity(input) {
            guard let qty = Double(input.quantityText.trimmingCharacters(in: .whitespacesAndNewlines)), qty > 0 else {
                return (false, input.isOptions ? "Enter lots in Step 2." : "Enter quantity in Step 2.")
            }
        }
        guard !input.setupType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return (false, "Pick a setup type in Step 3.")
        }
        let invKind = input.invalidationTypeRaw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard BarInvalidationKind(rawValue: invKind) != nil else {
            return (false, "Pick an invalidation type in Step 3.")
        }
        guard !input.invalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return (false, "Describe your invalidation in Step 3.")
        }
        if input.declarationKindWire == "scalper_session",
           input.scalperSessionId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return (false, "Enter scalper session id.")
        }
        return (true, nil)
    }

    /// Options lots populate quantity until lot size exists. Spot/equity ignore lots.
    static func lotsSatisfyQuantity(_ input: BarIntradayDeclarationSubmitInput) -> Bool {
        guard input.isOptions else { return false }
        guard let lots = Int(input.lotsText.trimmingCharacters(in: .whitespacesAndNewlines)), lots > 0 else {
            return false
        }
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
