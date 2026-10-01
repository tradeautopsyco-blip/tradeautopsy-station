import SwiftUI

/// Prototype 3-cell glance (`strip n3`). Last / History / Depth.
struct BarCashGlanceStrip: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        HStack(spacing: 6) {
            ForEach(BarCashCockpitSeed.stripKinds(for: viewModel.declareAssetClass), id: \.self) { kind in
                cell(kind)
            }
        }
    }

    @ViewBuilder
    private func cell(_ kind: BarCashCockpitSeed.StripKind) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title(kind))
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.6)
            valueRow(kind)
            Text(subtitle(kind))
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

    private func title(_ kind: BarCashCockpitSeed.StripKind) -> String {
        switch kind {
        case .funds: return "FUNDS"
        case .last: return "LAST · YOUR ENTRY"
        case .history: return "HISTORY"
        case .depth: return "DEPTH"
        case .margin: return "MARGIN"
        }
    }

    @ViewBuilder
    private func valueRow(_ kind: BarCashCockpitSeed.StripKind) -> some View {
        switch kind {
        case .funds:
            fundsValue
        case .last:
            lastValue
        case .history:
            historyValue
        case .depth:
            depthValue
        case .margin:
            Text(marginValue)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        }
    }

    @ViewBuilder
    private var fundsValue: some View {
        let glance = viewModel.shippingFundsGlance
        if glance.isLit {
            Text(glance.freeText)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        } else if let honesty = HonestyStatus.fromWire(glance.status) {
            HonestyChip(status: honesty)
        } else {
            Text("—")
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    @ViewBuilder
    private var lastValue: some View {
        let entry = viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)
        if !entry.isEmpty {
            HStack(spacing: 6) {
                Text(entry)
                    .font(BarDS.monoFont(13, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                if let freshness = BarDeskLastFormatting.freshnessBesideLast(
                    status: viewModel.deskLastStatus
                ) {
                    Text(freshness)
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Accent.amber)
                }
            }
        } else if let honesty = HonestyStatus.fromWire(viewModel.deskLastStatus) {
            HonestyChip(status: honesty)
        } else {
            Text(viewModel.deskLastStatus)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    @ViewBuilder
    private var historyValue: some View {
        if viewModel.deskHistoryStatus == "success",
                  let close = viewModel.deskHistoryCandles.last?.close,
                  !close.isEmpty {
            Text(close)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        } else if let honesty = HonestyStatus.fromWire(viewModel.deskHistoryStatus) {
            HonestyChip(status: honesty)
        } else {
            Text(viewModel.deskHistoryStatus)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    @ViewBuilder
    private var depthValue: some View {
        if viewModel.deskDepthDisplay,
           viewModel.deskDepthStatus.lowercased() == "success",
           let bid = viewModel.deskDepthBids.first?.price,
           let ask = viewModel.deskDepthAsks.last?.price {
            Text("\(bid) / \(ask)")
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        } else if !viewModel.deskDepthDisplay {
            HonestyChip(status: HonestyStatus.fromWire(viewModel.deskDepthStatus) ?? .unavailable)
        } else if let honesty = HonestyStatus.fromWire(viewModel.deskDepthStatus) {
            HonestyChip(status: honesty)
        } else {
            Text(viewModel.deskDepthStatus)
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    private var marginValue: String {
        let m = viewModel.futuresMarginReadout(symbol: viewModel.barDeclarationSymbol)
        if let mode = m.mode, let lev = m.leverage {
            return "\(mode) · \(lev)"
        }
        return "none"
    }

    private func subtitle(_ kind: BarCashCockpitSeed.StripKind) -> String {
        switch kind {
        case .funds:
            let book = viewModel.shippingFundsGlance.bookId.trimmingCharacters(in: .whitespacesAndNewlines)
            if book.isEmpty { return "obtain(funds) · shipping book" }
            return "obtain(funds) · \(book)"
        case .last:
            return "market/quote · \(bookLabel)"
        case .history:
            return BarDeskTemplate.historyGlanceLine(
                licensedStatus: viewModel.deskHistoryStatus,
                licensedIneligible: viewModel.deskHistoryIneligible,
                yahooStatus: viewModel.deskYahooHistoryStatus,
                yahooIneligible: viewModel.deskYahooHistoryIneligible,
                stitchYahoo: false,
                productUse: viewModel.deskHistoryProductUse,
                bookId: viewModel.deskHistoryBookId,
            )
        case .depth:
            return viewModel.deskDepthPhysicsNote
        case .margin:
            let m = viewModel.futuresMarginReadout(symbol: viewModel.barDeclarationSymbol)
            let liq = m.liq ?? "none"
            return "positionRisk · read-only · liq \(liq)"
        }
    }

    private var bookLabel: String {
        let book = (viewModel.declareBookId ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if !book.isEmpty { return book }
        let chrome = viewModel.accountChrome.bookId.trimmingCharacters(in: .whitespacesAndNewlines)
        if !chrome.isEmpty { return chrome }
        return viewModel.resolvedDeskSlug ?? "desk"
    }
}
