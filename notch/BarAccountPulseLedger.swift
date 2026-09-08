import SwiftUI

/// Prototype C: pulse (free + counts) + full-bleed obtain ledger. Throwaway HTML locked 2026-09-08.
struct BarAccountPulseLedger: View {
    let snapshot: BarAccountChrome.Snapshot

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            pulse
            counts
            ledger
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(accessibilitySummary)
    }

    private var pulse: some View {
        HStack(alignment: .firstTextBaseline) {
            VStack(alignment: .leading, spacing: 2) {
                Text(snapshot.bookLabel)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(0.04 * 10)
                Text(snapshot.freeText)
                    .font(BarDS.monoFont(20, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)
                    .kerning(-0.03 * 20)
            }
            Spacer(minLength: 8)
            fundsPill
        }
        .padding(.top, 4)
        .padding(.bottom, 10)
    }

    private var fundsPill: some View {
        let status = snapshot.fundsStatus
        let dot = DeskCapabilityChrome.dotName(forStatus: status == "success" ? "fresh" : status)
        let (fg, bg): (Color, Color) = {
            switch dot {
            case "teal":
                return (BarDS.Accent.teal, BarDS.Accent.teal.opacity(0.12))
            case "amber":
                return (BarDS.Accent.amber, BarDS.Accent.amber.opacity(0.12))
            default:
                return (BarDS.Accent.red, BarDS.Accent.red.opacity(0.12))
            }
        }()
        let label = DeskCapabilityChrome.pillLabel(
            kind: "Funds",
            status: status == "success" ? "fresh" : status
        )
        return Text(label)
            .font(BarDS.bodyFont(10, weight: .medium))
            .foregroundColor(fg)
            .padding(.vertical, 4)
            .padding(.horizontal, 8)
            .background(bg)
            .clipShape(Capsule())
    }

    private var counts: some View {
        HStack(spacing: 14) {
            Text("\(snapshot.holdingsCount) holdings")
            Text("\(snapshot.positionsCount) positions")
            Text(BarAccountChrome.ordersCountLabel(snapshot))
        }
        .font(BarDS.bodyFont(11, weight: .regular))
        .foregroundColor(BarDS.Text.secondary)
        .padding(.bottom, 12)
    }

    private var ledger: some View {
        VStack(alignment: .leading, spacing: 12) {
            ledgerBlock(
                title: "Holdings (obtain)",
                status: snapshot.holdingsStatus,
                kind: "Holdings",
                rows: snapshot.holdingsRows
            )
            ledgerBlock(
                title: "Positions (obtain) — not Today fill inventory",
                status: snapshot.positionsStatus,
                kind: "Positions",
                rows: snapshot.positionsRows
            )
        }
        .padding(12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
    }

    @ViewBuilder
    private func ledgerBlock(
        title: String,
        status: String,
        kind: String,
        rows: [BarAccountChrome.Row]
    ) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title)
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
            if let line = BarAccountChrome.listStatusLine(kind: kind, status: status) {
                Text(line)
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Accent.amber)
            } else if rows.isEmpty {
                Text("None")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
            } else {
                HStack {
                    Text("Symbol")
                    Spacer()
                    Text("Qty")
                }
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
                ForEach(rows) { row in
                    HStack {
                        Text(row.symbol)
                        Spacer()
                        Text(row.qty)
                            .font(BarDS.monoFont(12, weight: .regular))
                    }
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.primary)
                }
            }
        }
    }

    private var accessibilitySummary: String {
        "\(snapshot.bookLabel) \(snapshot.freeText). \(snapshot.holdingsCount) holdings. \(snapshot.positionsCount) positions. \(BarAccountChrome.ordersCountLabel(snapshot))"
    }
}
