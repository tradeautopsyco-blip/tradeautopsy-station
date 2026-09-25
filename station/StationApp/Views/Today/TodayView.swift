import SwiftUI
import Notch

public struct TodayView: View {
    @ObservedObject private var viewModel: TodayViewModel
    @ObservedObject private var journalViewModel: JournalViewModel
    @ObservedObject private var deskModeStore = RiskDeskModeStore.shared
    @ObservedObject private var layoutStore = TodayLayoutModeStore.shared
    private let onOpenNotch: (() -> Void)?
    @State private var inspector: ThisTradeInspectorModel?
    @State private var dismissDetect = false

    public init(
        viewModel: TodayViewModel,
        journalViewModel: JournalViewModel,
        onOpenNotch: (() -> Void)? = nil
    ) {
        self.viewModel = viewModel
        self.journalViewModel = journalViewModel
        self.onOpenNotch = onOpenNotch
    }

    public var body: some View {
        VStack(spacing: 0) {
            header
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    Text(viewModel.presentation.takeaway)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
                        .foregroundStyle(StationDS.Text.secondary)
                    if viewModel.presentation.showDegradedBanner,
                       let text = viewModel.presentation.degradedBannerText {
                        degradedBanner(text)
                    }
                    if viewModel.showCircuitBreakerBanner {
                        circuitBreakerBanner
                    }
                    switch layoutStore.mode {
                    case .daySpine:
                        daySpine
                    case .splitClocks:
                        splitClocks
                    }
                    Text(viewModel.presentation.caption)
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                }
                .padding(16)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task {
            await viewModel.load()
            await journalViewModel.load()
        }
        .refreshable {
            await viewModel.load()
            await journalViewModel.load()
        }
        .sheet(item: inspectorBinding) { model in
            ThisTradeInspectorSheet(model: model) { inspector = nil }
        }
    }

    private var inspectorBinding: Binding<ThisTradeInspectorModel?> {
        Binding(
            get: { inspector },
            set: { inspector = $0 }
        )
    }

    private var header: some View {
        HStack(spacing: 12) {
            Text("Today")
                .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                .foregroundStyle(StationDS.Text.primary)
            Text(layoutHeaderSubtitle)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            Spacer()
            Picker("Layout", selection: layoutBinding) {
                Text(TodayLayoutMode.daySpine.title).tag(TodayLayoutMode.daySpine)
                Text(TodayLayoutMode.splitClocks.title).tag(TodayLayoutMode.splitClocks)
            }
            .pickerStyle(.segmented)
            .frame(maxWidth: 280)
            .accessibilityLabel("Today layout")
            Button("Open Notch · Live") { onOpenNotch?() }
                .buttonStyle(.borderedProminent)
                .tint(StationDS.Accent.teal)
                .controlSize(.small)
        }
        .padding(.horizontal, 16)
        .frame(height: 44)
        .overlay(alignment: .bottom) {
            Rectangle().fill(StationDS.Border.divider).frame(height: StationDS.borderThin)
        }
    }

    private var layoutBinding: Binding<TodayLayoutMode> {
        Binding(
            get: { layoutStore.mode },
            set: { layoutStore.setMode($0) }
        )
    }

    /// Proto copy in the 44pt header. Demo · not live stays a prefix, not a second money owner.
    private var layoutHeaderSubtitle: String {
        let proto = layoutStore.mode == .daySpine
            ? viewModel.desk.daySpineSubtitle
            : TodayDeskPresentation.splitClocksSubtitle
        if viewModel.presentation.subtitle.contains("Demo · not live"),
           !proto.contains("Demo · not live") {
            return "Demo · not live · \(proto)"
        }
        return proto
    }

    private var daySpine: some View {
        VStack(alignment: .leading, spacing: 20) {
            remainingHeadline
            clockTiles
            closedFloorChart(mini: false)
            detectSection
            openBookForMode
            tradesSection
            heroGrid
            signalsSection
        }
    }

