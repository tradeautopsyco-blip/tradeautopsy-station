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
                    .font(StationDS.bodyFont(StationDS.FontSize.brief, weight: .semibold))
                    .foregroundStyle(StationDS.Text.primary)
                Text("Every declaration that hit Console · local week, then the hosted sheet")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
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
                Text("Post due").tag(JournalFacet.postDue)
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
        let due = viewModel.week.declarations.filter(\.isPostDue).count
        let unmatched = viewModel.week.declarations.filter(\.isUnmatched).count
        let pending = viewModel.week.declarations.filter(\.isPending).count
        return Text(
            "Capture is still Notch. This page is the frozen plan — matched, pending, expired, cancelled — plus pre / live / post. "
            + (due > 0 ? "\(due) post due. " : "Posts saved. ")
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
            Text(metaLine(card))
                .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                .foregroundStyle(StationDS.Text.muted)
            HStack(spacing: 6) {
                Text(card.symbol)
                    .font(StationDS.bodyFont(StationDS.FontSize.body, weight: .medium))
                Text("\(card.side) \(qtyText(card.quantity))")
                    .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                    .foregroundStyle(StationDS.Text.secondary)
            }
            Text(snapLine(card))
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
            Text("No frozen plan → no fidelity. Plan in Notch.")
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
                    labeled("Kind", kindLabel(card.declarationKind))
                    labeled("Setup", card.snapshot.setupLabel ?? "—")
                    labeled("Calm", calmLine(card.snapshot.calmScale))
                    labeled("Confidence", confLine(card.snapshot.confidenceScale))
                    labeled("SL", numberText(card.snapshot.stopLoss))
                    labeled("Target", numberText(card.snapshot.target))
                    labeled("Invalidation", card.snapshot.invalidationLine ?? "—")
                    labeled("SL consent", card.protectiveSlConsent ? "Yes" : "No")
                    labeled("Qty declared", qtyText(card.quantity))
                    labeled("Qty filled", card.quantityFilled.map(qtyText) ?? "—")
                    if let net = card.citedNet, let ccy = card.citedCurrency {
                        labeled("Matched net", "\(net) \(ccy)")
                    }
                    if let dims = card.fidelity.dimensions {
                        Text("Fidelity")
                            .font(StationDS.bodyFont(StationDS.FontSize.bodyXS))
                            .foregroundStyle(StationDS.Text.muted)
                        HStack {
                            fidChip("Entry", dims.entry)
                            fidChip("Stop", dims.stop)
                            fidChip("Target", dims.target)
                            fidChip("Size", dims.size)
                            fidChip("Inv", dims.inv)
                        }
                    }
                    labeled("Pre", card.notes.pre.isEmpty ? "—" : card.notes.pre)
                    labeled("Live", card.notes.live.isEmpty ? "—" : card.notes.live)
                    labeled("Post", card.notes.post.isEmpty ? "Due" : card.notes.post)
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
            pill(statusLabel(card.status), tone: statusTone(card.status))
            if card.isMatched {
                pill("Pre", tone: card.notes.pre.isEmpty ? .due : .ok)
                pill("Live", tone: card.notes.live.isEmpty ? .due : .ok)
                pill("Post", tone: card.notes.postIsEmpty ? .due : .ok)
            }
            if card.attachments.shots > 0 {
                pill("\(card.attachments.shots) shot\(card.attachments.shots == 1 ? "" : "s")", tone: .ok)
            }
            if card.attachments.voice {
                pill("Voice", tone: .ok)
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

    private func fidChip(_ label: String, _ ok: Bool) -> some View {
        pill(label, tone: ok ? .ok : .bad)
    }

    private func labeled(_ k: String, _ v: String) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(k).font(StationDS.bodyFont(StationDS.FontSize.bodyXS)).foregroundStyle(StationDS.Text.muted)
            Text(v).font(StationDS.bodyFont(StationDS.FontSize.bodySmall))
        }
    }

    private func metaLine(_ card: JournalDeclarationCard) -> String {
        "\(kindLabel(card.declarationKind)) · calm \(calmLine(card.snapshot.calmScale)) · conf \(confLine(card.snapshot.confidenceScale))"
    }

    private func snapLine(_ card: JournalDeclarationCard) -> String {
        let setup = card.snapshot.setupLabel ?? "—"
        let sl = numberText(card.snapshot.stopLoss)
        let inv = card.snapshot.invalidationKind ?? "—"
        var s = "\(setup) · SL \(sl) · invalidation \(inv)"
        if let score = card.fidelity.score {
            s += " · fidelity \(Int(score.rounded()))"
        }
        return s
    }

    private func kindLabel(_ k: String) -> String {
        switch k {
        case "intraday": return "Intraday"
        case "swing": return "Swing"
        case "positional": return "Positional"
        case "scalper_session": return "Scalper"
        case "pre_market": return "Pre-market"
        default: return k
        }
    }

    private func statusLabel(_ s: String) -> String {
        switch s {
        case "matched": return "Matched"
        case "pending": return "Pending"
        case "expired": return "Expired"
        case "cancelled": return "Cancelled"
        default: return s
        }
    }

    private func statusTone(_ s: String) -> PillTone {
        switch s {
        case "matched": return .ok
        case "pending": return .warn
        default: return .due
        }
    }

    private func calmLine(_ n: Double?) -> String {
        guard let n else { return "—" }
        let i = Int(n.rounded())
        let word = ["", "Calm", "Focused", "Tense", "Anxious", "Angry"][safe: i] ?? ""
        return word.isEmpty ? "\(i)" : "\(i) \(word)"
    }

    private func confLine(_ n: Double?) -> String {
        guard let n else { return "—" }
        return "\(Int(n.rounded()))"
    }

    private func numberText(_ n: Double?) -> String {
        guard let n else { return "—" }
        return String(format: n == n.rounded() ? "%.0f" : "%.2f", n)
    }

    private func qtyText(_ n: Double) -> String {
        n == n.rounded() ? String(Int(n)) : String(n)
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

private extension Array where Element == String {
    subscript(safe index: Int) -> String? {
        indices.contains(index) ? self[index] : nil
    }
}
