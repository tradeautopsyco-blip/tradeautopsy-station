import SwiftUI
import AppKit

public struct JournalView: View {
    @ObservedObject private var viewModel: JournalViewModel
    private let onOpenNotch: () -> Void
    private let openURL: (URL) -> Void

    public init(
        viewModel: JournalViewModel,
        onOpenNotch: @escaping () -> Void,
        openURL: @escaping (URL) -> Void = { url in
            NSWorkspace.shared.open(url)
        }
    ) {
        self.viewModel = viewModel
        self.onOpenNotch = onOpenNotch
        self.openURL = openURL
    }

    public var body: some View {
        VStack(spacing: 0) {
            header
            tools
            Divider().overlay(StationDS.Border.divider)
            HStack(alignment: .top, spacing: 0) {
                weekRail
                    .frame(width: 220)
                Divider().overlay(StationDS.Border.divider)
                dayColumn
                if viewModel.selectedSheet || viewModel.selectedCard != nil {
                    Divider().overlay(StationDS.Border.divider)
                    drawer
                        .frame(width: 280)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(StationDS.Fill.appPanel)
        .task { await viewModel.load() }
    }

    private var header: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text("Journal")
                    .font(DeskChrome.sans(DeskChrome.TypeScale.largeTitle, weight: .bold))
                    .foregroundStyle(StationDS.Text.primary)
                Text("Every declaration that hit Console · local week, then the hosted sheet")
                    .font(DeskChrome.sans(DeskChrome.TypeScale.callout))
                    .foregroundStyle(StationDS.Text.muted)
            }
            Spacer()
            Button("Make another") { onOpenNotch() }
                .buttonStyle(.borderedProminent)
                .tint(StationDS.Accent.teal)
                .controlSize(.small)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
    }

    private var tools: some View {
        HStack(spacing: 10) {
            TextField("Search symbol, setup, emotion…", text: $viewModel.query)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 280)
            Picker("Facet", selection: $viewModel.facet) {
                Text("All").tag(JournalFacet.all)
                Text("Matched").tag(JournalFacet.matched)
                Text("Pending").tag(JournalFacet.pending)
                Text("Unmatched").tag(JournalFacet.unmatched)
                Text("Impulsive").tag(JournalFacet.impulsive)
            }
            .pickerStyle(.segmented)
            .labelsHidden()
            Button("Export") {
                if let url = viewModel.exportURL {
                    openURL(url)
                }
            }
            .disabled(viewModel.exportURL == nil)
            .controlSize(.small)
        }
        .padding(.horizontal, 16)
        .padding(.bottom, 10)
    }

