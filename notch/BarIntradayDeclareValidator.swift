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
    var frustration: Int = 0
    var excitement: Int = 0
    var stanceRaw: String = ""
    var intent: String = ""
    var targetPriceText: String = ""
    var invalidationPriceText: String = ""
    var requiresCashProduct: Bool = false
    var cashProduct: String = ""
    var gate: BarPlanGateStripState = BarPlanGateStripState()
}

/// Pure validation helpers for the intraday Bar declaration flow (#116). Observable UI wires selections into these functions.
enum BarIntradayDeclareValidator {
    /// Four emotion sliders must all be chosen — never pre-filled.
    static func canProceedFromEmotionalCheckIn(
        calm: Int?,
        confidence: Int?,
        frustration: Int? = nil,
        excitement: Int? = nil
    ) -> Bool {
        BarPlanGateStrip.emotionFilled(
            calm: calm ?? 0,
            confidence: confidence ?? 0,
            frustration: frustration ?? 0,
            excitement: excitement ?? 0
        )
    }

    /// Sticky Confirm still needs a positive stop, plus the four sliders.
    static func stickyPreTradeConfirmEnabled(calm: Int?, confidence: Int?, stopLossText: String) -> Bool {
        guard let calm, let confidence else { return false }
        guard (1 ... 5).contains(calm), (1 ... 5).contains(confidence) else { return false }
        guard let sl = Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)), sl > 0 else { return false }
        return true
    }

    static func invalidationSatisfied(_ input: BarIntradayDeclarationSubmitInput) -> Bool {
        let invKind = input.invalidationTypeRaw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if input.isOptions {
            return !input.invalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        }
        guard let kind = BarInvalidationKind(rawValue: invKind) else { return false }
        if kind == .price {
            guard let p = Double(input.invalidationPriceText.trimmingCharacters(in: .whitespacesAndNewlines)), p > 0 else {
                return false
            }
            return true
        }
        return !input.invalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    static func targetSatisfied(_ input: BarIntradayDeclarationSubmitInput) -> Bool {
        guard let t = Double(input.targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines)), t > 0 else {
            return false
        }
        return true
    }

    /// Mirrors `BarDeclarationFlowView.buildJsonBody()` guards — drives disabled confirm + inline hint.
    static func submitReadiness(_ input: BarIntradayDeclarationSubmitInput) -> (ready: Bool, hint: String?) {
        if input.blocksDeclarationSubmit {
            return (false, "Circuit active — finish or clear the Harness intervention before declaring.")
        }
        if !input.isOptions, !input.isUsdm, !input.protectiveSlConsent {
            return (false, "Turn on auto-place stop loss in Step 4.")
        }
        let calmOpt: Int? = (1 ... 5).contains(input.calm) ? input.calm : nil
        let confOpt: Int? = (1 ... 5).contains(input.confidence) ? input.confidence : nil
        let frOpt: Int? = (1 ... 5).contains(input.frustration) ? input.frustration : nil
        let exOpt: Int? = (1 ... 5).contains(input.excitement) ? input.excitement : nil
        guard canProceedFromEmotionalCheckIn(
            calm: calmOpt,
            confidence: confOpt,
            frustration: frOpt,
            excitement: exOpt
        ) else {
            return (false, "Set calm, confidence, frustration, and excitement.")
        }
        guard stickyPreTradeConfirmEnabled(calm: calmOpt, confidence: confOpt, stopLossText: input.stopLossText) else {
            return (false, "Enter a stop loss price in Step 2.")
        }
        let stance = input.stanceRaw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        guard BarPlanStance(rawValue: stance) != nil else {
            return (false, "Pick planned or reactive.")
        }
        guard !input.intent.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
            return (false, "Write why this, in one sentence.")
        }
        let symTrim = input.symbolRaw.trimmingCharacters(in: .whitespacesAndNewlines)
        guard BarBrokerTicker.normalize(raw: input.symbolRaw) != nil else {
            if symTrim.isEmpty {
                return (false, "Enter a symbol in Step 2 (e.g. RELIANCE).")
            }
            return (false, "Symbol must be a broker ticker (e.g. RELIANCE), not a company name.")
        }
        if input.requiresCashProduct {
            let p = input.cashProduct.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            guard p == "CNC" || p == "MIS" else {
                return (false, "Pick CNC or MIS.")
            }
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
        } else if !lotsSatisfyQuantity(input) {
            guard let qty = Double(input.quantityText.trimmingCharacters(in: .whitespacesAndNewlines)), qty > 0 else {
                return (false, "Enter quantity in Step 2.")
            }
        }
        if !input.isOptions {
            guard !input.setupType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else {
                return (false, "Pick a setup type in Step 3.")
            }
        }
        if !invalidationSatisfied(input) {
            let invKind = input.invalidationTypeRaw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if BarInvalidationKind(rawValue: invKind) == .price {
                return (false, "Enter an invalidation price.")
            }
            if BarInvalidationKind(rawValue: invKind) == nil, !input.isOptions {
                return (false, "Pick an invalidation type in Step 3.")
            }
            return (false, "Describe your invalidation.")
        }
        if !targetSatisfied(input) {
            return (false, "Enter a target price.")
        }
        let emotionOk = BarPlanGateStrip.emotionFilled(
            calm: input.calm,
            confidence: input.confidence,
            frustration: input.frustration,
            excitement: input.excitement
        )
        let exitOk = invalidationSatisfied(input) && targetSatisfied(input)
        if let gateHint = BarPlanGateStrip.emptyHint(
            state: input.gate,
            emotionFilled: emotionOk,
            exitFilled: exitOk
        ) {
            return (false, gateHint)
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
