import SwiftUI

/// NFO options cockpit mosaic (prototype seed). Honesty hosts only — no fake klines.
struct BarNfoCockpitMosaic: View {
    @ObservedObject var viewModel: NotchViewModel
    var tiles: [BarNfoCockpitSeed.Tile]
    var editing: Bool = false
    var onRemove: ((BarNfoCockpitSeed.Kind) -> Void)?

    var body: some View {
        let placements = tiles.map {
            BarCockpitMosaicPlacement(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
        BarCockpitMosaicGrid(placements: placements) {
            ForEach(tiles, id: \.kind) { tile in
                cockpitTile(
                    title: BarNfoCockpitSeed.title(for: tile.kind),
                    note: BarNfoCockpitSeed.note(for: tile.kind),
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
    private func host(for kind: BarNfoCockpitSeed.Kind) -> some View {
        switch kind {
        case .session: sessionHole
        case .oi: oiHost
        case .payoff: payoffHole
        case .depth:
            BarDeskDepthLadder(
                status: viewModel.deskDepthStatus,
                display: viewModel.deskDepthDisplay,
                bids: viewModel.deskDepthBids,
                asks: viewModel.deskDepthAsks,
                physicsNote: viewModel.deskDepthPhysicsNote,
            )
        case .chain: chainHost
        case .greeks: greeksHole
        case .legs: emptyBlock(title: "No legs yet", body: "Legs live on the Plan dock.")
        case .ladder: emptyBlock(title: "Ladder", body: "σ rungs 2–5 stay dark. Rung 1 is on the Plan dock.")
        case .ticket: emptyBlock(title: "No COM ticket", body: "NFO Confirm is LiveBook. TRADE parked.")
        }
    }

    private var greeksHole: some View {
        VStack(alignment: .leading, spacing: 6) {
            HonestyChip(status: HonestyStatus.fromWire(viewModel.deskGreeksStatus) ?? .unavailable)
            Text(
                BarNfoGreeksCopy.provenance(
                    asked: viewModel.deskGreeksAsked,
                    status: viewModel.deskGreeksStatus,
                    ineligible: viewModel.deskGreeksIneligible,
                )
            )
            .font(BarDS.monoFont(10, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var sessionHole: some View {
        VStack(spacing: 6) {
            Text(BarNfoHistoryCopy.sessionHoleTitle)
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text(BarNfoHistoryCopy.sessionHoleBody)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, minHeight: 88)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var chainPresentation: BarOptionsChainPresentation {
        BarOptionsChainPresentation.from(
            underlying: viewModel.barDeclarationSymbol,
            chainStatus: viewModel.deskChainStatus,
        )
    }

    @ViewBuilder
    private var chainHost: some View {
        switch chainPresentation {
        case .nothingDeclared:
            emptyBlock(title: "Nothing declared yet", body: "Type an underlying and the chain around your strike appears here.")
        case .unavailable:
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: .unavailable)
                Text("BoundedSnapshot is complete-or-refused. No strike grid while the master and chain are holes.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        case .empty:
            emptyBlock(title: "No contracts", body: "Expiry list is empty — not a guessed strike grid.")
        case .lit:
            if viewModel.deskChainRows.isEmpty {
                emptyBlock(
                    title: "chain snapshot live · no strike grid (raw strike/expiry)",
                    body: "Rows come from the NFO master. A missing master row means that strike is absent.",
                )
            } else {
                chainCatalog
            }
        }
    }

    private var chainCatalog: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 8) {
                catalogHead("symbol")
                catalogHead("strike")
                catalogHead("side")
                catalogHead("expiry")
                catalogHead("last")
            }
            .padding(.bottom, 4)
            ForEach(viewModel.deskChainRows) { row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    catalogCell(row.symbol)
                    catalogCell(row.strikeRaw)
                    catalogCell(row.side)
                    catalogCell(row.expiryRaw)
                    catalogCell(row.last)
                }
                .padding(.vertical, 5)
            }
            Text("NFO master rows for this underlying. Raw strike/expiry. No IV column. Not a strike grid.")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.top, 6)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func catalogHead(_ title: String) -> some View {
        Text(title)
            .font(BarDS.monoFont(9.5, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func catalogCell(_ value: String?) -> some View {
        Text(value ?? "")
            .font(BarDS.monoFont(11, weight: .regular))
            .foregroundColor(BarDS.Text.primary)
            .lineLimit(1)
            .minimumScaleFactor(0.7)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    @ViewBuilder
    private var oiHost: some View {
        if let openInt = viewModel.deskOiOpenInt {
            nfoOiExact(openInt)
        } else if let honesty = HonestyStatus.fromWire(viewModel.deskOiStatus) {
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: honesty)
                Text(oiHonestyNote)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        } else {
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: .unavailable)
                Text(oiHonestyNote)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var oiHonestyNote: String {
        "market/open_interest · quote field open_int. oi_las* stay dark. Not eapi sumOpenInterest."
    }

    private func nfoOiExact(_ openInt: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(openInt)
                .font(BarDS.monoFont(15, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
            if let field = viewModel.deskOiField {
                Text(field)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            Text("LatestState · quote field open_int. \"0\" is a real reading. oi_las* stay dark. Not eapi sumOpenInterest.")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var payoffHole: some View {
        VStack(spacing: 6) {
            Text(BarNfoPayoffCopy.holeTitle)
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text(BarNfoPayoffCopy.holeBody)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
        .frame(maxWidth: .infinity, minHeight: 88)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func emptyBlock(title: String, body: String) -> some View {
        VStack(spacing: 3) {
            Text(title)
                .font(BarDS.bodyFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
            Text(body)
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(16)
        .frame(maxWidth: .infinity)
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(style: StrokeStyle(lineWidth: 1, dash: [5, 4]))
                .foregroundColor(BarDS.Border.card),
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
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }
}
