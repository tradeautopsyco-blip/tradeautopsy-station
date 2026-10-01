import SwiftUI

/// Typed risk (desk floor) + dashed proposed size — display only, no formula (Wave 4.2).
struct BarPlanSizerPlanRow: View {
    @ObservedObject var viewModel: NotchViewModel
    var stopLossText: String

    private var presentation: BarPlanSizer.Presentation {
        let entry = Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        let stop = Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines))
        let inv = Double(viewModel.declInvalidationPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        let quote = viewModel.deskQuoteCurrency
            ?? DeskMoneyFormatting.quoteCurrency(forBrokerSlug: viewModel.activeExecutionBrokerSlug)
            ?? ""
        return BarPlanSizer.present(
            BarPlanSizer.Inputs(
                dailyFloor: DeskRulesStore.shared.dailyFloor,
                entry: entry,
                invalidationPrice: inv,
                stopLoss: stop,
                fundsLit: viewModel.shippingFundsGlance.isLit
            ),
            quoteCurrency: quote
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Planned risk (floor)")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                Text(presentation.plannedRiskText)
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
            }
            HStack {
                Text("Proposed size")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                Text(presentation.sizeText)
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(presentation.sizeEmphasisDashed ? BarDS.Text.muted : BarDS.Text.primary)
                    .strikethrough(presentation.sizeEmphasisDashed, color: BarDS.Text.muted)
            }
            Text(presentation.footnote)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}
