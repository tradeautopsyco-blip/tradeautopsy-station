import SwiftUI

/// Fill-gate verdict for *this* trade. Does not invent a stop. Does not FX-blend.
public enum DetectCardKind: Equatable, Sendable {
    /// Declared SL is tighter than the live SL (live is worse).
    case liveWorse
    /// Live SL is tighter than the plan (plan was looser).
    case planTighter
    /// Plan and live loss match.
    case planHolds
    /// No declared stop — Plan in Notch / Not now. Not a nag.
    case noForm
}

public struct DetectCardInput: Equatable, Sendable {
    public var qty: Double
    public var entry: Double?
    public var planStop: Double?
    public var liveStop: Double?
    public var sideBuy: Bool
    public var accountEquity: Double?
    public var tradeCurrency: String
    public var accountCurrency: String

    public init(
        qty: Double,
        entry: Double?,
        planStop: Double?,
        liveStop: Double?,
        sideBuy: Bool,
        accountEquity: Double?,
        tradeCurrency: String,
        accountCurrency: String
    ) {
        self.qty = qty
        self.entry = entry
        self.planStop = planStop
        self.liveStop = liveStop
        self.sideBuy = sideBuy
        self.accountEquity = accountEquity
        self.tradeCurrency = tradeCurrency
        self.accountCurrency = accountCurrency
    }
}

public struct DetectCardResult: Equatable, Sendable {
    public var kind: DetectCardKind
    public var planLoss: Double?
    public var liveLoss: Double?
    public var accountPercent: Double?
    /// True when stop is ≥ 13% of account — warning, not a normal reading.
    public var sizeWarning: Bool
    /// DualNoBlend: mixed currencies never produce a blended %.
    public var dualNoBlendBlockedPercent: Bool

    public static let sizeWarningThreshold = 0.13

    public var headline: String {
        switch kind {
        case .noForm:
            return "No stop on this fill"
        case .liveWorse:
            return "Live SL is worse than the plan"
        case .planTighter:
            return "Live SL is tighter than the plan"
        case .planHolds:
            return "Live SL matches the plan"
        }
    }

    public var body: String {
        switch kind {
        case .noForm:
            return "Plan in Notch / Not now. This card does not invent a stop."
        case .liveWorse, .planTighter, .planHolds:
            return "This trade only — plan vs live SL. Not remaining-risk. Not a blended desk."
        }
    }
}

public enum DetectCard {
    /// Plan vs live SL from declared entry/stop × qty. Missing form → `.noForm`.
    public static func evaluate(_ input: DetectCardInput) -> DetectCardResult {
        let planLoss = BarPlanLadder.maxPlannedLossINR(
            units: input.qty,
            entry: input.entry,
            stop: input.planStop,
            sideBuy: input.sideBuy
        )
        let liveLoss = BarPlanLadder.maxPlannedLossINR(
            units: input.qty,
            entry: input.entry,
            stop: input.liveStop,
            sideBuy: input.sideBuy
        )

        let kind: DetectCardKind
        if input.planStop == nil {
            kind = .noForm
        } else if let planLoss, let liveLoss {
            if liveLoss > planLoss {
                kind = .liveWorse
            } else if liveLoss < planLoss {
                kind = .planTighter
            } else {
                kind = .planHolds
            }
        } else if input.planStop != nil {
            kind = .planHolds
        } else {
            kind = .noForm
        }

        let currenciesMatch =
            input.tradeCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
            == input.accountCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let dualBlocked = !currenciesMatch
        let worst = liveLoss ?? planLoss
        var percent: Double?
        if !dualBlocked, let worst, let equity = input.accountEquity, equity > 0 {
            percent = worst / equity
        }

        let warn = (percent ?? 0) >= DetectCardResult.sizeWarningThreshold
        return DetectCardResult(
            kind: kind,
            planLoss: planLoss,
            liveLoss: liveLoss,
            accountPercent: percent,
            sizeWarning: warn,
            dualNoBlendBlockedPercent: dualBlocked
        )
    }
}

/// Shared overlay. Layout chrome is the shell's job; this is the card itself.
public struct DetectCardView: View {
    public var result: DetectCardResult
    public var onPlanInNotch: (() -> Void)?
    public var onNotNow: (() -> Void)?

    public init(
        result: DetectCardResult,
        onPlanInNotch: (() -> Void)? = nil,
        onNotNow: (() -> Void)? = nil
    ) {
        self.result = result
        self.onPlanInNotch = onPlanInNotch
        self.onNotNow = onNotNow
    }

    public var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(result.headline)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(result.sizeWarning ? BarDS.Accent.amber : BarDS.Text.primary)
            Text(result.body)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if result.sizeWarning {
                Text("Stop is ≥ 13% of account — warning, not a normal reading.")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
            }
            if result.dualNoBlendBlockedPercent {
                Text("% of account stays — until trade and account share a currency.")
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
            if result.kind == .noForm {
                HStack(spacing: 8) {
                    Button("Plan in Notch") { onPlanInNotch?() }
                        .buttonStyle(.plain)
                        .font(BarDS.bodyFont(11, weight: .semibold))
                        .foregroundColor(BarDS.Accent.teal)
                    Button("Not now") { onNotNow?() }
                        .buttonStyle(.plain)
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundColor(BarDS.Text.muted)
                }
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(result.sizeWarning ? BarDS.Accent.amber.opacity(0.35) : BarDS.Border.card, lineWidth: 1)
        )
    }
}
