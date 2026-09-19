import SwiftUI

/// Cash cockpit mosaic (prototype `CASH_SEEDS` / `LAST_SEEDS`). Session owns;
/// Depth follows on spot/equity. No fake klines.
struct BarCashCockpitMosaic: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            cockpitTile(title: "Session", note: sessionNote) {
                sessionHost
            }
            if BarCashCockpitSeed.showsDepth(for: viewModel.declareAssetClass) {
                cockpitTile(title: "Depth", note: "market/order_book") {
                    BarDeskDepthLadder(
                        status: viewModel.deskDepthStatus,
                        display: viewModel.deskDepthDisplay,
                        bids: viewModel.deskDepthBids,
                        asks: viewModel.deskDepthAsks,
                        physicsNote: viewModel.deskDepthPhysicsNote,
                    )
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    @ViewBuilder
    private var sessionHost: some View {
        if viewModel.deskHistoryStatus == "success", !viewModel.deskHistoryCandles.isEmpty {
            BarOptionsSessionChart(candles: viewModel.deskHistoryCandles)
                .frame(maxWidth: .infinity, minHeight: 140, maxHeight: .infinity)
                .background(BarDS.Fill.elevated)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                        .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
                )
        } else {
            sessionHole
        }
    }

    private var sessionHole: some View {
        VStack(spacing: 6) {
            Text(sessionHoleTitle)
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text(sessionNote)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, minHeight: 140, maxHeight: .infinity)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var sessionHoleTitle: String {
        if viewModel.declareAssetClass == .usdm || viewModel.declareAssetClass == .coinm {
            return "no History tile on this book"
        }
        return "no session series"
    }

    private var sessionNote: String {
        if viewModel.declareAssetClass == .usdm || viewModel.declareAssetClass == .coinm {
            return "Last-only glance · history stays dark"
        }
        return BarDeskTemplate.historyGlanceLine(
            licensedStatus: viewModel.deskHistoryStatus,
            licensedIneligible: viewModel.deskHistoryIneligible,
            yahooStatus: viewModel.deskYahooHistoryStatus,
            yahooIneligible: viewModel.deskYahooHistoryIneligible,
            stitchYahoo: false,
            productUse: viewModel.deskHistoryProductUse,
            bookId: viewModel.deskHistoryBookId,
        )
    }

    private func cockpitTile<Content: View>(
        title: String,
        note: String,
        @ViewBuilder content: () -> Content,
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(title)
                    .font(BarDS.bodyFont(12, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)
                Spacer(minLength: 8)
                Text(note)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .lineLimit(1)
            }
            content()
        }
        .padding(10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 12, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }
}