    private var weekRail: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("This week")
                .font(StationDS.bodyFont(StationDS.FontSize.sectionLabel, weight: .semibold))
                .foregroundStyle(StationDS.Text.section)
                .padding(.horizontal, 12)
            if viewModel.week.days.isEmpty {
                Text("Older days live in Console.")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
                    .padding(.horizontal, 12)
            } else {
                ForEach(viewModel.week.days) { day in
                    Button {
                        viewModel.selectedDay = day.localDate
                        viewModel.selectedCardId = nil
                        viewModel.selectedSheet = false
                    } label: {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(dayTitle(day.localDate))
                                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                            Text(day.sheet?.emotion.isEmpty == false ? day.sheet!.emotion : "Declarations")
                                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                                .foregroundStyle(StationDS.Text.muted)
                        }
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(10)
                        .background(
                            viewModel.selectedDay == day.localDate
                                ? StationDS.Fill.elevated
                                : Color.clear
                        )
                        .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                    }
                    .buttonStyle(.plain)
                    .padding(.horizontal, 8)
                }
            }
            Text("Older days live in Console. Query by emotion and setup is the same search.")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
                .padding(12)
            Spacer()
        }
        .padding(.top, 12)
    }

    private var dayColumn: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 18) {
                takeaway
                daySheetBlock
                declarationsBlock
                if !viewModel.week.impulsive.isEmpty {
                    impulsiveBlock
                }
            }
            .padding(16)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var takeaway: some View {
        let unmatched = viewModel.week.declarations.filter(\.isUnmatched).count
        let pending = viewModel.week.declarations.filter(\.isPending).count
        return Text(
            "Capture is still Notch. This page is the frozen plan — matched, pending, expired, cancelled. "
            + (unmatched > 0 ? "\(unmatched) never filled. " : "")
            + (pending > 0 ? "\(pending) still pending." : "")
        )
        .font(StationDS.bodyFont(StationDS.FontSize.bodySmall, weight: .medium))
        .foregroundStyle(StationDS.Text.secondary)
    }

    private var daySheetBlock: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Day sheet")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            Button {
                viewModel.selectedSheet = true
                viewModel.selectedCardId = nil
            } label: {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Notebook pin · feeling \(viewModel.week.sheet?.emotion.isEmpty == false ? viewModel.week.sheet!.emotion : "—")")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                    Text(viewModel.week.sheet?.body.isEmpty == false ? viewModel.week.sheet!.body : "No pin on Console for this local day.")
                        .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                        .multilineTextAlignment(.leading)
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(12)
                .background(StationDS.Fill.card)
                .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.card))
            }
            .buttonStyle(.plain)
        }
    }

    private var declarationsBlock: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Declarations that went to Console")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            if viewModel.week.declarations.isEmpty {
                Text("Nothing in this filter. Declarations still live on Console. This is the local week.")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.muted)
            } else {
                ForEach(viewModel.week.declarations) { card in
                    Button {
                        viewModel.selectedCardId = card.id
                        viewModel.selectedSheet = false
                    } label: {
                        declarationRow(card)
                    }
                    .buttonStyle(.plain)
                }
            }
            Text("A changed plan is a new declaration. Cancelled-before-fill and expired unmatched stay on the sheet — they already went to Console.")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
        }
    }

    private func declarationRow(_ card: JournalDeclarationCard) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text("\(JournalCardPaint.kindLabel(card.declarationKind)) · calm \(JournalCardPaint.calmLine(card.snapshot.calmScale)) · conf \(JournalCardPaint.confLine(card.snapshot.confidenceScale))")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            HStack(spacing: 6) {
                Text(card.symbol)
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                Text("\(card.side) \(JournalCardPaint.qtyText(card.quantity))")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.secondary)
            }
            Text(JournalCardPaint.snapLine(card))
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            chips(card)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(12)
        .background(viewModel.selectedCardId == card.id ? StationDS.Fill.elevated : StationDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.card))
    }

    private var impulsiveBlock: some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Impulsive · no declaration")
                .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .semibold))
                .foregroundStyle(StationDS.Text.secondary)
            ForEach(viewModel.week.impulsive) { row in
                VStack(alignment: .leading, spacing: 4) {
                    Text("Still open · no declarationId")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                    Text(row.symbol)
                        .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                    HStack {
                        pill("Impulsive", tone: .bad)
                        Button("Plan in Notch") { onOpenNotch() }
                            .controlSize(.mini)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(12)
                .background(StationDS.Fill.card)
                .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.card))
            }
            Text("Plan in Notch.")
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
        }
    }

    private var drawer: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                if viewModel.selectedSheet {
                    Text("Day sheet")
                        .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                    Text("Feeling \(viewModel.week.sheet?.emotion ?? "—")")
                        .foregroundStyle(StationDS.Text.muted)
                    Text(viewModel.week.sheet?.body ?? "No pin.")
                    Text("Shadow journal and weekly review live in Console — Station keeps the local week.")
                        .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                        .foregroundStyle(StationDS.Text.muted)
                } else if let card = viewModel.selectedCard {
                    Text(card.symbol)
                        .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                    chips(card)
                    ForEach(JournalCardPaint.drawerRows(card), id: \.label) { row in
                        labeled(row.label, row.value)
                    }
                }
                Button("Close") {
                    viewModel.selectedCardId = nil
                    viewModel.selectedSheet = false
                }
                .controlSize(.small)
            }
            .padding(16)
            .foregroundStyle(StationDS.Text.primary)
        }
        .background(StationDS.Fill.elevated)
    }

    private func chips(_ card: JournalDeclarationCard) -> some View {
        HStack(spacing: 6) {
            ForEach(JournalCardPaint.statusChips(card), id: \.self) { text in
                pill(text, tone: statusTone(card.status))
            }
        }
    }

    private enum PillTone { case ok, warn, due, bad }

    private func pill(_ text: String, tone: PillTone) -> some View {
        let color: Color = {
            switch tone {
            case .ok: return StationDS.Accent.green
            case .warn: return StationDS.Accent.amber
            case .due: return StationDS.Accent.amber
            case .bad: return StationDS.Accent.red
            }
        }()
        return Text(text)
            .font(StationDS.bodyFont(StationDS.FontSize.chip, weight: .medium))
            .foregroundStyle(color)
            .padding(.horizontal, 8)
            .padding(.vertical, 3)
            .background(color.opacity(0.12))
            .clipShape(Capsule())
    }

    private func labeled(_ k: String, _ v: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(k).font(StationDS.bodyFont(StationDS.FontSize.bodyXS)).foregroundStyle(StationDS.Text.muted)
            Text(v).font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
        }
    }

    private func statusTone(_ s: String) -> PillTone {
        switch s {
        case "matched": return .ok
        case "pending": return .warn
        default: return .due
        }
    }

    private func dayTitle(_ iso: String) -> String {
        let f = DateFormatter()
        f.calendar = Calendar(identifier: .gregorian)
        f.locale = Locale(identifier: "en_IN")
        f.timeZone = TimeZone(identifier: "Asia/Kolkata")
        f.dateFormat = "yyyy-MM-dd"
        guard let d = f.date(from: iso) else { return iso }
        f.dateFormat = "EEE d MMM"
        return f.string(from: d)
    }
}
