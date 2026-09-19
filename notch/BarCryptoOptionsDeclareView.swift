import SwiftUI

/// Crypto Options Pre-trade cockpit (dated European contracts, `BTC-260925-90000-C`).
/// Mosaic + 6-cell glance strip + Plan rail. Ticket-C lives on the mosaic.
/// Money is USDT; there is no lot size, no NRML, no product code.
/// Chain and OI glance the named options book. Greeks are the venue's own published mark
/// table passed through per contract — lit only when the desk says `success` and the rights
/// say `display`, dark as a chip otherwise; Station never computes them. At-expiry is the
/// European cash settlement identity (S from eapi index). σ rungs 2–5 stay dark.
/// Rung 1 runs on typed numbers. Confirm is LiveBook intent.
struct BarCryptoOptionsDeclareView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarOptionsGlanceStrip(viewModel: viewModel)
            HStack(alignment: .top, spacing: 10) {
                cryptoMosaic
                cryptoPlanRail
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var cryptoMosaic: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 8) {
                cockpitTile(title: "Session", note: "GET /eapi/v1/klines") {
                    sessionChart
                }
                cockpitTile(title: "Open interest", note: "GET /eapi/v1/openInterest") {
                    oiHost
                }
            }
            HStack(alignment: .top, spacing: 8) {
                cockpitTile(title: "At expiry", note: "eapi index · European") {
                    atExpiryPanel
                }
                cockpitTile(title: "Depth", note: "GET /eapi/v1/depth") {
                    BarDeskDepthLadder(
                        status: viewModel.deskDepthStatus,
                        display: viewModel.deskDepthDisplay,
                        bids: viewModel.deskDepthBids,
                        asks: viewModel.deskDepthAsks,
                        physicsNote: viewModel.deskDepthPhysicsNote,
                    )
                }
            }
            cockpitTile(title: "Chain · catalog", note: "showsStrikeGrid = false") {
                chainHost
            }
            cockpitTile(title: "Ticket", note: "type · TIF · post-only") {
                BarDeskTicketTile(
                    viewModel: viewModel,
                    sideBuy: $sideBuy,
                    quantityText: $quantityText,
                )
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private var cryptoPlanRail: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Text("PLAN")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(1.6)
                Text("USDT · contracts · TRADE parked · ticket on mosaic")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("State check")
                stateCheck

                groupLab("Contract")
                contractFields
                contractPills

                groupLab("Legs")
                legsHost

                groupLab("Risk")
                riskFields

                groupLab("Horizon — how far ahead the σ rungs look")
                horizonPills
                Text("Set by your style chip. Edit it and every σ rung recomputes — the horizon is part of the number, not a setting.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("Invalidation")
                premortem

                groupLab("Greeks")
                greeksGrid
                Text(greeksProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("Ladder")
                ladderHeader
                ladderRungs
                Text(ladderProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                if viewModel.showsConfirmControl {
                    confirmBar
                }
            }
            .padding(.vertical, 4)
        }
        .frame(minWidth: 260, idealWidth: 300, maxWidth: 360, alignment: .topLeading)
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
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    // MARK: Zone A

    private var analyticsZone: some View {
        optionsZone(
            title: "Analytics",
            note: "market/quote · market/option_chain · market/open_interest · market/order_book · market/ohlcv",
        ) {
            VStack(alignment: .leading, spacing: 12) {
                panelHead("Session", trailing: sessionTrailing)
                lastStrip
                sessionChart
                Text("History · GET /eapi/v1/klines on binance-com-options. Empty is unavailable, not a zero candle. Spot XRPUSDT klines stay off this panel.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                panelHead("Chain · around your strike", trailing: chainTrailing)
                chainHost
                panelHead("Open interest", trailing: oiTrailing)
                oiHost
                panelHead("Depth", trailing: depthTrailing)
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

    /// Only vocabulary on this strip is the desk's own wire status — nothing is hardcoded.
    private var sessionTrailing: String {
        viewModel.deskHistoryStatus
    }

    /// Last is live on this desk. The number shown is the entry the desk prefilled or you
    /// typed — never a second quote invented here.
    private var lastStrip: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Last · your entry")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                Text("market/quote · prefills the entry premium while the desk quotes")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 8)
            VStack(alignment: .trailing, spacing: 2) {
                if let entry = entryPrice {
                    Text(usdt(entry))
                        .font(BarDS.monoFont(15, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    if let freshness = BarDeskLastFormatting.freshnessBesideLast(
                        status: viewModel.deskLastStatus
                    ) {
                        Text(freshness)
                            .font(BarDS.monoFont(10, weight: .regular))
                            .foregroundColor(BarDS.Accent.amber)
                    }
                } else if let honesty = HonestyStatus.fromWire(viewModel.deskLastStatus) {
                    HonestyChip(status: honesty)
                } else {
                    Text(viewModel.deskLastStatus)
                        .font(BarDS.monoFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                }
            }
        }
        .padding(11)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    @ViewBuilder
    private var sessionChart: some View {
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
                .frame(maxWidth: .infinity, minHeight: 88)
                .background(BarDS.Fill.elevated)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                        .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
                )
        } else {
            sessionChartHole
        }
    }

    private var sessionChartHole: some View {
        VStack(spacing: 6) {
            Text("no session series")
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text("market/ohlcv · GET /eapi/v1/klines · unavailable")
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

    private var chainPresentation: BarOptionsChainPresentation {
        BarOptionsChainPresentation.from(
            underlying: viewModel.barDeclarationSymbol,
            chainStatus: viewModel.deskChainStatus,
        )
    }

    private var chainTrailing: String {
        switch chainPresentation {
        case .nothingDeclared: return "—"
        case .unavailable: return "unavailable"
        case .empty: return "empty"
        case .lit: return viewModel.deskChainStatus
        }
    }

    @ViewBuilder
    private var chainHost: some View {
        switch chainPresentation {
        case .nothingDeclared:
            emptyBlock(title: "Nothing declared yet", body: "Type an underlying and the chain around your strike appears here.")
        case .unavailable:
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: .unavailable)
                Text("BoundedSnapshot is complete-or-refused. Chain rows come from eapi `optionSymbols` for this contract’s underlying and expiry — not from NFO, not from a fake `/optionChain`.")
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
                    body: "Rows come from the Binance options catalog. A missing row means that contract is absent.",
                )
            } else {
                chainCatalog
            }
        }
    }

    /// Catalog list from `optionSymbols`. Click binds the mixed-case id (same as paste).
    /// Not a CE/PE/IV/OI strike grid. `showsStrikeGrid` stays false.
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
                Button {
                    viewModel.bindChainCatalogSymbol(row.symbol)
                } label: {
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        catalogCell(row.symbol, emphasis: row.symbol == viewModel.deskSelectedInstrumentId)
                        catalogCell(row.strikeRaw)
                        catalogCell(row.side)
                        catalogCell(row.expiryRaw)
                        catalogCell(row.last)
                    }
                    .padding(.vertical, 5)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
            Text("optionSymbols for this underlying + expiry. Click binds. No IV column. Not `/eapi/v1/optionChain`.")
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

    private func catalogCell(_ value: String?, emphasis: Bool = false) -> some View {
        Text(value ?? "")
            .font(BarDS.monoFont(11, weight: emphasis ? .medium : .regular))
            .foregroundColor(emphasis ? BarDS.Accent.teal : BarDS.Text.primary)
            .lineLimit(1)
            .minimumScaleFactor(0.7)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var depthTrailing: String {
        if !viewModel.deskDepthDisplay { return "unavailable" }
        let wire = viewModel.deskDepthStatus.trimmingCharacters(in: .whitespacesAndNewlines)
        return wire.isEmpty ? "unavailable" : wire
    }

    private var oiTrailing: String {
        let wire = viewModel.deskOiStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if wire.isEmpty || wire == "unavailable" { return "unavailable" }
        return viewModel.deskOiStatus
    }

    @ViewBuilder
    private var oiHost: some View {
        if let sum = viewModel.deskOiSumOpenInterest {
            oiLatestExact(sum)
        } else if !viewModel.deskOiRows.isEmpty {
            oiLatestRows
        } else {
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: .unavailable)
                Text("market/open_interest · `GET /eapi/v1/openInterest` `sumOpenInterest`. NFO quote JSON still does not name OI.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    /// Exact-match LatestState: the venue's own `sumOpenInterest` string, digit for digit.
    private func oiLatestExact(_ sum: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(sum)
                .font(BarDS.monoFont(15, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
            if let symbol = viewModel.deskOiSymbol {
                Text(symbol)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            if let usd = viewModel.deskOiSumOpenInterestUsd {
                Text("sumOpenInterestUsd \(usd)")
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            if let timestamp = viewModel.deskOiTimestamp {
                Text(timestamp)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            Text("LatestState · GET /eapi/v1/openInterest. Not depth. Not a strike grid.")
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

    /// No exact symbol match: list the expiry rows. Never invent a total.
    private var oiLatestRows: some View {
        VStack(alignment: .leading, spacing: 0) {
            ForEach(Array(viewModel.deskOiRows.enumerated()), id: \.offset) { _, row in
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(row.symbol)
                        .font(BarDS.monoFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.primary)
                    Spacer(minLength: 8)
                    if let oi = row.sumOpenInterest {
                        Text(oi)
                            .font(BarDS.monoFont(12, weight: .medium))
                            .foregroundColor(BarDS.Text.primary)
                    }
                }
                .padding(.vertical, 6)
            }
            Text("LatestState for this underlying + expiry. Not depth. Not a strike grid.")
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

    // MARK: Zone B

    private var consequenceZone: some View {
        optionsZone(
            title: "Consequence",
            note: "derived/greeks · venue_published mark pass-through, per contract",
        ) {
            VStack(alignment: .leading, spacing: 12) {
                panelHead("Position", trailing: netPremiumLine)
                legsHost
                greeksGrid
                Text(greeksProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                panelHead("At expiry", trailing: "per contract · unit not applied")
                atExpiryPanel

                ladderHeader
                ladderRungs
                marginRow
                Text(ladderProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                legendRow
            }
        }
    }

    private var hasLegs: Bool { !viewModel.optionLegs.isEmpty }

    private var netPremiumLine: String { "premium unavailable" }

    @ViewBuilder
    private var legsHost: some View {
        if viewModel.optionLegs.isEmpty {
            emptyBlock(title: "No legs yet", body: "Pick a strike, a side, and a number of contracts — then add the leg. The ladder below fills as you go.")
        } else {
            VStack(spacing: 7) {
                ForEach(Array(viewModel.optionLegs.enumerated()), id: \.offset) { index, leg in
                    HStack(spacing: 10) {
                        Text(leg.sideBuy ? "BUY" : "SELL")
                            .font(BarDS.monoFont(10, weight: .bold))
                            .foregroundColor(leg.sideBuy ? BarDS.Accent.teal : BarDS.Accent.red)
                            .padding(.vertical, 3)
                            .padding(.horizontal, 7)
                            .background((leg.sideBuy ? BarDS.Accent.teal : BarDS.Accent.red).opacity(0.10))
                            .clipShape(RoundedRectangle(cornerRadius: 5, style: .continuous))
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(leg.underlying) \(leg.strike) \(rightWord(leg.right))")
                                .font(BarDS.monoFont(12, weight: .medium))
                                .foregroundColor(BarDS.Text.primary)
                            Text("\(leg.lots) contract\(leg.lots == 1 ? "" : "s") · \(leg.expiry.isEmpty ? "expiry unset" : leg.expiry)")
                                .font(BarDS.monoFont(10, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        Spacer(minLength: 8)
                        Text("px unavailable")
                            .font(BarDS.monoFont(11, weight: .medium))
                            .foregroundColor(BarDS.Accent.red)
                        Button {
                            viewModel.optionLegs.remove(at: index)
                        } label: {
                            Text("×")
                                .font(BarDS.bodyFont(15, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Remove leg")
                    }
                    .padding(.vertical, 9)
                    .padding(.horizontal, 11)
                    .background(BarDS.Fill.elevated)
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                            .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
                    )
                }
            }
        }
    }

    /// Wire keeps the `CE`/`PE` vocabulary; the crypto surface never prints an NFO code.
    private func rightWord(_ right: String) -> String {
        right == "PE" ? "Put" : "Call"
    }

    /// Mark is per contract, exactly like OI — legs are a local plan and do not gate it.
    /// No unit suffixes: the Binance mark table names none, so Station must not name any.
    private var greeksCells: [(label: String, value: String?)] {
        [
            ("Delta", viewModel.deskGreeksDelta),
            ("Gamma", viewModel.deskGreeksGamma),
            ("Theta", viewModel.deskGreeksTheta),
            ("Vega", viewModel.deskGreeksVega),
        ]
    }

    /// Dark chip in the shared dialect. Lit is not a fifth honesty state, so a lit row
    /// never reaches this — an unrecognised wire word is still `unavailable`.
    private var greeksChipStatus: HonestyStatus {
        HonestyStatus.fromWire(viewModel.deskGreeksStatus) ?? .unavailable
    }

    private var greeksGrid: some View {
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 1) {
            ForEach(greeksCells, id: \.label) { cell in
                VStack(alignment: .leading, spacing: 4) {
                    Text(cell.label.uppercased())
                        .font(BarDS.monoFont(9.5, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                    if let value = cell.value {
                        // The venue's own string, digit for digit — never reformatted.
                        Text(value)
                            .font(BarDS.monoFont(12, weight: .medium))
                            .foregroundColor(BarDS.Text.primary)
                            .lineLimit(1)
                            .minimumScaleFactor(0.7)
                    } else {
                        HonestyChip(status: greeksChipStatus)
                    }
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(BarDS.Fill.elevated)
            }
        }
        .background(BarDS.Border.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
    }

    /// Three states, three sentences. Lit names the origin the envelope carried. A desk
    /// that asked and got a hole is *not* waiting — say what came back instead of
    /// claiming a pending request. Only a desk with no dated contract is waiting.
    private var greeksProv: String {
        if !viewModel.deskGreeksProv.isEmpty {
            return viewModel.deskGreeksProv
        }
        if viewModel.deskGreeksAsked {
            return "derived/greeks · \(viewModel.deskGreeksStatus) — Station copies the venue's mark table; it does not price this book."
        }
        return "derived/greeks — waiting on a declared contract."
    }

    private func payoffHole(reason: String) -> some View {
        VStack(spacing: 6) {
            Text(BarCryptoSettlement.holeTitle)
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text(reason)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .multilineTextAlignment(.center)
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

    private var settlementInputs: BarCryptoSettlement.Inputs {
        let selected = viewModel.deskSelectedInstrumentId
        let dated = InstrumentTickBookId.isDatedOptionContract(selected)
            || viewModel.optionLegs.first != nil
        let first = viewModel.optionLegs.first
        let strike = first?.strike ?? viewModel.declOptionStrike
        let right = BarCryptoSettlement.Right.fromWire(first?.right ?? viewModel.declOptionRight)
        let side: BarCryptoSettlement.Side = (first?.sideBuy ?? sideBuy) ? .buy : .sell
        let count = Int(planUnits ?? 0)
        let indexLit = viewModel.deskIndexStatus
            .trimmingCharacters(in: .whitespacesAndNewlines)
            .lowercased() == "success"
        return BarCryptoSettlement.Inputs(
            datedContractBound: dated,
            strikeText: strike,
            right: right,
            side: side,
            premiumText: viewModel.declEntryPrice,
            contractCount: count,
            indexPriceText: indexLit ? viewModel.deskIndexPrice : nil,
            expiryDateMs: expiryDateMs,
            nowMs: Int64(Date().timeIntervalSince1970 * 1000)
        )
    }

    private var expiryDateMs: Int64? {
        let selected = viewModel.deskSelectedInstrumentId
        let raw = viewModel.deskChainRows.first(where: { $0.symbol == selected })?.expiryRaw
            ?? viewModel.deskChainRows.first?.expiryRaw
        if let raw, let ms = Int64(raw.trimmingCharacters(in: .whitespacesAndNewlines)) {
            return ms
        }
        if InstrumentTickBookId.isDatedOptionContract(selected) {
            return BarCryptoSettlement.expiryDateMs(fromDatedContract: selected)
        }
        if let expiry = viewModel.optionLegs.first?.expiry,
           let fromLeg = BarCryptoSettlement.expiryDateMs(fromDatedContract: "X-\(expiry)-0-C")
        {
            return fromLeg
        }
        return nil
    }

    private var settlementKind: BarCryptoSettlement.Kind {
        BarCryptoSettlement.evaluate(settlementInputs)
    }

    private var calendarDTEForChip: Int? {
        if case let .lit(lit) = settlementKind, lit.dteKnown { return lit.dte }
        if let ms = expiryDateMs {
            return BarCryptoSettlement.calendarDTE(
                expiryDateMs: ms,
                nowMs: Int64(Date().timeIntervalSince1970 * 1000)
            )
        }
        return nil
    }

    @ViewBuilder
    private var atExpiryPanel: some View {
        switch settlementKind {
        case let .hole(reason):
            payoffHole(reason: reason)
        case let .lit(lit):
            VStack(alignment: .leading, spacing: 6) {
                BarCryptoSettlementPolyline(
                    points: lit.points,
                    spotS: lit.spotS,
                    openWing: lit.openWing
                )
                .frame(maxWidth: .infinity, minHeight: 88)
                .background(BarDS.Fill.elevated)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                        .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
                )
                Text(lit.caption)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var declaredMax: Double? {
        Double(viewModel.declMaxPlannedLossText.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private var entryPrice: Double? {
        Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// Contracts, never lots — Binance options carry no lot size.
    private var planUnits: Double? {
        if let first = viewModel.optionLegs.first {
            return Double(first.lots)
        }
        let t = viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let contracts = Int(t), contracts > 0 else { return nil }
        return Double(contracts)
    }

    /// Same shape as the NFO rung 1 — typed numbers, else the declared limit as the anchor.
    /// Kept local so no INR-named helper renders a USDT figure.
    private var stopRung: Double? {
        if let rung = BarPlanLadder.rung1(
            units: planUnits,
            entry: entryPrice,
            stop: Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)),
            sideBuy: sideBuy,
        ) {
            return rung
        }
        if let declaredMax {
            return -abs(declaredMax)
        }
        return nil
    }

    private var ladderHeader: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let rung = stopRung {
                Text(usdt(rung))
                    .font(BarDS.monoFont(27, weight: .medium))
                    .foregroundColor(overLimit(rung) ? BarDS.Accent.red : BarDS.Text.primary)
                Text(worstCap(rung))
                    .font(BarDS.bodyFont(11.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            } else {
                Text("—")
                    .font(BarDS.monoFont(27, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
                Text(hasLegs
                    ? "Greeks stay dark, so only the rung you typed can light. That is the honest answer."
                    : "Add a leg and a contract count to see what this can cost you.")
                    .font(BarDS.bodyFont(11.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Text("horizon · \(horizonNote)")
                .font(BarDS.monoFont(10.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
    }

    private func overLimit(_ rung: Double) -> Bool {
        guard let max = declaredMax else { return false }
        return abs(rung) > max
    }

    private func worstCap(_ rung: Double) -> String {
        if let max = declaredMax, abs(rung) > max {
            return String(format: "worst case %@ — %.1f× the loss you planned for", horizonPhrase, abs(rung) / max)
        }
        if declaredMax != nil {
            return "worst case \(horizonPhrase) — inside the limit you declared"
        }
        return "Set a stop and a max planned loss to light the first rung."
    }

    /// Crypto trades round the clock — the horizon counts days, not exchange sessions.
    private var horizonPhrase: String {
        viewModel.declHorizonDays == 1 ? "in the next 24h" : "over \(viewModel.declHorizonDays) days"
    }

    private var horizonNote: String {
        let days = viewModel.declHorizonDays
        let left = days == 1 ? "next 24h" : "\(days) days"
        return "\(left) · σ unavailable"
    }

    /// Rung 1 is typed. Everything below it needs the chain, so it stays dark.
    private var ladderRungList: [BarCryptoLadderRung] {
        let stopTrim = stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)
        let stopSub = stopTrim.isEmpty
            ? "set a stop premium to light this rung"
            : "premium at \(stopTrim) USDT · typed, no market data needed"
        let rung1 = hasLegs ? stopRung : nil
        return [
            BarCryptoLadderRung(
                label: "At your stop",
                sub: stopSub,
                amountUSDT: rung1,
                honesty: rung1 == nil ? .empty : nil,
            ),
            BarCryptoLadderRung(
                label: "1σ adverse \(horizonPhrase)",
                sub: "σ scaled from chain IV",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "2σ / gap \(horizonPhrase)",
                sub: "σ scaled from chain IV",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "Terminal at expiry",
                sub: "payoff needs premiums from the chain",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "Distance to breach your limit",
                sub: declaredMax == nil ? "set a max planned loss" : "needs Greeks to solve for the move",
                amountUSDT: nil,
                honesty: declaredMax == nil ? .empty : .unavailable,
            ),
        ]
    }

    private var ladderRungs: some View {
        VStack(spacing: 0) {
            if !hasLegs {
                emptyBlock(title: "The ladder is empty", body: "Every rung below fills from something you declare. Nothing is guessed for you.")
                    .padding(.vertical, 8)
            } else {
                ForEach(Array(ladderRungList.enumerated()), id: \.offset) { _, rung in
                    ladderRungRow(rung)
                }
            }
        }
    }

    private func ladderRungRow(_ rung: BarCryptoLadderRung) -> some View {
        HStack(alignment: .center, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(rung.label)
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(rung.honesty == nil ? BarDS.Text.primary : BarDS.Text.muted)
                Text(rung.sub)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 8)
            if let amt = rung.amountUSDT {
                VStack(alignment: .trailing, spacing: 2) {
                    Text(usdt(amt))
                        .font(BarDS.monoFont(15, weight: .medium))
                        .foregroundColor(overLimit(amt) ? BarDS.Accent.red : BarDS.Text.primary)
                    if let max = declaredMax, max > 0 {
                        Text(String(format: "%.1f× declared", abs(amt) / max))
                            .font(BarDS.monoFont(10.5, weight: .regular))
                            .foregroundColor(BarDS.Text.muted)
                    }
                }
            } else if let honesty = rung.honesty {
                HonestyChip(status: honesty)
            }
        }
        .padding(.vertical, 9)
        .overlay(alignment: .bottom) {
            Rectangle().fill(BarDS.Border.row).frame(height: 0.5)
        }
    }

    private var marginRow: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Margin blocked")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                Text("the venue's margin engine is Binance's number, never ours")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            Spacer(minLength: 8)
            HonestyChip(status: .unavailable)
        }
        .padding(11)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var ladderProv: String {
        "rung 1 · your typed numbers only. Rungs 2–5 stay dark — σ / IV / greeks unspecified."
    }

    private var legendRow: some View {
        VStack(alignment: .leading, spacing: 4) {
            legendItem(BarDS.Accent.teal.opacity(0.35), "within your declared limit")
            legendItem(BarDS.Accent.red.opacity(0.45), "past your declared limit")
            legendItem(nil, "inherited dark — waiting on an input", dashed: true)
            legendItem(BarDS.Accent.red.opacity(0.4), "unavailable — source refused")
        }
        .padding(.top, 4)
    }

    private func legendItem(_ fill: Color?, _ label: String, dashed: Bool = false) -> some View {
        HStack(spacing: 6) {
            RoundedRectangle(cornerRadius: 2)
                .stroke(BarDS.Text.muted, style: StrokeStyle(lineWidth: 1, dash: dashed ? [3, 2] : []))
                .background(fill ?? Color.clear)
                .frame(width: 9, height: 9)
            Text(label)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    // MARK: Zone C

    private var planAndConfirm: some View {
        VStack(alignment: .leading, spacing: 12) {
            planZone
            if viewModel.showsConfirmControl {
                confirmBar
            }
        }
    }

    private var planZone: some View {
        optionsZone(title: "Plan", note: "what you are declaring") {
            VStack(alignment: .leading, spacing: 16) {
                groupLab("State check")
                stateCheck

                groupLab("Contract")
                contractFields
                contractPills
                ticketFields

                groupLab("Risk")
                riskFields

                groupLab("Horizon — how far ahead the σ rungs look")
                horizonPills
                Text("Set by your style chip. Edit it and every σ rung recomputes — the horizon is part of the number, not a setting.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("Invalidation")
                premortem
            }
        }
    }

    private var stateCheck: some View {
        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        if (1 ... 5).contains(calm), (1 ... 5).contains(conf) {
            let cw = ["Calm", "Focused", "Tense", "Anxious", "Angry"][calm - 1]
            let cf = ["Low", "Flat", "Neutral", "Good", "Sharp"][conf - 1]
            return AnyView(
                HStack(spacing: 10) {
                    Text("CALM")
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                    Text("\(calm) · \(cw)")
                        .font(BarDS.monoFont(12, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    Text("CONFIDENCE")
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                    Text("\(conf) · \(cf)")
                        .font(BarDS.monoFont(12, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    Spacer(minLength: 8)
                    Button("change") {
                        viewModel.declEmotionalCalm = 0
                        viewModel.declEmotionalConfidence = 0
                    }
                    .buttonStyle(.plain)
                    .font(BarDS.bodyFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                }
                .padding(9)
                .background(BarDS.Fill.elevated)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            )
        }
        return AnyView(
            VStack(alignment: .leading, spacing: 10) {
                Text("Two readings. Answer honestly — it changes what the system flags.")
                    .font(BarDS.bodyFont(12.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                scaleRow(
                    title: "Psychological calm — 1 is best",
                    labels: ["Calm", "Focused", "Tense", "Anxious", "Angry"],
                    value: Binding(
                        get: { viewModel.declEmotionalCalm },
                        set: { viewModel.declEmotionalCalm = $0 },
                    ),
                )
                scaleRow(
                    title: "Confidence — 5 is best",
                    labels: ["Low", "Flat", "Neutral", "Good", "Sharp"],
                    value: Binding(
                        get: { viewModel.declEmotionalConfidence },
                        set: { viewModel.declEmotionalConfidence = $0 },
                    ),
                )
            }
        )
    }

    private func scaleRow(title: String, labels: [String], value: Binding<Int>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased())
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            HStack(spacing: 7) {
                ForEach(1 ... 5, id: \.self) { i in
                    Button {
                        value.wrappedValue = i
                    } label: {
                        VStack(spacing: 2) {
                            Text("\(i)")
                                .font(BarDS.monoFont(15, weight: .medium))
                                .foregroundColor(value.wrappedValue == i ? BarDS.Accent.teal : BarDS.Text.primary)
                            Text(labels[i - 1])
                                .font(BarDS.bodyFont(10, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 9)
                        .background(value.wrappedValue == i ? BarDS.Accent.teal.opacity(0.06) : Color.clear)
                        .overlay(
                            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                                .stroke(
                                    value.wrappedValue == i ? BarDS.Accent.teal.opacity(0.35) : BarDS.Border.card,
                                    lineWidth: BarDS.borderThin,
                                ),
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    private var contractFields: some View {
        VStack(spacing: 9) {
            labeledField("Underlying", placeholder: "BTC", text: $viewModel.barDeclarationSymbol)
            HStack(spacing: 9) {
                labeledField("Expiry · YYMMDD", placeholder: "260925", text: $viewModel.declOptionExpiry)
                labeledField("Strike", placeholder: "90000", text: $viewModel.declOptionStrike)
            }
            labeledField("Contracts", placeholder: "1", text: $viewModel.declLots)
            Text("Binance sizes in contracts and settles in USDT — no lot size, no product code.")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var contractPills: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 7) {
                BarChip(label: "Call", selected: viewModel.declOptionRight == "CE") {
                    viewModel.declOptionRight = "CE"
                }
                BarChip(label: "Put", selected: viewModel.declOptionRight == "PE") {
                    viewModel.declOptionRight = "PE"
                }
                Spacer(minLength: 8)
                BarChip(label: "Buy", selected: sideBuy) { sideBuy = true }
                BarChip(label: "Sell", selected: !sideBuy) { sideBuy = false }
            }
            Button(action: addLeg) {
                Text("＋ Add this leg")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .padding(.vertical, 6)
                    .padding(.horizontal, 13)
                    .overlay(
                        Capsule().stroke(BarDS.Border.chipUnselected, lineWidth: BarDS.borderThin),
                    )
            }
            .buttonStyle(.plain)
        }
    }

    private var ticketFields: some View {
        let spec = BarDeskTicketSpec.forBook(BarDeskTemplate.binanceComOptionsBookId)
        let illegal = BarDeskTicketIllegal.reason(
            bookId: BarDeskTemplate.binanceComOptionsBookId,
            instrumentId: viewModel.deskSelectedInstrumentId,
            type: viewModel.deskTicket.type,
        )
        return VStack(alignment: .leading, spacing: 8) {
            groupLab("Ticket")
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

    /// `lots` is the payload's size field; on this desk it carries contracts.
    private func addLeg() {
        let contracts = Int(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)) ?? 0
        let strike = viewModel.declOptionStrike.trimmingCharacters(in: .whitespacesAndNewlines)
        let und = viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard contracts > 0, !strike.isEmpty, !und.isEmpty else { return }
        viewModel.barDeclarationSymbol = und
        viewModel.optionLegs.append(
            BarIntradayDeclarationPayload.OptionLeg(
                underlying: und,
                expiry: viewModel.declOptionExpiry.trimmingCharacters(in: .whitespacesAndNewlines),
                strike: strike,
                right: viewModel.declOptionRight == "PE" ? "PE" : "CE",
                sideBuy: sideBuy,
                lots: contracts,
            ),
        )
    }

    private var riskFields: some View {
        VStack(spacing: 9) {
            labeledField("Entry — premium in USDT", placeholder: "0.0012", text: $viewModel.declEntryPrice)
            labeledField("Stop — exact premium in USDT", placeholder: "0.0006", text: $stopLossText)
            labeledField("Target — premium in USDT", placeholder: "0.0030", text: $targetPriceText)
            labeledField("Max planned loss USDT", placeholder: "250", text: $viewModel.declMaxPlannedLossText)
        }
    }

    private var horizonPills: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 7) {
                horizonChip(.today)
                horizonChip(.threeSessions)
                horizonChip(.expiry)
            }
            HStack(spacing: 9) {
                Slider(
                    value: Binding(
                        get: { Double(viewModel.declHorizonDays) },
                        set: {
                            viewModel.declHorizonDays = BarPlanHorizon.clamp(Int($0.rounded()))
                            viewModel.declHorizonMode = BarPlanHorizon.mode(
                                forDays: viewModel.declHorizonDays,
                                dte: nil,
                            )
                        },
                    ),
                    in: Double(BarPlanHorizon.dayRange.lowerBound) ... Double(BarPlanHorizon.dayRange.upperBound),
                    step: 1,
                )
                Text(horizonSliderLabel)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .frame(minWidth: 88, alignment: .trailing)
            }
            HStack(spacing: 6) {
                Text("DTE")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                if let dte = calendarDTEForChip {
                    Text("\(dte)")
                        .font(BarDS.monoFont(12, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                } else {
                    HonestyChip(status: .unavailable)
                }
            }
        }
    }

    private var horizonSliderLabel: String {
        let d = viewModel.declHorizonDays
        if d == 1 { return "1 day (next 24h)" }
        return "\(d) days"
    }

    private func horizonChip(_ mode: BarPlanHorizon.Mode) -> some View {
        let selected = viewModel.declHorizonMode == mode
        return BarChip(label: mode.chipLabel, selected: selected) {
            viewModel.declHorizonMode = mode
            if let days = BarPlanHorizon.days(for: mode, dte: calendarDTEForChip) {
                viewModel.declHorizonDays = days
            }
            // expiry with unknown DTE: chip records intent, days stay as-is
        }
    }

    /// USDT twin of `BarOptionsPremortem` — the shared helper prints ₹.
    private var premortemQuestion: String {
        if hasLegs, let max = declaredMax {
            return "This trade hit your declared limit of \(usdt(max)). What happened?"
        }
        return "Write what would prove this trade wrong."
    }

    private var premortemHint: String {
        if hasLegs, declaredMax != nil {
            return "σ rungs are dark, so this is seeded from your declared limit instead of the chain."
        }
        return "Fill the contract and size, and this question gets sharper."
    }

    private var premortem: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(premortemQuestion)
                .font(BarDS.bodyFont(13.5, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .fixedSize(horizontal: false, vertical: true)
            Text(premortemHint)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
            ZStack(alignment: .topLeading) {
                if viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text("It failed because…")
                        .font(BarDS.bodyFont(12, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
                        .padding(.top, 10)
                        .padding(.leading, 6)
                }
                TextEditor(text: $viewModel.declInvalidationCondition)
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 60)
                    .padding(.vertical, 4)
                    .padding(.horizontal, 4)
            }
            .padding(8)
            .background(BarDS.Fill.elevated)
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(BarDS.Border.input, lineWidth: BarDS.borderThin),
            )
        }
        .padding(12)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.subtle, lineWidth: BarDS.borderThin),
        )
    }

    /// Submit is gated by the caller's readiness, the busy flag, and the live kill switch.
    private var submitBlocked: Bool {
        viewModel.barLiveState?.blocksDeclarationSubmit == true
    }

    private var confirmBar: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let e = viewModel.barDeclarationLastError, !e.isEmpty {
                Text(e)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            }
            BarBigButton(
                label: viewModel.barDeclarationBusy ? "Submitting…" : "Confirm — enter trade",
                style: .primary,
                action: onConfirm,
            )
            .disabled(!submitReady || viewModel.barDeclarationBusy || submitBlocked || !viewModel.showsConfirmControl)
            .opacity(submitReady && !viewModel.barDeclarationBusy && !submitBlocked ? 1 : 0.3)
            if !submitReady, let hint = submitHint, !viewModel.barDeclarationBusy {
                Text(hint)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .padding(.top, 4)
    }

    // MARK: money

    /// USDT never rounds a small premium away: `0.001` prints `0.001 USDT`, not `0.00`.
    /// Two places at and above 1, then enough places to keep four significant digits.
    private func usdt(_ value: Double) -> String {
        let magnitude = abs(value)
        var places = 2
        if magnitude > 0, magnitude < 1 {
            places = min(8, max(2, 3 - Int(floor(log10(magnitude)))))
        }
        var text = String(format: "%.\(places)f", magnitude)
        while places > 2, text.hasSuffix("0") {
            text.removeLast()
            places -= 1
        }
        // Below the last fixed place, print the exponent rather than a false zero.
        if magnitude > 0, Double(text) == 0 {
            text = String(format: "%.2e", magnitude)
        }
        return "\(text) USDT"
    }

    // MARK: chrome

    private func optionsZone<Content: View>(
        title: String,
        note: String,
        @ViewBuilder content: () -> Content,
    ) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 10) {
                Text(title.uppercased())
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(1.6)
                Rectangle().fill(BarDS.Border.card).frame(height: 1)
                Text(note)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .lineLimit(1)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            content()
                .padding(14)
        }
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func panelHead(_ title: String, trailing: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(BarDS.Text.secondary)
            Spacer(minLength: 8)
            Text(trailing)
                .font(BarDS.monoFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .lineLimit(2)
                .multilineTextAlignment(.trailing)
        }
    }

    private func groupLab(_ text: String) -> some View {
        Text(text.uppercased())
            .font(BarDS.monoFont(10, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .kerning(1.4)
    }

    private func labeledField(_ label: String, placeholder: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label.uppercased())
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            BarInputField(placeholder: placeholder, text: text, marginBottom: 0)
        }
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
}

/// Crypto twin of `BarOptionsLadderRung` — same shape, USDT amount instead of ₹.
private struct BarCryptoLadderRung {
    var label: String
    var sub: String
    var amountUSDT: Double?
    var honesty: HonestyStatus?
}

/// European settlement polyline. Short call keeps an open right wing — no numeric cap.
struct BarCryptoSettlementPolyline: View {
    let points: [BarCryptoSettlement.Point]
    let spotS: Double
    let openWing: Bool

    var body: some View {
        Canvas { context, size in
            guard points.count >= 2, size.width > 0, size.height > 0 else { return }
            let xs = points.map(\.s)
            let ys = points.map(\.pnl)
            let minX = xs.min() ?? 0
            var maxX = xs.max() ?? 1
            var minY = ys.min() ?? 0
            var maxY = ys.max() ?? 1
            if openWing {
                maxX = max(maxX, spotS) + max(abs(maxX - minX) * 0.15, 1)
            }
            if maxX <= minX { maxX = minX + 1 }
            if maxY <= minY { maxY = minY + 1 }
            let pad = (maxY - minY) * 0.12
            minY -= pad
            maxY += pad
            func x(_ s: Double) -> CGFloat {
                CGFloat((s - minX) / (maxX - minX)) * size.width
            }
            func y(_ pnl: Double) -> CGFloat {
                let t = (pnl - minY) / (maxY - minY)
                return size.height - CGFloat(t) * size.height
            }
            var line = Path()
            let ordered = points.sorted { $0.s < $1.s }
            if let first = ordered.first {
                line.move(to: CGPoint(x: x(first.s), y: y(first.pnl)))
                for p in ordered.dropFirst() {
                    line.addLine(to: CGPoint(x: x(p.s), y: y(p.pnl)))
                }
                if openWing, let last = ordered.last {
                    let edge = CGPoint(x: size.width, y: y(last.pnl))
                    line.addLine(to: edge)
                }
            }
            context.stroke(line, with: .color(BarDS.Accent.teal), lineWidth: 1.5)
            let sx = x(spotS)
            var marker = Path()
            marker.move(to: CGPoint(x: sx, y: 0))
            marker.addLine(to: CGPoint(x: sx, y: size.height))
            context.stroke(marker, with: .color(BarDS.Text.muted.opacity(0.6)), lineWidth: 1)
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .accessibilityLabel("Settlement payoff")
    }
}
