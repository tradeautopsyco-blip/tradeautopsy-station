import Foundation
import SwiftUI

/// Desk-tab readout of Station desk rules. Display only — does not POST loss-limits or fire Kill.
@MainActor
struct BarDeskRulesReadout: View {
    @ObservedObject private var store: DeskRulesStore

    init(store: DeskRulesStore) {
        self.store = store
    }

    init() {
        self.init(store: DeskRulesStore.shared)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            BarSectionLabel(text: "Desk rules")
            Text("From Station Settings. Display only — they do not fire Kill.")
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            readoutRow(label: "Daily floor", value: moneyText(store.dailyFloor))
            readoutRow(label: "Mean loss", value: moneyText(store.meanLoss))
            readoutRow(label: "Max round trips", value: store.maxRoundTrips.map(String.init) ?? "—")
        }
    }

    private func readoutRow(label: String, value: String) -> some View {
        HStack {
            Text(label)
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
            Spacer(minLength: 0)
            Text(value)
                .font(BarDS.monoFont(12, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
    }

    private func moneyText(_ value: Double?) -> String {
        guard let value else { return "—" }
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.maximumFractionDigits = 0
        return formatter.string(from: NSNumber(value: value)) ?? "—"
    }
}
