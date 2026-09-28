import AppKit
import SwiftUI

public struct BookAutopsyReportView: View {
    @ObservedObject private var today: TodayViewModel
    @ObservedObject private var brokers: BrokersViewModel
    @Binding private var selectedSymbol: String?
    @Binding private var selectedSearch: BookAutopsySearch?
    private let onOpenJournal: () -> Void
    private let onOpenNotch: () -> Void

    public init(
        today: TodayViewModel,
        brokers: BrokersViewModel,
        selectedSymbol: Binding<String?>,
        selectedSearch: Binding<BookAutopsySearch?>,
        onOpenJournal: @escaping () -> Void,
        onOpenNotch: @escaping () -> Void
    ) {
        self.today = today
        self.brokers = brokers
        self._selectedSymbol = selectedSymbol
        self._selectedSearch = selectedSearch
        self.onOpenJournal = onOpenJournal
        self.onOpenNotch = onOpenNotch
    }

    private var screen: TodayScreenPresentation { today.presentation }

    private var presentation: BookAutopsyPresentation {
        BookAutopsyPresentation.build(
            screen: screen,
            brokerSlug: today.desk.brokerSlug,
            configuredBrokerSlugs: brokers.configuredBrokerSlugs,
            search: selectedSearch
        )
    }

    private var peek: BookAutopsyPeek? {
        guard let selectedSymbol else { return nil }
        return presentation.peek(for: selectedSymbol, in: screen)
    }

