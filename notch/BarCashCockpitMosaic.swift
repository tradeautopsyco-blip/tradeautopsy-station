import SwiftUI

/// Cash cockpit mosaic (prototype `CASH_SEEDS` / `LAST_SEEDS`). Session owns;
/// Depth follows on spot/equity once the lock names it. No fake klines.
struct BarCashCockpitMosaic: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var stopLossText: String
    @Binding var targetPriceText: String
    var tiles: [BarCashCockpitSeed.Tile]
    var editing: Bool = false
    var onRemove: ((BarCashCockpitSeed.Kind) -> Void)?

    var body: some View {
        let placements = tiles.map {
            BarCockpitMosaicPlacement(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
        BarCockpitMosaicGrid(placements: placements) {
            ForEach(tiles, id: \.kind) { tile in
                cockpitTile(
                    title: BarCashCockpitSeed.title(for: tile.kind),
                    note: tileNote(tile.kind),
                    removable: editing && tile.kind != .ticket,
                    onRemove: { onRemove?(tile.kind) },
                ) {
                    host(for: tile.kind)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    @ViewBuilder
    private func host(for kind: BarCashCockpitSeed.Kind) -> some View {
        switch kind {
        case .session: sessionHost
        case .depth:
            BarDeskDepthLadder(
                status: viewModel.deskDepthStatus,
                display: viewModel.deskDepthDisplay,
                bids: viewModel.deskDepthBids,
                asks: viewModel.deskDepthAsks,
                physicsNote: viewModel.deskDepthPhysicsNote,
            )
        case .ticket:
            ScrollView {
                BarDeskTicketTile(
                    viewModel: viewModel,
                    sideBuy: $sideBuy,
                    quantityText: $quantityText,
                )
                .frame(maxWidth: .infinity, alignment: .topLeading)
            }
            .scrollIndicators(.automatic)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private func tileNote(_ kind: BarCashCockpitSeed.Kind) -> String {
        switch kind {
        case .session: return sessionNote
        case .depth: return "market/order_book"
        case .ticket: return "type · size · TIF"
        }
    }

    @ViewBuilder
    private var sessionHost: some View {
        if viewModel.deskHistoryStatus == "success", !viewModel.deskHistoryCandles.isEmpty {
            BarOptionsSessionChart(
                candles: viewModel.deskHistoryCandles,
                drag: SessionChartDragBindings(
                    sideBuy: sideBuy,
                    entryText: $viewModel.declEntryPrice,
                    stopText: $stopLossText,
                    targetText: $targetPriceText,
                ),
                last: viewModel.sessionChartLast,
                symbol: viewModel.barDeclarationSymbol,
                interval: viewModel.deskHistoryInterval,
            )
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
        return "no session series"
    }

    private var sessionNote: String {
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
        removable: Bool = false,
        onRemove: (() -> Void)? = nil,
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
                if removable {
                    Button("×", action: { onRemove?() })
                        .font(BarDS.monoFont(11, weight: .medium))
                        .buttonStyle(.plain)
                        .foregroundColor(BarDS.Text.muted)
                        .accessibilityLabel("Remove \(title)")
                }
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
