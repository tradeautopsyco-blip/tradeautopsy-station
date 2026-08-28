import SwiftUI

/// Non-interactive capsule for one honesty state.
struct HonestyChip: View {
    let status: HonestyStatus

    var body: some View {
        Text(status.chipLabel)
            .font(BarDS.bodyFont(BarDS.FontSize.chip, weight: .regular))
            .foregroundColor(BarDS.Text.secondary)
            .padding(.vertical, 5)
            .padding(.horizontal, 11)
            .background(Color.white.opacity(0.03))
            .clipShape(Capsule())
            .overlay(
                Capsule()
                    .stroke(BarDS.Border.chipUnselected, lineWidth: BarDS.borderThin)
            )
            .accessibilityLabel(status.chipLabel)
            .allowsHitTesting(false)
    }
}

/// Catalog of all four honesty chips. Nothing else is visual in this track.
struct HonestyChipCatalog: View {
    var body: some View {
        HStack(spacing: 6) {
            ForEach(HonestyStatus.allCases, id: \.self) { status in
                HonestyChip(status: status)
            }
        }
    }
}
