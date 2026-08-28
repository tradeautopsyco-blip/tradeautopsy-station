import SwiftUI

enum BarPlanLadderHonesty: Equatable, Sendable {
    /// Missing typed units, entry, or stop — no number (not market unavailable).
    case empty
    case declared
}

/// Rung 1 from typed numbers. No chain, no lot size, no S3.
enum BarPlanLadder {
    /// `units × (stop − entry) × sign` where sign is +1 BUY, −1 SELL.
    static func rung1(units: Double?, entry: Double?, stop: Double?, sideBuy: Bool) -> Double? {
        guard let units, units > 0, let entry, let stop else { return nil }
        let sign: Double = sideBuy ? 1 : -1
        return units * (stop - entry) * sign
    }

    /// `abs(rung1)` only when rung1 is a loss. Nil when not a loss — do not invent a number.
    static func maxPlannedLossINR(rung1: Double?) -> Double? {
        guard let rung1, rung1 < 0 else { return nil }
        return abs(rung1)
    }

    static func maxPlannedLossINR(
        units: Double?,
        entry: Double?,
        stop: Double?,
        sideBuy: Bool,
    ) -> Double? {
        maxPlannedLossINR(rung1: rung1(units: units, entry: entry, stop: stop, sideBuy: sideBuy))
    }

    static func honesty(rung1: Double?) -> BarPlanLadderHonesty {
        rung1 == nil ? .empty : .declared
    }

    static func exceedsDeclaredLimit(rung1: Double?, declaredLimitINR: Double?) -> Bool {
        guard let rung1, let declaredLimitINR else { return false }
        return abs(rung1) > declaredLimitINR
    }
}

/// Tiny options ladder: typed rung 1 + declared-limit anchor. σ is caption-only until chain exists.
struct BarPlanLadderView: View {
    let rung1: Double?
    let declaredMaxLossINR: Double?

    var body: some View {
        let over = BarPlanLadder.exceedsDeclaredLimit(rung1: rung1, declaredLimitINR: declaredMaxLossINR)
        let valueColor: Color = over ? BarDS.Accent.red : BarDS.Text.hint

        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("At your stop")
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                if let loss = declaredMaxLossINR {
                    Text(String(format: "MAX LOSS (plan): ₹%.0f", loss))
                        .font(BarDS.monoFont(10, weight: .semibold))
                        .foregroundColor(valueColor)
                } else {
                    HonestyChip(status: .empty)
                }
            }

            HStack(spacing: 6) {
                Text("1σ / 2σ — needs chain")
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                HonestyChip(status: .unavailable)
            }

            if let limit = declaredMaxLossINR {
                Text(String(format: "Declared limit ₹%.0f", limit))
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(over ? BarDS.Accent.red : BarDS.Text.labels)
            }
        }
        .padding(.vertical, 4)
        .padding(.bottom, 4)
    }
}