    public var body: some View {
        ZStack(alignment: .topLeading) {
            ScrollView {
                VStack(alignment: .leading, spacing: 0) {
                    breadcrumb
                    Text("Book autopsy")
                        .font(StationDS.bodyFont(StationDS.FontSize.metricValue + 4, weight: .semibold))
                        .foregroundStyle(WorkspaceChrome.text)
                        .padding(.top, 8)
                    if let search = presentation.search {
                        searchChip(search)
                            .padding(.top, 10)
                    }
                    cards
                        .padding(.top, 16)
                    chart
                        .padding(.top, 20)
                }
                .padding(.horizontal, 24)
                .padding(.top, 20)
                .padding(.bottom, 24)
                .frame(maxWidth: 720, alignment: .leading)
            }

            if let peek {
                peekCard(peek)
                    .padding(.leading, 24)
                    .padding(.top, 168)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(WorkspaceChrome.ground)
        .accessibilityIdentifier("workspace-report")
        .onExitCommand {
            guard selectedSymbol != nil else { return }
            selectedSymbol = nil
        }
    }

    private var breadcrumb: some View {
        HStack(spacing: 4) {
            Text("Report")
                .foregroundStyle(WorkspaceChrome.muted)
            Text("/")
                .foregroundStyle(WorkspaceChrome.faint)
            Text("Book autopsy")
                .foregroundStyle(StationDS.Text.section)
        }
        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
    }

    private func searchChip(_ search: BookAutopsySearch) -> some View {
        HStack(spacing: 8) {
            Text("Search · \(search.rawValue)")
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(WorkspaceChrome.muted)
            Button("Clear") { selectedSearch = nil }
                .buttonStyle(.plain)
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(WorkspaceChrome.accent)
        }
    }

    @ViewBuilder
    private var cards: some View {
        if presentation.cards.isEmpty {
            Text("No connected broker")
                .font(StationDS.bodyFont(StationDS.FontSize.body))
                .foregroundStyle(WorkspaceChrome.muted)
        } else {
            HStack(spacing: 12) {
                ForEach(presentation.cards) { card in
                    VStack(alignment: .leading, spacing: 6) {
                        Text(card.name)
                            .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                            .foregroundStyle(WorkspaceChrome.muted)
                        Text(card.value)
                            .font(WorkspaceChrome.metricFont())
                            .foregroundStyle(WorkspaceChrome.text)
                        Text(card.caption)
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(WorkspaceChrome.faint)
                    }
                    .padding(14)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(StationDS.Fill.card, in: RoundedRectangle(cornerRadius: StationDS.Radius.card))
                    .overlay(
                        RoundedRectangle(cornerRadius: StationDS.Radius.card)
                            .stroke(StationDS.Border.card, lineWidth: StationDS.borderThin)
                    )
                }
            }
        }
    }

    private var chart: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Closed trades by hour")
                .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                .foregroundStyle(WorkspaceChrome.muted)
            if let caption = presentation.chartCaption {
                Text(caption)
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                    .foregroundStyle(WorkspaceChrome.faint)
            }
            HStack(alignment: .bottom, spacing: 6) {
                if presentation.bars.isEmpty {
                    Color.clear.frame(height: 96)
                } else {
                    let maxCount = presentation.bars.map(\.count).max() ?? 1
                    ForEach(presentation.bars) { bar in
                        VStack(spacing: 4) {
                            RoundedRectangle(cornerRadius: 2)
                                .fill(WorkspaceChrome.accent.opacity(0.45))
                                .frame(height: max(6, 96 * CGFloat(bar.count) / CGFloat(maxCount)))
                            Text("\(bar.hour)")
                                .font(StationDS.monoFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(WorkspaceChrome.faint)
                        }
                        .frame(maxWidth: .infinity, alignment: .bottom)
                    }
                }
            }
            .frame(maxWidth: .infinity, minHeight: 108, alignment: .bottom)
        }
        .contentShape(Rectangle())
        .onTapGesture { selectedSymbol = nil }
        .accessibilityIdentifier("workspace-chart")
    }

    private func peekCard(_ peek: BookAutopsyPeek) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack(alignment: .top) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(peek.symbol)
                        .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                        .foregroundStyle(WorkspaceChrome.text)
                    Text(peek.subtitle)
                        .font(StationDS.monoFont(StationDS.FontSize.bodySmall))
                        .foregroundStyle(WorkspaceChrome.muted)
                }
                Spacer(minLength: 8)
                HStack(spacing: 4) {
                    peekButton("book", help: "Open journal", identifier: "workspace-peek-journal", action: onOpenJournal)
                    peekButton("capsule", help: "Open Notch", identifier: "workspace-peek-notch", action: onOpenNotch)
                    peekButton("link", help: "Copy symbol", identifier: "workspace-peek-copy") { copy(peek.symbol) }
                }
            }
            VStack(alignment: .leading, spacing: 6) {
                ForEach(peek.facts) { fact in
                    HStack(alignment: .firstTextBaseline, spacing: 12) {
                        Text(fact.label)
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(WorkspaceChrome.faint)
                            .frame(width: 44, alignment: .leading)
                        Text(fact.value)
                            .font(StationDS.bodyFont(StationDS.FontSize.body))
                            .foregroundStyle(WorkspaceChrome.text)
                    }
                }
            }
            Divider().overlay(WorkspaceChrome.line)
            HStack {
                Text("P&L")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
                    .foregroundStyle(WorkspaceChrome.muted)
                Spacer()
                Text(peek.valueText)
                    .font(StationDS.monoFont(StationDS.FontSize.body, weight: .medium))
                    .foregroundStyle(toneColor(peek.tone))
            }
        }
        .padding(14)
        .frame(width: 360, alignment: .leading)
        .background(WorkspaceChrome.card, in: RoundedRectangle(cornerRadius: StationDS.Radius.card + 4))
        .overlay(
            RoundedRectangle(cornerRadius: StationDS.Radius.card + 4)
                .stroke(StationDS.Border.card, lineWidth: StationDS.borderThin)
        )
        .shadow(color: .black.opacity(0.25), radius: 12, y: 6)
        .accessibilityIdentifier("workspace-peek")
    }

    private func peekButton(
        _ systemName: String,
        help: String,
        identifier: String,
        action: @escaping () -> Void
    ) -> some View {
        Button(action: action) {
            Image(systemName: systemName)
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(WorkspaceChrome.muted)
                .frame(width: 26, height: 26)
                .background(StationDS.Fill.input, in: Circle())
        }
        .buttonStyle(.plain)
        .help(help)
        .accessibilityIdentifier(identifier)
    }

    private func copy(_ symbol: String) {
        let board = NSPasteboard.general
        board.clearContents()
        board.setString(symbol, forType: .string)
    }

    private func toneColor(_ tone: TodayValueTone) -> Color {
        switch tone {
        case .profit: return WorkspaceChrome.profit
        case .loss: return WorkspaceChrome.loss
        case .neutral, .empty: return WorkspaceChrome.text
        }
    }
}
