import SwiftUI

struct BarPlanSlSuggestorRow: View {
    @ObservedObject var viewModel: NotchViewModel
    var stopLossText: String
    @State private var marginRiskPct: String = ""

    private var presentation: BarPlanSlSuggestor.Presentation {
        let margin = BarWorkingLivePresentation.marginDisplay(
            brokerSyncClass: viewModel.brokerSyncClass,
            barSyncState: viewModel.barLiveState?.syncState,
            fundsStatus: viewModel.shippingFundsGlance.status,
            freeText: viewModel.shippingFundsGlance.freeText
        )
        let pct = Double(marginRiskPct.trimmingCharacters(in: .whitespacesAndNewlines))
        let entry = Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        return BarPlanSlSuggestor.present(
            BarPlanSlSuggestor.Input(
                riskPercentOfMargin: pct,
                marginLit: viewModel.shippingFundsGlance.isLit,
                marginDisplay: margin.value,
                entry: entry,
                sideBuy: true,
                bookId: viewModel.declareBookId ?? ""
            )
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("SL suggestor (% of margin)")
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Text.section)
                .textCase(.uppercase)
            HStack {
                BarInputField(placeholder: "Risk % of margin", text: $marginRiskPct)
                Text("→ stop \(presentation.suggestedStopText)")
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
            Text(presentation.footnote)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
