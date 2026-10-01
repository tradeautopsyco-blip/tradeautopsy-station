import SwiftUI

/// Tradeture-style preview strip — agent envelope when available; local ladder otherwise.
struct BarPlanRiskPreviewRow: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    var stopLossText: String
    @Binding var quantityText: String
    var targetPriceText: String
    var instrumentRole: String = "cash"
    var onApplyQuantity: ((String) -> Void)? = nil

    @State private var budgetMode: BarPlanRiskPreview.BudgetMode = .riskPercent
    @State private var budgetValueText: String = ""

    private var presentation: BarPlanRiskPreview.Presentation {
        let req = buildRequest()
        if let agent = viewModel.planRiskPreviewPresentation {
            return agent
        }
        let quote = viewModel.deskQuoteCurrency
            ?? DeskMoneyFormatting.quoteCurrency(forBrokerSlug: viewModel.activeExecutionBrokerSlug)
            ?? ""
        return BarPlanRiskPreview.localPresentation(req, quoteCurrency: quote)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Risk preview")
                .font(BarDS.bodyFont(11, weight: .semibold))
                .foregroundColor(BarDS.Text.section)
                .textCase(.uppercase)

            Picker("Budget", selection: $budgetMode) {
                Text("Risk %").tag(BarPlanRiskPreview.BudgetMode.riskPercent)
                Text("Fixed").tag(BarPlanRiskPreview.BudgetMode.fixedMoney)
            }
            .pickerStyle(.segmented)
            .onChange(of: budgetMode) { _, _ in schedulePreview() }
            .onChange(of: budgetValueText) { _, _ in schedulePreview() }

            HStack {
                Text(budgetMode == .riskPercent ? "Risk %" : "Fixed amount")
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer()
                BarInputField(
                    placeholder: budgetMode == .riskPercent ? "No default" : "Desk floor",
                    text: $budgetValueText
                )
                .frame(maxWidth: 120)
            }

            metricRow("Balance", presentation.balanceText)
            metricRow("Risk at stop", presentation.riskText)
            metricRow("Fees", presentation.feesText)
            metricRow("Reward", presentation.rewardText)
            metricRow("R:R excl fees", presentation.rrExFeesText)
            metricRow("R:R incl fees", presentation.rrInFeesText)
            metricRow("Proposed size", presentation.proposedSizeText, dashed: presentation.proposedSizeDashed)
            if !presentation.proposedSizeDashed {
                Button("Apply size") {
                    let size = presentation.proposedSizeText
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
        .onAppear {
            if budgetValueText.isEmpty, budgetMode == .fixedMoney, let floor = DeskRulesStore.shared.dailyFloor {
                budgetValueText = String(format: "%.0f", floor)
            }
            schedulePreview()
        }
        .onChange(of: stopLossText) { _, _ in schedulePreview() }
        .onChange(of: quantityText) { _, _ in schedulePreview() }
        .onChange(of: targetPriceText) { _, _ in schedulePreview() }
        .onChange(of: viewModel.declEntryPrice) { _, _ in schedulePreview() }
        .onChange(of: sideBuy) { _, _ in schedulePreview() }
    }

    private func metricRow(_ label: String, _ value: String, dashed: Bool = false) -> some View {
        HStack {
            Text(label)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.hint)
            Spacer(minLength: 8)
            Text(value)
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(dashed ? BarDS.Text.muted : BarDS.Text.primary)
                .strikethrough(dashed, color: BarDS.Text.muted)
        }
    }

    private func buildRequest() -> BarPlanRiskPreview.Request {
        let entry = Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        let stop = Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines))
        let target = Double(targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines))
        let qty = Double(quantityText.trimmingCharacters(in: .whitespacesAndNewlines))
        let budget = Double(budgetValueText.trimmingCharacters(in: .whitespacesAndNewlines))
        let glance = viewModel.shippingFundsGlance
        let tick = Double(viewModel.deskTickSize ?? "")
        let step = Double(viewModel.deskStepSize ?? "")
        let levText = viewModel.futuresMarginReadout(symbol: viewModel.barDeclarationSymbol).leverage
        let lev = Double(levText ?? "")
        return BarPlanRiskPreview.Request(
            bookId: viewModel.declareBookId ?? "",
            sideBuy: sideBuy,
            budgetMode: budgetMode,
            budgetValue: budget,
            entry: entry,
            stop: stop,
            target: target,
            overrideQty: qty,
            fundsLit: glance.isLit,
            fundsBalance: BarPlanRiskPreview.freeAmount(from: glance.freeText),
            fundsDisplayText: glance.freeText,
            priceIncrement: tick.flatMap { $0 > 0 ? $0 : nil },
            multiplier: nil,
            unitBatchSize: step.flatMap { $0 > 0 ? $0 : nil },
            instrumentRole: instrumentRole,
            leverage: (instrumentRole == "nfo_future" || instrumentRole == "cash")
                ? lev.flatMap { $0 > 0 ? $0 : nil }
                : nil,
            symbol: viewModel.barDeclarationSymbol
        )
    }

    private func schedulePreview() {
        viewModel.schedulePlanRiskPreview(request: buildRequest())
    }
}