    private var splitClocks: some View {
        HStack(alignment: .top, spacing: 16) {
            VStack(alignment: .leading, spacing: 16) {
                Text("HAPPENED · THIS DAY")
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                    .foregroundStyle(StationDS.Text.muted)
                Text(viewModel.desk.dateHeadline)
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                Text("Closed P&L · not mixed with open MTM")
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
                happenedTile
                closedFloorChart(mini: false)
                tradesSection
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            VStack(alignment: .leading, spacing: 16) {
                Text("Now · still on")
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                    .foregroundStyle(StationDS.Text.muted)
                nowTile
                detectSection
                openBookForMode
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var remainingHeadline: some View {
        HStack(alignment: .top) {
            VStack(alignment: .leading, spacing: 6) {
                HStack(alignment: .firstTextBaseline, spacing: 8) {
                    Text(viewModel.desk.remainingText)
                        .font(StationDS.bodyFont(28, weight: .bold))
                        .foregroundStyle(StationDS.Text.primary)
                    Text("remaining")
                        .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                        .foregroundStyle(StationDS.Text.secondary)
                }
                Text(remainingUsedLine)
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
                Text("Settings floor minus closed used. Open MTM is not in this number.")
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
            Spacer()
            Text(viewModel.desk.remainingBadge)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(StationDS.Text.muted)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .overlay(
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(StationDS.Border.divider, lineWidth: 1)
                )
        }
    }

    private var clockTiles: some View {
        HStack(spacing: 10) {
            happenedTile
            nowTile
        }
    }

    private var happenedTile: some View {
        let pnl = viewModel.presentation.heroTiles.first { $0.id == "pnl" }
        return VStack(alignment: .leading, spacing: 8) {
            Text("HAPPENED")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(StationDS.Text.muted)
            Text("Closed round trips")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.secondary)
            Text(pnl?.value ?? TodayScreenPresentation.emDash)
                .font(StationDS.bodyFont(22, weight: .bold))
                .foregroundStyle(color(for: pnl?.tone ?? .empty))
            Text("\(viewModel.desk.happenedClosedCount) RTs · hero is closed only")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(StationDS.Border.divider, lineWidth: 1))
    }

    private var nowTile: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("NOW")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(StationDS.Text.muted)
            Text("Open book")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.secondary)
            Text("\(viewModel.desk.nowOpenCount)")
                .font(StationDS.bodyFont(22, weight: .bold))
                .foregroundStyle(StationDS.Text.primary)
            Text("MTM stays on the row. DualNoBlend.")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(StationDS.Border.divider, lineWidth: 1))
    }

    @ViewBuilder
    private var detectSection: some View {
        if let detect = viewModel.detectCardInput(declarations: journalViewModel.weekDeclarations),
           !dismissDetect {
            DetectCardView(
                result: DetectCard.evaluate(detect),
                onPlanInNotch: { onOpenNotch?() },
                onNotNow: { dismissDetect = true }
            )
        }
    }

    private func closedFloorChart(mini: Bool) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("This day only — closed P&L vs floor")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            TodayClosedFloorChart(
                points: viewModel.desk.chartPoints,
                floor: viewModel.desk.floorLine,
                quoteCurrency: viewModel.desk.quoteCurrency,
                brokerSlug: viewModel.desk.brokerSlug,
                bookId: viewModel.desk.brokerSlug == "kotak_neo" ? "kotak-nse-bse-cash" : nil
            )
            .frame(maxWidth: .infinity)
            .frame(height: mini ? 88 : 156)
            Text(viewModel.desk.chartReadout)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
        }
    }

    private var remainingUsedLine: String {
        let used = viewModel.desk.usedCaption
        let percent = viewModel.desk.floorUsedPercentText
        if used == TodayScreenPresentation.emDash || percent == TodayScreenPresentation.emDash {
            return TodayScreenPresentation.emDash
        }
        return "\(used) · \(percent)"
    }

    private var heroGrid: some View {
        HStack(spacing: 10) {
            ForEach(viewModel.presentation.heroTiles) { tile in
                heroTile(tile)
            }
        }
    }

    private func heroTile(_ tile: TodayHeroTilePresentation) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            Text(tile.label.uppercased())
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(StationDS.Text.muted)
            Text(tile.value)
                .font(StationDS.bodyFont(28, weight: .bold))
                .foregroundStyle(color(for: tile.tone))
            Text(tile.caption)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
                .lineLimit(2)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(16)
        .background(tileBackground(for: tile.tone))
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(tileBorder(for: tile.tone), lineWidth: 1)
        )
    }

    private var signalsSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Behavior signals")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(StationDS.Text.secondary)
                Spacer()
                Text(viewModel.presentation.signalsMeta)
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
            HStack(spacing: 10) {
                if viewModel.presentation.showSignalsUnavailableMessage {
                    signalsUnavailableMessage
                } else {
                    ForEach(viewModel.presentation.signals) { signal in
                        signalCard(signal)
                    }
                }
            }
        }
    }

    private var signalsUnavailableMessage: some View {
        HStack(spacing: 10) {
            Text(TodayScreenPresentation.emDash)
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.muted)
            Text("Signals unavailable")
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(StationDS.Text.secondary)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(StationDS.Border.divider, lineWidth: 1)
        )
    }

    private func signalCard(_ signal: TodaySignalCardPresentation) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(signal.name)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .semibold))
                Spacer()
                Text(signal.severity)
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(signalBadgeBackground(signal.tone))
                    .clipShape(Capsule())
            }
            Text(signal.description)
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(signalBackground(signal.tone))
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(signalBorder(signal.tone), lineWidth: 1)
        )
    }

    @ViewBuilder
    private var openBookForMode: some View {
        switch deskModeStore.mode {
        case .blotter:
            openBookSection
        case .notchFlip:
            openBookFilmstrip
        case .sessionTape:
            sessionTapeSection
        }
    }

    private var openBookSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Open now")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(StationDS.Text.secondary)
                Spacer()
                Text("Fill inventory · not obtain holdings · MTM stays — until a lock owns it")
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
            Text("Overnight tagging needs a session date on the row. Gap-through-SL is a sentence, not a remaining-risk tile.")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            if viewModel.presentation.showShallowImpact,
               !viewModel.presentation.shallowImpactCaption.isEmpty {
                Text(viewModel.presentation.shallowImpactCaption)
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
            if viewModel.presentation.showEmptyOpenBook {
                Text("No open positions")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                    .foregroundStyle(StationDS.Text.muted)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(14)
                    .background(StationDS.Fill.input)
                    .clipShape(RoundedRectangle(cornerRadius: 16))
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(StationDS.Border.divider, lineWidth: 1)
                    )
            } else {
                VStack(spacing: 0) {
                    HStack {
                        cell("Symbol", flex: true, header: true)
                        cell("Side", width: 56, header: true)
                        cell("Qty", width: 48, header: true, align: .trailing)
                        cell("MTM", width: 88, header: true, align: .trailing)
                        cell("Behavior", width: 88, header: true)
                        cell("Deeper", width: 56, header: true, align: .trailing)
                    }
                    .padding(.horizontal, 14)
                    .frame(height: 30)
                    .background(Color(white: 0.08))
                    ForEach(viewModel.presentation.openRows) { row in
                        HStack {
                            cell(row.symbol, flex: true, mono: true)
                            cell(row.sideText, width: 56, mono: true)
                            cell(row.qtyText, width: 48, align: .trailing, mono: true)
                            cell(row.mtmText, width: 88, align: .trailing, mono: true, tone: row.mtmTone)
                            behaviorPill(row.behaviorText)
                            Button("Deeper") { openInspector(row) }
                                .buttonStyle(.plain)
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
                                .foregroundStyle(StationDS.Accent.teal)
                                .frame(width: 56, alignment: .trailing)
                        }
                        .padding(.horizontal, 14)
                        .frame(height: 36)
                        .overlay(alignment: .bottom) {
                            Rectangle().fill(StationDS.Border.divider).frame(height: 0.5)
                        }
                    }
                }
                .background(StationDS.Fill.input)
                .clipShape(RoundedRectangle(cornerRadius: 16))
                .overlay(RoundedRectangle(cornerRadius: 16).stroke(StationDS.Border.divider, lineWidth: 1))
            }
        }
    }

    private var openBookFilmstrip: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Today · filmstrip")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            Text("Mode B — Notch is the desk. This strip is inventory, not a second blotter.")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(viewModel.presentation.openRows) { row in
                        Button {
                            openInspector(row)
                        } label: {
                            VStack(alignment: .leading, spacing: 4) {
                                Text(row.symbol)
                                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                                Text("\(row.sideText) · \(row.qtyText) · MTM \(row.mtmText)")
                                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                                    .foregroundStyle(StationDS.Text.muted)
                            }
                            .padding(10)
                            .background(StationDS.Fill.input)
                            .clipShape(RoundedRectangle(cornerRadius: 12))
                        }
                        .buttonStyle(.plain)
                    }
                    if viewModel.presentation.showEmptyOpenBook {
                        Text("No open inventory")
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(StationDS.Text.muted)
                            .padding(10)
                    }
                }
            }
        }
    }

    private var sessionTapeSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Session tape")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(StationDS.Text.secondary)
                Spacer()
                Button {
                    onOpenNotch?()
                } label: {
                    Text("Notch")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
                        .padding(.horizontal, 12)
                        .padding(.vertical, 6)
                        .background(StationDS.Accent.teal.opacity(0.15))
                        .clipShape(Capsule())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Open Notch")
            }
            Text("Pinned while still open · fill inventory, not overnight remaining-risk.")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            ScrollView(.horizontal, showsIndicators: false) {
                HStack(spacing: 8) {
                    ForEach(viewModel.presentation.openRows) { row in
                        Text("\(row.symbol) · \(row.behaviorText)")
                            .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                            .padding(.horizontal, 10)
                            .padding(.vertical, 6)
                            .background(StationDS.Fill.input)
                            .clipShape(Capsule())
                    }
                }
            }
            ForEach(viewModel.presentation.trades) { trade in
                Button {
                    inspector = ThisTradeInspectorModel(
                        symbol: trade.symbol,
                        sideText: "closed",
                        qtyText: "—",
                        planStopText: "—",
                        liveStopText: "—",
                        pulseMTMText: trade.pnlText,
                        equityIfSLCaption: "Closed round-trip. Detect is for open fills."
                    )
                } label: {
                    HStack {
                        Text(trade.timeText)
                            .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(StationDS.Text.muted)
                            .frame(width: 52, alignment: .leading)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(trade.symbol)
                                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .semibold))
                            Text("Tape card · \(trade.pnlText)")
                                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(StationDS.Text.muted)
                        }
                        Spacer()
                    }
                    .padding(12)
                    .background(StationDS.Fill.input)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)
            }
            ForEach(viewModel.presentation.openRows) { row in
                Button {
                    openInspector(row)
                } label: {
                    HStack {
                        Text("open")
                            .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(StationDS.Text.muted)
                            .frame(width: 52, alignment: .leading)
                        VStack(alignment: .leading, spacing: 2) {
                            Text(row.symbol)
                                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .semibold))
                            Text("Still on the book · MTM \(row.mtmText)")
                                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(StationDS.Text.muted)
                        }
                        Spacer()
                    }
                    .padding(12)
                    .background(StationDS.Fill.input)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func behaviorPill(_ text: String) -> some View {
        Text(text)
            .font(StationDS.monoFont(9, weight: .semibold))
            .foregroundStyle(StationDS.Text.secondary)
            .padding(.horizontal, 6)
            .padding(.vertical, 3)
            .background(StationDS.Fill.appPanel)
            .clipShape(Capsule())
            .frame(width: 88, alignment: .leading)
    }

    private func openInspector(_ row: TodayOpenRowPresentation) {
        let joined = viewModel.detectCardInput(declarations: journalViewModel.weekDeclarations)
        let quote = viewModel.lastDeskQuoteCurrency ?? ""
        let detect = DetectCard.evaluate(
            joined
                ?? DetectCardInput(
                    qty: Double(row.qtyText) ?? 0,
                    entry: nil,
                    planStop: nil,
                    liveStop: nil,
                    sideBuy: !row.sideText.uppercased().contains("SELL"),
                    accountEquity: nil,
                    tradeCurrency: quote,
                    accountCurrency: quote
                )
        )
        inspector = ThisTradeInspectorModel.fromOpenRow(
            symbol: row.symbol,
            sideText: row.sideText,
            qtyText: row.qtyText,
            mtmText: row.mtmText,
            detect: detect
        )
    }

    private var tradesSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Closed round-trips")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(StationDS.Text.secondary)
                Spacer()
                if !viewModel.presentation.tradesMeta.isEmpty {
                    Text(viewModel.presentation.tradesMeta)
                        .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                }
            }
            if viewModel.presentation.showEmptyTable {
                emptyTrades
            } else {
                tradesTable
            }
        }
    }

    private var emptyTrades: some View {
        VStack(spacing: 10) {
            Text(TodayScreenPresentation.emDash)
                .font(.system(size: 28))
                .foregroundStyle(StationDS.Text.muted)
            Text("No closed trades yet today")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            Text("Round-trips appear after a buy→sell cycle completes.")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 48)
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(StationDS.Border.divider, style: StrokeStyle(lineWidth: 1, dash: [6]))
        )
    }

    private var tradesTable: some View {
        VStack(spacing: 0) {
            tableHeader
            ForEach(viewModel.presentation.trades) { row in
                tableRow(row)
            }
        }
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(StationDS.Border.divider, lineWidth: 1))
    }

    private var tableHeader: some View {
        HStack {
            cell("Time", width: 52, header: true)
            cell("Symbol", flex: true, header: true)
            cell("Side", width: 56, header: true)
            cell("Hold", width: 56, header: true)
            cell("Net", width: 88, header: true, align: .trailing)
            cell("Flag", width: 92, header: true, align: .trailing)
            if viewModel.presentation.showShallowImpact {
                cell("Account", width: 64, header: true, align: .trailing)
                cell("Goal", width: 56, header: true, align: .trailing)
            }
        }
        .padding(.horizontal, 14)
        .frame(height: 30)
        .background(Color(white: 0.08))
    }

    private func tableRow(_ row: TodayTradeRowPresentation) -> some View {
        HStack {
            cell(row.timeText, width: 52)
            cell(row.symbol, flex: true, mono: true)
            cell(row.sideText, width: 56, mono: true)
            cell(row.holdText, width: 56, mono: true)
            cell(row.pnlText, width: 88, align: .trailing, mono: true, tone: row.pnlTone)
            cell(row.flagText, width: 92, align: .trailing, tone: row.isFlagged ? .loss : .neutral)
            if viewModel.presentation.showShallowImpact {
                cell(row.accountShareText, width: 64, align: .trailing, mono: true, tone: .empty)
                cell(row.goalText, width: 56, align: .trailing, mono: true, tone: .empty)
            }
        }
        .padding(.horizontal, 14)
        .frame(height: 36)
        .background(row.isFlagged ? Color(red: 0.96, green: 0.27, blue: 0.36, opacity: 0.04) : .clear)
        .overlay(alignment: .bottom) {
            Rectangle().fill(StationDS.Border.divider).frame(height: 0.5)
        }
    }

    private func cell(
        _ text: String,
        width: CGFloat? = nil,
        flex: Bool = false,
        header: Bool = false,
        align: Alignment = .leading,
        mono: Bool = false,
        tone: TodayValueTone = .neutral
    ) -> some View {
        Group {
            if flex {
                Text(text)
            } else {
                Text(text).frame(width: width, alignment: align)
            }
        }
        .font(mono ? StationDS.monoFont(StationDS.FontSize.bodyXS) : StationDS.bodyFont(header ? StationDS.FontSize.bodyXS : StationDS.FontSize.bodySmall, weight: header ? .semibold : .regular))
        .foregroundStyle(header ? StationDS.Text.muted : color(for: tone))
        .frame(maxWidth: flex ? .infinity : nil, alignment: align)
    }

    private func degradedBanner(_ text: String) -> some View {
        Text(text)
            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
            .foregroundStyle(StationDS.Text.secondary)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(StationDS.Accent.amber.opacity(0.10))
            .clipShape(RoundedRectangle(cornerRadius: 12))
            .overlay(
                RoundedRectangle(cornerRadius: 12)
                    .stroke(StationDS.Accent.amber.opacity(0.28), lineWidth: 1)
            )
    }

    private var circuitBreakerBanner: some View {
        HStack(spacing: 12) {
            Text("⬡")
                .foregroundStyle(todayColor(.loss))
            VStack(alignment: .leading, spacing: 2) {
                Text("Circuit breaker active")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .semibold))
                    .foregroundStyle(todayColor(.loss))
                Text("Review behavior before resuming.")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            }
            Spacer()
            Button(viewModel.circuitBreakerResumeEnabled ? "Review & resume" : resumeCountdownLabel) {
                viewModel.resumeCircuitBreaker()
            }
            .disabled(!viewModel.circuitBreakerResumeEnabled)
            .buttonStyle(.plain)
            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS, weight: .semibold))
            .foregroundStyle(todayColor(.loss))
            .padding(.horizontal, 14)
            .padding(.vertical, 6)
            .overlay(Capsule().stroke(todayColor(.loss).opacity(0.35), lineWidth: 1))
        }
        .padding(12)
        .background(Color(red: 0.96, green: 0.27, blue: 0.36, opacity: 0.1))
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(todayColor(.loss).opacity(0.28), lineWidth: 1)
        )
    }

    private var resumeCountdownLabel: String {
        let secs = viewModel.circuitBreakerCountdownSecs
        let m = secs / 60
        let s = secs % 60
        return String(format: "Review & resume · %d:%02d", m, s)
    }

    private func color(for tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return todayColor(.profit)
        case .loss: return todayColor(.loss)
        case .empty, .neutral: return StationDS.Text.secondary
        }
    }

    private func todayColor(_ tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit)
        case .loss: return Color(hex: TodayPalette.loss)
        case .neutral: return Color(hex: TodayPalette.neutral)
        case .empty: return StationDS.Text.muted
        }
    }

    private func tileBackground(for tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit).opacity(0.08)
        case .loss: return Color(hex: TodayPalette.loss).opacity(0.1)
        default: return StationDS.Fill.input
        }
    }

    private func tileBorder(for tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit).opacity(0.22)
        case .loss: return Color(hex: TodayPalette.loss).opacity(0.28)
        default: return StationDS.Border.divider
        }
    }

    private func signalBackground(_ tone: TodaySignalTone) -> Color {
        switch tone {
        case .firing: return Color(hex: TodayPalette.loss).opacity(0.10)
        case .watch: return Color(hex: TodayPalette.watch).opacity(0.10)
        default: return StationDS.Fill.input
        }
    }

    private func signalBorder(_ tone: TodaySignalTone) -> Color {
        switch tone {
        case .firing: return Color(hex: TodayPalette.loss).opacity(0.28)
        case .watch: return Color(hex: TodayPalette.watch).opacity(0.28)
        default: return StationDS.Border.divider
        }
    }

    private func signalBadgeBackground(_ tone: TodaySignalTone) -> Color {
        switch tone {
        case .firing: return Color(hex: TodayPalette.loss).opacity(0.12)
        case .watch: return Color(hex: TodayPalette.watch).opacity(0.12)
        default: return StationDS.Fill.appPanel
        }
    }
}

