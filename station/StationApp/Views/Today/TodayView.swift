import Notch
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
        .background(Color.black)
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
                .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .semibold))
                .foregroundStyle(BarDS.Text.primary)
            Text(viewModel.presentation.subtitle)
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS))
                .foregroundStyle(BarDS.Text.muted)
            Spacer()
        }
        .padding(.horizontal, 16)
        .frame(height: 44)
        .overlay(alignment: .bottom) {
            Rectangle().fill(BarDS.Border.divider).frame(height: BarDS.borderThin)
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
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
                .foregroundStyle(BarDS.Text.muted)
            Text(tile.value)
                .font(BarDS.bodyFont(28, weight: .bold))
                .foregroundStyle(color(for: tile.tone))
            Text(tile.caption)
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS))
                .foregroundStyle(BarDS.Text.muted)
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
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(BarDS.Text.secondary)
                Spacer()
                Text(viewModel.presentation.signalsMeta)
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS))
                    .foregroundStyle(BarDS.Text.muted)
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
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .semibold))
                .foregroundStyle(BarDS.Text.muted)
            Text("Signals unavailable")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall))
                .foregroundStyle(BarDS.Text.secondary)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(BarDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(BarDS.Border.divider, lineWidth: 1)
        )
    }

    private func signalCard(_ signal: TodaySignalCardPresentation) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(signal.name)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                Spacer()
                Text(signal.severity)
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .semibold))
                    .padding(.horizontal, 8)
                    .padding(.vertical, 3)
                    .background(signalBadgeBackground(signal.tone))
                    .clipShape(Capsule())
            }
            Text(signal.description)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS))
                .foregroundStyle(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(14)
        .background(BarDS.Fill.input)
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
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .semibold))
                    .foregroundStyle(BarDS.Text.secondary)
                Spacer()
                if !viewModel.presentation.tradesMeta.isEmpty {
                    Text(viewModel.presentation.tradesMeta)
                        .font(BarDS.monoFont(BarDS.FontSize.bodyXS))
                        .foregroundStyle(BarDS.Text.muted)
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
                .foregroundStyle(BarDS.Text.muted)
            Text("No closed trades yet today")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .semibold))
                .foregroundStyle(BarDS.Text.secondary)
            Text("Round-trips appear after a buy→sell cycle completes.")
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS))
                .foregroundStyle(BarDS.Text.muted)
                .multilineTextAlignment(.center)
        }
        .frame(maxWidth: .infinity)
        .padding(.vertical, 48)
        .background(BarDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(
            RoundedRectangle(cornerRadius: 16)
                .stroke(BarDS.Border.divider, style: StrokeStyle(lineWidth: 1, dash: [6]))
        )
    }

    private var tradesTable: some View {
        VStack(spacing: 0) {
            tableHeader
            ForEach(viewModel.presentation.trades) { row in
                tableRow(row)
            }
        }
        .background(BarDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: 16))
        .overlay(RoundedRectangle(cornerRadius: 16).stroke(BarDS.Border.divider, lineWidth: 1))
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
            Rectangle().fill(BarDS.Border.divider).frame(height: 0.5)
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
        .font(mono ? BarDS.monoFont(BarDS.FontSize.bodyXS) : BarDS.bodyFont(header ? BarDS.FontSize.bodyXS : BarDS.FontSize.bodySmall, weight: header ? .semibold : .regular))
        .foregroundStyle(header ? BarDS.Text.muted : color(for: tone))
        .frame(maxWidth: flex ? .infinity : nil, alignment: align)
    }

    private func degradedBanner(_ text: String) -> some View {
        Text(text)
            .font(BarDS.bodyFont(BarDS.FontSize.bodySmall))
            .foregroundStyle(BarDS.Text.secondary)
            .padding(12)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(BarDS.Fill.input)
            .clipShape(RoundedRectangle(cornerRadius: 12))
    }

    private var circuitBreakerBanner: some View {
        HStack(spacing: 12) {
            Text("⬡")
                .foregroundStyle(todayColor(.loss))
            VStack(alignment: .leading, spacing: 2) {
                Text("Circuit breaker active")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                    .foregroundStyle(todayColor(.loss))
                Text("Review behavior before resuming.")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS))
                    .foregroundStyle(BarDS.Text.muted)
            }
            Spacer()
            Button(viewModel.circuitBreakerResumeEnabled ? "Review & resume" : resumeCountdownLabel) {
                viewModel.resumeCircuitBreaker()
            }
            .disabled(!viewModel.circuitBreakerResumeEnabled)
            .buttonStyle(.plain)
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
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
        case .empty, .neutral: return BarDS.Text.secondary
        }
    }

    private func todayColor(_ tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit)
        case .loss: return Color(hex: TodayPalette.loss)
        case .neutral: return Color(hex: TodayPalette.neutral)
        case .empty: return BarDS.Text.muted
        }
    }

    private func tileBackground(for tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit).opacity(0.08)
        case .loss: return Color(hex: TodayPalette.loss).opacity(0.1)
        default: return BarDS.Fill.input
        }
    }

    private func tileBorder(for tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return Color(hex: TodayPalette.profit).opacity(0.22)
        case .loss: return Color(hex: TodayPalette.loss).opacity(0.28)
        default: return BarDS.Border.divider
        }
    }

    private func signalBorder(_ tone: TodaySignalTone) -> Color {
        switch tone {
        case .firing: return Color(hex: TodayPalette.loss).opacity(0.28)
        case .watch: return Color(hex: TodayPalette.watch).opacity(0.28)
        default: return BarDS.Border.divider
        }
    }

    private func signalBadgeBackground(_ tone: TodaySignalTone) -> Color {
        switch tone {
        case .firing: return Color(hex: TodayPalette.loss).opacity(0.12)
        case .watch: return Color(hex: TodayPalette.watch).opacity(0.12)
        default: return BarDS.Fill.appPanel
        }
    }
}

private extension Color {
    init(hex: String) {
        let hex = hex.trimmingCharacters(in: CharacterSet.alphanumerics.inverted)
        var int: UInt64 = 0
        Scanner(string: hex).scanHexInt64(&int)
        let r = Double((int >> 16) & 0xFF) / 255
        let g = Double((int >> 8) & 0xFF) / 255
        let b = Double(int & 0xFF) / 255
        self.init(red: r, green: g, blue: b)
    }
}
