import SwiftUI

/// Venue-stack USDM / Coin-M ticket. DualNoBlend by `book_id` — Coin-M Conditional is dapi `algoOrder`.
/// Cross / leverage / liq are read-only when a positionbook row exists; none if flat. Never POST leverage.
struct BarUsdmTicketView: View {
    @Binding var ticket: BarDeskTicketIntent
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var triggerPriceText: String
    var marginMode: String?
    var leverage: String?
    var liquidationPrice: String?

    private var spec: BarDeskTicketSpec { BarDeskTicketSpec.forBook(ticket.bookId) }
    private var isCoinM: Bool { ticket.bookId == BarDeskTemplate.binanceComCoinmBookId }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 5) {
                pretradeSidePill(title: "Buy / Long", selected: sideBuy) { sideBuy = true }
                pretradeSidePill(title: "Sell / Short", selected: !sideBuy) { sideBuy = false }
            }
            typeTabs
            if ticket.type != .market {
                Text("Limit price lives on Entry. Plan SL/TP are not venue STOP_MARKET.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            if ticket.showsTif {
                HStack(spacing: 5) {
                    ForEach(BarTicketTif.allCases) { t in
                        BarChip(label: t.rawValue, selected: ticket.tif == t) {
                            ticket.tif = t
                        }
                    }
                }
            }
            if ticket.type == .conditional {
                BarInputField(
                    placeholder: isCoinM ? "triggerPrice · dapi algoOrder" : "triggerPrice · algoOrder",
                    text: $triggerPriceText,
                )
            }
            BarInputField(placeholder: "Contracts", text: $quantityText)
            BarChip(label: "Reduce-only", selected: ticket.reduceOnly) {
                ticket.reduceOnly.toggle()
            }
            marginChips
            Text("path \(ticket.path) · Confirm is LiveBook intent, not a venue POST")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var typeTabs: some View {
        HStack(spacing: 4) {
            ForEach(spec.types) { t in
                Button {
                    ticket.type = t
                    ticket = ticket.coerced()
                } label: {
                    Text(spec.label(for: t))
                        .font(BarDS.bodyFont(11, weight: .semibold))
                        .foregroundColor(ticket.type == t ? BarDS.Text.primary : BarDS.Text.secondary)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 7)
                        .background(ticket.type == t ? Color.white.opacity(0.10) : Color.white.opacity(0.03))
                        .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                }
                .buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder
    private var marginChips: some View {
        HStack(spacing: 5) {
            readoutChip(marginMode ?? "none")
            readoutChip(leverage.map { "\($0) read-only" } ?? "x none")
            readoutChip(liquidationPrice.map { "liq \($0)" } ?? "liq none")
        }
    }

    private func readoutChip(_ text: String) -> some View {
        Text(text)
            .font(BarDS.monoFont(10, weight: .medium))
            .foregroundColor(BarDS.Text.hint)
            .padding(.vertical, 4)
            .padding(.horizontal, 8)
            .background(Color.white.opacity(0.04))
            .clipShape(Capsule())
    }

    private func pretradeSidePill(title: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(selected ? BarDS.Text.primary : BarDS.Text.secondary)
                .padding(.vertical, 6)
                .padding(.horizontal, 12)
                .background(selected ? Color.white.opacity(0.10) : Color.white.opacity(0.03))
                .clipShape(Capsule())
                .overlay(
                    Capsule()
                        .stroke(
                            selected ? Color.white.opacity(0.20) : Color.white.opacity(0.10),
                            lineWidth: 0.5,
                        ),
                )
        }
        .buttonStyle(.plain)
    }
}
