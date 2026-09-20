import SwiftUI

/// Declared vs actual escrow reconciliation (Notch reads server tree only — #125).
/// Layout: dual columns + center connectors, seven fixed rows, status ring (spec: 56pt).
struct BarEscrowMatchView: View {
    let report: BarEscrowMatchReport?
    var pending: BarPendingDeclaration? = nil

    @State private var expandedRowIds: Set<String> = []

    private let escrowRingDiameter: CGFloat = 56

    var body: some View {
        let rows = BarEscrowMatchPresentation.matchRows(from: report, pending: pending)
        let summary = BarEscrowMatchPresentation.summaryLine(from: report)

        VStack(alignment: .leading, spacing: 12) {
            headerRow(summary: summary)

            ForEach(rows) { row in
                escrowLedgerRow(row)
            }
            if rows.isEmpty {
                Text("No escrow match yet.")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
            }
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func headerRow(summary: String?) -> some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 4) {
                Text("MATCH — declared vs actual")
                    .font(BarDS.bodyFont(10, weight: .bold))
                    .foregroundColor(BarDS.Text.hint)
                    .tracking(0.8)
                if let summary, !summary.isEmpty {
                    Text(summary)
                        .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .semibold))
                        .foregroundColor(BarDS.Accent.teal.opacity(0.88))
                        .fixedSize(horizontal: false, vertical: true)
                        .accessibilityLabel(summary)
                } else {
                    Text("No escrow summary on this refresh.")
                        .font(BarDS.bodyFont(10, weight: .medium))
                        .foregroundColor(BarDS.Text.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)

            BarEscrowMatchNode(
                nodeKey: "Fidelity",
                nodeValue: Self.ringCenterLabel(for: report),
                state: centerNodeState(for: report),
            )
            .frame(minWidth: 88, maxWidth: 120)

            escrowStatusRing
        }
    }

    private func centerNodeState(for report: BarEscrowMatchReport?) -> BarEscrowMatchNode.NodeState {
        switch Self.ringTone(for: report) {
        case .green: return .ok
        case .amber: return .wrn
        case .red: return .brk
        }
    }

    private var escrowStatusRing: some View {
        let tone = BarEscrowMatchView.ringTone(for: report)
        let label = BarEscrowMatchView.ringCenterLabel(for: report)
        let d = escrowRingDiameter
        return ZStack {
            Circle()
                .stroke(toneColor(tone).opacity(0.35), lineWidth: 3)
                .frame(width: d, height: d)
            Circle()
                .trim(from: 0, to: ringProgress(for: report))
                .stroke(toneColor(tone), style: StrokeStyle(lineWidth: 4, lineCap: .round))
                .rotationEffect(.degrees(-90))
                .frame(width: d - 4, height: d - 4)
            Text(label)
                .font(BarDS.monoFont(12, weight: .bold))
                .foregroundColor(BarDS.Text.primary)
                .minimumScaleFactor(0.5)
                .lineLimit(2)
                .multilineTextAlignment(.center)
                .frame(width: d - 18)
        }
        .accessibilityElement(children: .ignore)
        .accessibilityLabel("Escrow status \(label)")
    }

    private func ringProgress(for report: BarEscrowMatchReport?) -> CGFloat {
        guard let p = report?.fidelityPct, p.isFinite, p >= 0 else { return 0.35 }
        return CGFloat(min(1, max(0, p / 100)))
    }

    static func ringCenterLabel(for report: BarEscrowMatchReport?) -> String {
        guard let p = report?.fidelityPct, p.isFinite else { return "—" }
        return "\(Int(p.rounded()))%"
    }

    static func ringTone(for report: BarEscrowMatchReport?) -> BarEscrowRowTone {
        guard let p = report?.fidelityPct, p.isFinite else { return .amber }
        if p >= 80 { return .green }
        if p >= 50 { return .amber }
        return .red
    }

    private func escrowLedgerRow(_ row: BarEscrowRowPresentation) -> some View {
        let expanded = expandedRowIds.contains(row.id)
        return VStack(alignment: .leading, spacing: 6) {
            Button {
                withAnimation(.easeInOut(duration: 0.22)) {
                    if expanded {
                        expandedRowIds.remove(row.id)
                    } else {
                        expandedRowIds.insert(row.id)
                    }
                }
            } label: {
                VStack(alignment: .leading, spacing: 4) {
                    Text(row.label.uppercased())
                        .font(BarDS.bodyFont(9, weight: .bold))
                        .foregroundColor(BarDS.Text.secondary)
                    HStack(alignment: .center, spacing: 0) {
                        ledgerColumn(text: row.declared, title: "Declared", alignLeading: true)
                        connectorBar(tone: row.tone)
                        ledgerColumn(text: row.actual, title: "Actual", alignLeading: false)
                    }
                }
                .padding(.vertical, 6)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("\(row.label). Declared \(row.declared). Actual \(row.actual). \(expanded ? "Expanded" : "Collapsed"). Double tap to \(expanded ? "collapse" : "expand")")

            if expanded {
                expandedBody(for: row)
                    .transition(.opacity.combined(with: .move(edge: .top)))
            }

            BarDSDivider()
        }
    }

    @ViewBuilder
    private func expandedBody(for row: BarEscrowRowPresentation) -> some View {
        if let reason = row.breakReason, !reason.isEmpty {
            Text(reason)
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Accent.amber.opacity(0.9))
                .fixedSize(horizontal: false, vertical: true)
        } else if row.id.hasPrefix("placeholder.") {
            Text("Awaiting escrow node for this slot — Notch does not invent fills.")
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
        } else {
            Text("No break detail from server for this row.")
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
        }
    }

    private func ledgerColumn(text: String, title: String, alignLeading: Bool) -> some View {
        VStack(alignment: alignLeading ? .leading : .trailing, spacing: 2) {
            Text(title.uppercased())
                .font(BarDS.bodyFont(8, weight: .bold))
                .foregroundColor(BarDS.Text.muted)
            Text(text)
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .multilineTextAlignment(alignLeading ? .leading : .trailing)
        }
        .frame(maxWidth: .infinity, alignment: alignLeading ? .leading : .trailing)
    }

    private func connectorBar(tone: BarEscrowRowTone) -> some View {
        ZStack {
            Rectangle()
                .fill(Color.white.opacity(0.12))
                .frame(width: 2)
                .frame(height: 28)
            Circle()
                .fill(dotColor(tone))
                .frame(width: 8, height: 8)
        }
        .frame(width: 18)
        .accessibilityLabel("Match \(String(describing: tone))")
    }

    private func dotColor(_ tone: BarEscrowRowTone) -> Color {
        switch tone {
        case .green: BarDS.Accent.teal.opacity(0.9)
        case .amber: BarDS.Accent.amber.opacity(0.9)
        case .red: BarDS.Accent.red.opacity(0.9)
        }
    }

    private func toneColor(_ tone: BarEscrowRowTone) -> Color {
        dotColor(tone)
    }
}
