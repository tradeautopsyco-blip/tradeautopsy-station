import SwiftUI

/// Working live desk — Plan-family glance + session chart + conditions (read-only levels).
struct BarWorkingLiveCockpit: View {
    @ObservedObject var viewModel: NotchViewModel
    var liveState: BarLiveStateResponse?
    /// Selected row from multi-trade list; falls back to live-state mirror pending.
    var selectedPending: BarPendingDeclaration?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("LIVE DESK")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(Color.white.opacity(0.38))
                .tracking(0.6)
            glanceStrip
            conditionsStrip
            sessionChart
        }
        .onAppear {
            let sym = workingSymbol
            guard !sym.isEmpty else { return }
            viewModel.refreshDeskExtracts(symbol: sym, instrumentId: viewModel.deskSelectedInstrumentId)
        }
        .onChange(of: workingSymbol) { _, sym in
            guard !sym.isEmpty else { return }
            viewModel.refreshDeskExtracts(symbol: sym, instrumentId: viewModel.deskSelectedInstrumentId)
        }
    }

    private var effectivePending: BarPendingDeclaration? {
        selectedPending ?? liveState?.pendingDeclaration
    }

    private var workingSymbol: String {
        let pending = effectivePending?.symbol.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !pending.isEmpty { return pending }
        return viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private var glanceStrip: some View {
        HStack(spacing: 6) {
            glanceCell(title: "LAST", value: lastGlanceValue, subtitle: lastGlanceSubtitle)
            glanceCell(title: "UNREALISED P&L", value: unrealGlanceValue, subtitle: unrealGlanceSubtitle, valueColor: unrealColor)
            glanceCell(title: "MARGIN", value: margin.value, subtitle: margin.subtitle)
        }
    }

    private var conditionsStrip: some View {
        ScrollView(.horizontal, showsIndicators: false) {
            HStack(spacing: 6) {
                ForEach(Array(conditions.enumerated()), id: \.offset) { _, cell in
                    glanceCell(title: cell.title, value: cell.value, subtitle: cell.subtitle)
                        .frame(minWidth: 88)
                }
            }
        }
    }

    @ViewBuilder
    private var sessionChart: some View {
        if viewModel.deskHistoryStatus == "success", !viewModel.deskHistoryCandles.isEmpty {
            BarOptionsSessionChart(
                candles: viewModel.deskHistoryCandles,
                drag: nil,
                fixedPlanLevels: planLevels,
                last: viewModel.sessionChartLast,
                symbol: workingSymbol,
                interval: viewModel.deskHistoryInterval,
            )
            .frame(maxWidth: .infinity, minHeight: 168, maxHeight: 220)
            .background(BarDS.Fill.elevated)
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
            )
            .accessibilityLabel("Working session chart with plan levels")
        } else {
            sessionHole
        }
    }

    private var sessionHole: some View {
        VStack(spacing: 6) {
            Text("no session series")
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text(
                BarDeskTemplate.historyGlanceLine(
                    licensedStatus: viewModel.deskHistoryStatus,
                    licensedIneligible: viewModel.deskHistoryIneligible,
                    yahooStatus: viewModel.deskYahooHistoryStatus,
                    yahooIneligible: viewModel.deskYahooHistoryIneligible,
                    stitchYahoo: false,
                    productUse: viewModel.deskHistoryProductUse,
                    bookId: viewModel.deskHistoryBookId,
                ),
            )
            .font(BarDS.monoFont(10, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .multilineTextAlignment(.center)
            .fixedSize(horizontal: false, vertical: true)
            if planLevels != nil {
                Text("Entry / stop / target show when session history loads.")
                    .font(BarDS.monoFont(9, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .multilineTextAlignment(.center)
            }
        }
        .frame(maxWidth: .infinity, minHeight: 168)
        .padding(.horizontal, 12)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func glanceCell(
        title: String,
        value: String,
        subtitle: String,
        valueColor: Color = BarDS.Text.primary,
    ) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title)
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.6)
            Text(value)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(valueColor)
                .lineLimit(2)
                .minimumScaleFactor(0.85)
            Text(subtitle)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .lineLimit(2)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 7)
        .padding(.horizontal, 9)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var planLevels: SessionChartPlanLevels? {
        BarWorkingLivePresentation.planLevels(pending: effectivePending)
    }

    private var margin: (value: String, subtitle: String) {
        BarWorkingLivePresentation.marginDisplay(
            brokerSyncClass: viewModel.brokerSyncClass,
            barSyncState: liveState?.syncState,
            fundsStatus: viewModel.accountChrome.fundsStatus,
            freeText: viewModel.accountChrome.freeText,
        )
    }

    private var conditions: [BarWorkingLivePresentation.ConditionCell] {
        BarWorkingLivePresentation.conditionCells(
            symbol: workingSymbol,
            last: viewModel.deskQuoteLast,
            lastStatus: viewModel.deskLastStatus,
            historyStatus: viewModel.deskHistoryStatus,
            planState: liveState?.planState,
            barSyncState: liveState?.syncState,
            brokerSyncClass: viewModel.brokerSyncClass,
        )
    }

    private var hasOpenFill: Bool {
        if let avg = effectivePending?.avgFill, avg > 0 { return true }
        return viewModel.hasOpenPositions
    }

    private var unrealGlanceSubtitle: String {
        BarWorkingLivePresentation.unrealizedSubtitle(
            pending: effectivePending,
            hasOpenFill: hasOpenFill,
        )
    }

    private var unrealGlanceValue: String {
        let unreal = liveState?.unrealizedPnL
        let worst = liveState?.composite?.worstCase
        if let unreal {
            return viewModel.formatDeskMoney(unreal)
        }
        if hasOpenFill, let worst {
            return viewModel.formatDeskMoney(worst)
        }
        if effectivePending != nil {
            return "—"
        }
        if let worst {
            return viewModel.formatDeskMoney(worst)
        }
        return "—"
    }

    private var unrealColor: Color {
        guard hasOpenFill || liveState?.unrealizedPnL != nil else {
            return BarDS.Text.secondary
        }
        let raw = liveState?.unrealizedPnL ?? liveState?.composite?.worstCase ?? 0
        if raw < 0 { return BarDS.Accent.red }
        if raw > 0 { return BarDS.Accent.green }
        return BarDS.Text.primary
    }

    private var lastGlanceValue: String {
        if let freshness = BarDeskLastFormatting.freshnessBesideLast(status: viewModel.deskLastStatus),
           let last = viewModel.deskQuoteLast
        {
            return "\(BarWorkingCompare.formatPrice(last)) · \(freshness)"
        }
        if let honesty = HonestyStatus.fromWire(viewModel.deskLastStatus) {
            return honesty.rawValue.replacingOccurrences(of: "_", with: " ")
        }
        return BarWorkingCompare.labeledLast(last: viewModel.deskQuoteLast, status: viewModel.deskLastStatus)
    }

    private var lastGlanceSubtitle: String {
        let book = (viewModel.declareBookId ?? viewModel.accountChrome.bookId)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if book.isEmpty { return "market/quote" }
        return "market/quote · \(book)"
    }
}