/// Closed P&L series vs Settings floor. Open MTM is not drawn. Not TradingView.
struct TodayClosedFloorChart: View {
    let points: [TodayClosedChartPoint]
    let floor: Double?
    let quoteCurrency: String?
    let brokerSlug: String?
    var bookId: String? = nil

    var body: some View {
        let layout = TodayClosedFloorChartLayout.build(
            points: points,
            floor: floor,
            quoteCurrency: quoteCurrency,
            brokerSlug: brokerSlug,
            bookId: bookId,
            now: Date(),
            calendar: .current
        )
        Canvas { context, size in
            let tickBand: CGFloat = layout.showsNseSessionTicks ? 18 : 0
            let plotHeight = max(size.height - tickBand, 1)
            func x(_ unit: Double) -> CGFloat { CGFloat(unit) * size.width }
            func y(_ unit: Double) -> CGFloat { plotHeight - CGFloat(unit) * plotHeight }

            if let floorY = layout.floorY {
                var floorLine = Path()
                floorLine.move(to: CGPoint(x: 0, y: y(floorY)))
                floorLine.addLine(to: CGPoint(x: size.width, y: y(floorY)))
                context.stroke(
                    floorLine,
                    with: .color(Color(hex: TodayPalette.loss).opacity(0.55)),
                    style: StrokeStyle(lineWidth: 1, dash: [4, 3])
                )
            }
            var zeroLine = Path()
            zeroLine.move(to: CGPoint(x: 0, y: y(layout.zeroY)))
            zeroLine.addLine(to: CGPoint(x: size.width, y: y(layout.zeroY)))
            context.stroke(
                zeroLine,
                with: .color(StationDS.Text.muted.opacity(0.25)),
                style: StrokeStyle(lineWidth: 1, dash: [3, 3])
            )

            if layout.plotPoints.isEmpty {
                drawTicks(context: context, layout: layout, size: size, plotHeight: plotHeight)
                return
            }

            let fillColor: Color
            if layout.fillIsLoss == true {
                fillColor = Color(hex: TodayPalette.loss)
            } else if layout.fillIsLoss == false {
                fillColor = Color(hex: TodayPalette.profit)
            } else {
                fillColor = StationDS.Text.muted
            }

            var area = Path()
            let first = layout.plotPoints[0]
            let zero = y(layout.zeroY)
            area.move(to: CGPoint(x: x(first.x), y: zero))
            for point in layout.plotPoints {
                area.addLine(to: CGPoint(x: x(point.x), y: y(point.y)))
            }
            if let last = layout.plotPoints.last {
                area.addLine(to: CGPoint(x: x(last.x), y: zero))
            }
            area.closeSubpath()
            context.fill(area, with: .color(fillColor.opacity(0.18)))

            var line = Path()
            for (index, point) in layout.plotPoints.enumerated() {
                let pt = CGPoint(x: x(point.x), y: y(point.y))
                if index == 0 {
                    line.move(to: pt)
                } else {
                    line.addLine(to: pt)
                }
            }
            context.stroke(line, with: .color(fillColor), lineWidth: 1.5)

            if let last = layout.plotPoints.last, let pill = layout.lastPillText {
                let pt = CGPoint(x: x(last.x), y: y(last.y))
                let label = Text(pill)
                    .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
                    .foregroundColor(fillColor)
                context.draw(label, at: CGPoint(x: min(max(pt.x, 28), size.width - 28), y: max(pt.y - 10, 8)))
            }

            drawTicks(context: context, layout: layout, size: size, plotHeight: plotHeight)
        }
        .padding(10)
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 12))
        .overlay(RoundedRectangle(cornerRadius: 12).stroke(StationDS.Border.divider, lineWidth: 1))
        .accessibilityLabel("Closed P and L versus floor")
    }

    private func drawTicks(
        context: GraphicsContext,
        layout: TodayClosedFloorChartLayout,
        size: CGSize,
        plotHeight: CGFloat
    ) {
        guard layout.showsNseSessionTicks else { return }
        for (label, unitX) in zip(layout.tickLabels, layout.tickXs) {
            let tick = Text(label)
                .font(StationDS.monoFont(9))
                .foregroundColor(StationDS.Text.muted)
            context.draw(
                tick,
                at: CGPoint(x: CGFloat(unitX) * size.width, y: min(plotHeight + 10, size.height - 2))
            )
        }
    }
}

struct ThisTradeInspectorSheet: View {
    let model: ThisTradeInspectorModel
    var onClose: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Text("This trade")
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                Spacer()
                Button("Close", action: onClose)
                    .buttonStyle(.plain)
            }
            Text("\(model.symbol) · \(model.sideText) · \(model.qtyText)")
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
            labeled("Plan stop (loss)", model.planStopText)
            labeled("Live stop (loss)", model.liveStopText)
            labeled("Pulse MTM", model.pulseMTMText)
            Text(model.equityIfSLCaption)
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(StationDS.Text.secondary)
            Text(model.journalStub)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            Spacer()
        }
        .padding(20)
        .frame(minWidth: 360, minHeight: 280)
    }

    private func labeled(_ title: String, _ value: String) -> some View {
        HStack {
            Text(title)
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(StationDS.Text.muted)
            Spacer()
            Text(value)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS, weight: .semibold))
        }
    }
}
