import Notch
import SwiftUI

/// Sidebar footer row — shipping-book `obtain(funds)` glance (Wave 4.1).
struct StationShippingFundsSidebarRow: View {
    let glance: BarShippingFundsGlance.Presentation
    let brokerSyncClass: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text("Funds")
                    .foregroundStyle(StationDS.Text.secondary)
                Spacer(minLength: 8)
                fundsValue
            }
            HStack {
                Text("Broker sync")
                    .foregroundStyle(StationDS.Text.muted)
                Spacer(minLength: 8)
                Text(brokerSyncClass)
                    .foregroundStyle(StationDS.Text.secondary)
            }
        }
        .font(StationDS.monoFont(StationDS.FontSize.chip, weight: .medium))
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(StationDS.Fill.sidebar)
        .overlay(alignment: .top) {
            Rectangle()
                .fill(StationDS.Border.divider)
                .frame(height: StationDS.borderThin)
        }
        .accessibilityIdentifier("stationShippingFundsRow")
    }

    @ViewBuilder
    private var fundsValue: some View {
        if glance.isLit {
            Text(glance.freeText)
                .foregroundStyle(StationDS.Text.primary)
        } else {
            Text("—")
                .foregroundStyle(StationDS.Text.muted)
        }
    }
}
