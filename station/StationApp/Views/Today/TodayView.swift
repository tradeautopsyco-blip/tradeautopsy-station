import SwiftUI

public struct TodayView: View {
    @ObservedObject private var viewModel: TodayViewModel

    public init(viewModel: TodayViewModel) {
        self.viewModel = viewModel
    }

    public var body: some View {
        VStack(spacing: 0) {
            header
            ScrollView {
                VStack(alignment: .leading, spacing: 20) {
                    if viewModel.presentation.showDegradedBanner,
                       let text = viewModel.presentation.degradedBannerText {
                        degradedBanner(text)
                    }
                    if viewModel.showCircuitBreakerBanner {
                        circuitBreakerBanner
                    }
                    heroGrid
                    signalsSection
                    tradesSection
                }
                .padding(16)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .task {
            await viewModel.load()
        }
        .refreshable {
            await viewModel.load()
        }
    }

    private var header: some View {
        HStack {
            Text("Today")
                .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                .foregroundStyle(StationDS.Text.primary)
            Text(viewModel.presentation.subtitle)
                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            Spacer()
        }
        .padding(.horizontal, 16)
        .frame(height: 44)
        .overlay(alignment: .bottom) {
            Rectangle().fill(StationDS.Border.divider).frame(height: StationDS.borderThin)
        }
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
        .background(StationDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(signalBorder(signal.tone), lineWidth: 1)
        )
    }

    private var tradesSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text("Trades today")
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
            cell("Entry", width: 76, header: true, align: .trailing)
            cell("Exit", width: 76, header: true, align: .trailing)
            cell("Net P&L", width: 88, header: true, align: .trailing)
            cell("Flag", width: 92, header: true, align: .trailing)
        }
        .padding(.horizontal, 14)
        .frame(height: 30)
        .background(Color(white: 0.08))
    }

    private func tableRow(_ row: TodayTradeRowPresentation) -> some View {
        HStack {
            cell(row.timeText, width: 52)
            cell(row.symbol, flex: true, mono: true)
            cell(row.avgEntryText, width: 76, align: .trailing, mono: true)
            cell(row.avgExitText, width: 76, align: .trailing, mono: true)
            cell(row.pnlText, width: 88, align: .trailing, mono: true, tone: row.pnlTone)
            cell(row.flagText, width: 92, align: .trailing, tone: row.isFlagged ? .loss : .neutral)
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
            .background(StationDS.Fill.input)
            .clipShape(RoundedRectangle(cornerRadius: 12))
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
