import SwiftUI

/// Limit-only crypto Options ticket. Confirm is LiveBook intent — not `POST /eapi/v1/order`.
struct BarCryptoOptionsTicketView: View {
    @ObservedObject var viewModel: NotchViewModel

    private var spec: BarDeskTicketSpec {
        BarDeskTicketSpec.forBook(BarDeskTemplate.binanceComOptionsBookId)
    }

    private var illegal: String? {
        BarDeskTicketIllegal.reason(
            bookId: BarDeskTemplate.binanceComOptionsBookId,
            instrumentId: viewModel.deskSelectedInstrumentId,
            type: viewModel.deskTicket.type,
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 4) {
                ForEach(spec.types) { t in
                    Text(spec.label(for: t))
                        .font(BarDS.bodyFont(11, weight: .semibold))
                        .foregroundColor(BarDS.Text.primary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 7)
                        .background(Color.white.opacity(0.10))
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
            }
            HStack(spacing: 5) {
                ForEach(BarTicketTif.allCases) { t in
                    BarChip(label: t.rawValue, selected: viewModel.deskTicket.tif == t) {
                        viewModel.deskTicket.tif = t
                        viewModel.persistDeskTicket()
                    }
                }
            }
            BarChip(label: "Post-only", selected: viewModel.deskTicket.postOnly) {
                viewModel.deskTicket.postOnly.toggle()
                viewModel.persistDeskTicket()
            }
            if let illegal {
                Text(illegal)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Text("path \(spec.orderPath) · Confirm is LiveBook intent, not a venue POST")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }
}

/// Ticket-C mosaic tile. Venue fields live here; Plan rail keeps mood / setup / Confirm.
struct BarDeskTicketTile: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String

    var body: some View {
        switch BarDeskTicketSurface.surface(
            for: viewModel.declareAssetClass,
            slug: viewModel.resolvedDeskSlug,
            instrumentId: viewModel.deskSelectedInstrumentId,
        ) {
        case .spot:
            BarSpotTicketView(
                ticket: $viewModel.deskTicket,
                sideBuy: $sideBuy,
                quantityText: $quantityText,
                quoteOrderQtyText: $viewModel.quoteOrderQtyText,
                availableLine: viewModel.accountChrome.freeText == "—"
                    ? nil
                    : "available = \(viewModel.accountChrome.freeText)",
            )
            .onChange(of: viewModel.deskTicket) { _, _ in viewModel.persistDeskTicket() }
            .onChange(of: viewModel.quoteOrderQtyText) { _, _ in viewModel.persistDeskTicket() }
        case .usdm, .coinm:
            BarUsdmTicketView(
                ticket: $viewModel.deskTicket,
                sideBuy: $sideBuy,
                quantityText: $quantityText,
                triggerPriceText: $viewModel.ticketTriggerPrice,
                marginMode: futuresMargin.mode,
                leverage: futuresMargin.leverage,
                liquidationPrice: futuresMargin.liq,
            )
            .onChange(of: viewModel.deskTicket) { _, _ in viewModel.persistDeskTicket() }
            .onChange(of: viewModel.ticketTriggerPrice) { _, _ in viewModel.persistDeskTicket() }
        case .cryptoOptions:
            BarCryptoOptionsTicketView(viewModel: viewModel)
        case .none:
            EmptyView()
        }
    }

    private var futuresMargin: (mode: String?, leverage: String?, liq: String?) {
        viewModel.futuresMarginReadout(symbol: viewModel.barDeclarationSymbol)
    }
}
