import SwiftUI

/// Venue-stack spot ticket: MARKET | LIMIT | LIMIT_MAKER. Size is base XOR `quoteOrderQty`.
struct BarSpotTicketView: View {
    @Binding var ticket: BarDeskTicketIntent
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var quoteOrderQtyText: String
    var availableLine: String?

    private var spec: BarDeskTicketSpec { BarDeskTicketSpec.forBook(ticket.bookId) }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 5) {
                pretradeSidePill(title: "Buy", selected: sideBuy) { sideBuy = true }
                pretradeSidePill(title: "Sell", selected: !sideBuy) { sideBuy = false }
            }
            typeTabs
            HStack(spacing: 5) {
                BarChip(label: "base qty", selected: ticket.sizeMode == .base) {
                    ticket.sizeMode = .base
                    ticket = ticket.coerced()
                }
                BarChip(label: "quoteOrderQty", selected: ticket.sizeMode == .quote) {
                    ticket.sizeMode = .quote
                    ticket = ticket.coerced()
                }
            }
            if ticket.sizeMode == .quote {
                BarInputField(placeholder: "quoteOrderQty · USDT", text: $quoteOrderQtyText)
            } else {
                BarInputField(placeholder: "Quantity · base", text: $quantityText)
            }
            if ticket.showsTif {
                tifRow
            }
            if let availableLine, !availableLine.isEmpty {
                Text(availableLine)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.hint)
            }
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

    private var tifRow: some View {
        HStack(spacing: 5) {
            ForEach(BarTicketTif.allCases) { t in
                BarChip(label: t.rawValue, selected: ticket.tif == t) {
                    ticket.tif = t
                }
            }
        }
    }

    private func pretradeSidePill(title: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(selected ? BarDS.Text.primary : BarDS.Text.secondary)
                .padding(.vertical, 6)
                .padding(.horizontal, 16)
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
