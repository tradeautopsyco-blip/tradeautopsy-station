import SwiftUI

struct BarPlanSlSuggestorRow: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    var stopLossText: String
    var targetPriceText: String
    var instrumentRole: String = "cash"
    var onApplyQuantity: ((String) -> Void)? = nil

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
        let authored = viewModel.planRiskPreviewPresentation?.proposedSizeText ?? "—"
        return BarPlanSlSuggestor.present(
            BarPlanSlSuggestor.Input(
                riskPercentOfMargin: pct,
                marginLit: viewModel.shippingFundsGlance.isLit,
                marginDisplay: margin.value,
                entry: entry,
                sideBuy: sideBuy,
                bookId: viewModel.declareBookId ?? "",
                authoredSizeText: authored
            )
        )
    }

    private var sizeDashed: Bool {
        let t = presentation.suggestedSizeText.trimmingCharacters(in: .whitespacesAndNewlines)
        return t.isEmpty || t == "—"
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
            HStack {
                Text("Proposed size")
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                Text(presentation.suggestedSizeText)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(sizeDashed ? BarDS.Text.muted : BarDS.Text.primary)
                    .strikethrough(sizeDashed, color: BarDS.Text.muted)
            }
            if !sizeDashed {
                Button("Apply size") {
                    let size = presentation.suggestedSizeText
                    if let onApplyQuantity {
                        onApplyQuantity(size)
                    } else {
                        quantityText = size
                    }
                }
                .buttonStyle(.plain)
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Accent.teal)
            }
            Text(presentation.footnote)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
        .onAppear { schedulePreview() }
        .onChange(of: marginRiskPct) { _, _ in schedulePreview() }
        .onChange(of: stopLossText) { _, _ in schedulePreview() }
        .onChange(of: quantityText) { _, _ in schedulePreview() }
        .onChange(of: targetPriceText) { _, _ in schedulePreview() }
        .onChange(of: viewModel.declEntryPrice) { _, _ in schedulePreview() }
        .onChange(of: sideBuy) { _, _ in schedulePreview() }
    }

    private func schedulePreview() {
        let pct = Double(marginRiskPct.trimmingCharacters(in: .whitespacesAndNewlines))
        let req = viewModel.buildPlanRiskPreviewRequest(
            sideBuy: sideBuy,
            budgetMode: .riskPercent,
            budgetValue: pct,
            stopLossText: stopLossText,
            quantityText: quantityText,
            targetPriceText: targetPriceText,
            instrumentRole: instrumentRole
        )
        viewModel.schedulePlanRiskPreview(request: req)
    }
}
