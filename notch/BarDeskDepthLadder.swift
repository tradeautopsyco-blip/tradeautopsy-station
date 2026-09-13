import SwiftUI

/// One venue ladder level. Strings only — never reparsed Doubles.
struct DeskDepthLevel: Identifiable, Equatable {
    var id: String { "\(side)-\(price)-\(quantity)-\(orders ?? "")" }
    let side: String
    let price: String
    let quantity: String
    let orders: String?
}

/// Shared Notch ladder. Success paints bids/asks; Unusable/Unavailable are typed holes.
/// Never a `synced` badge — REST books and COM gaps stay bounded_snapshot / unusable.
struct BarDeskDepthLadder: View {
    let status: String
    var display: Bool = true
    let bids: [DeskDepthLevel]
    let asks: [DeskDepthLevel]
    let physicsNote: String

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if !display {
                hole(.unavailable)
            } else if let honesty = HonestyStatus.fromWire(status) {
                hole(honesty)
            } else if status.lowercased() == "success", !bids.isEmpty || !asks.isEmpty {
                HStack(alignment: .top, spacing: 12) {
                    column(title: "bids", rows: bids)
                    column(title: "asks", rows: asks)
                }
                note
            } else {
                hole(.unavailable)
            }
        }
        .padding(11)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
    }

    private var note: some View {
        Text(physicsNote)
            .font(BarDS.monoFont(10, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .fixedSize(horizontal: false, vertical: true)
    }

    private func hole(_ status: HonestyStatus) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HonestyChip(status: status)
            note
        }
    }

    private func column(title: String, rows: [DeskDepthLevel]) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            ForEach(rows) { row in
                HStack(spacing: 6) {
                    Text(row.price)
                        .font(BarDS.monoFont(11, weight: .medium))
                        .foregroundColor(BarDS.Text.primary)
                    Text(row.quantity)
                        .font(BarDS.monoFont(11, weight: .regular))
                        .foregroundColor(BarDS.Text.secondary)
                    if let orders = row.orders {
                        Text(orders)
                            .font(BarDS.monoFont(10, weight: .regular))
                            .foregroundColor(BarDS.Text.muted)
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
