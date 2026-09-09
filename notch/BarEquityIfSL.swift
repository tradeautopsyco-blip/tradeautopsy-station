import SwiftUI

/// Mode B — after Declare, the big box is account path to SL. Not broker OHLCV.
public struct EquityIfSLSnapshot: Equatable, Sendable {
    public var maxPlannedLoss: Double?
    public var declared: Bool
    /// Polyline is illustration only — labeled scenario, never live remaining-risk.
    public var scenarioCaption: String

    public init(maxPlannedLoss: Double?, declared: Bool) {
        self.maxPlannedLoss = maxPlannedLoss
        self.declared = declared
        if declared, let loss = maxPlannedLoss {
            scenarioCaption = String(
                format: "Scenario · equity path to SL · max planned loss %.0f. Not remaining-risk. Not broker OHLCV.",
                loss
            )
        } else if declared {
            scenarioCaption = "Declared — max planned loss empty until units, entry, and stop are typed."
        } else {
            scenarioCaption = "Pre-trade stays glance + form. Declare to flip this box to equity-if-SL."
        }
    }
}

public enum EquityIfSL {
    public static func snapshot(
        units: Double?,
        entry: Double?,
        stop: Double?,
        sideBuy: Bool,
        declared: Bool
    ) -> EquityIfSLSnapshot {
        let loss = BarPlanLadder.maxPlannedLossINR(
            units: units,
            entry: entry,
            stop: stop,
            sideBuy: sideBuy
        )
        return EquityIfSLSnapshot(maxPlannedLoss: loss, declared: declared)
    }

    /// Illustration polyline (account declining toward SL). Not a live series.
    static func scenarioPoints(maxPlannedLoss: Double) -> [BarTAPoint] {
        let loss = abs(maxPlannedLoss)
        return [
            BarTAPoint(x: "now", y: 0),
            BarTAPoint(x: "½", y: -loss * 0.5),
            BarTAPoint(x: "SL", y: -loss),
        ]
    }
}

struct BarEquityIfSLBox: View {
    let snapshot: EquityIfSLSnapshot

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(snapshot.declared ? "Account · equity if SL" : "Glance")
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Text.hint)
            Text(snapshot.scenarioCaption)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            if snapshot.declared, let loss = snapshot.maxPlannedLoss {
                BarTALinePlot(
                    points: EquityIfSL.scenarioPoints(maxPlannedLoss: loss),
                    kind: .loss,
                    height: 72
                )
                .accessibilityLabel("Scenario equity path to stop, not live remaining risk")
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: 1)
        )
    }
}
