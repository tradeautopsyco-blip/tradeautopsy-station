import SwiftUI

/// Shipping-book funds glance for Plan rails (obtain `funds`, not S8 ledger).
struct BarShippingFundsPlanRow: View {
    let glance: BarShippingFundsGlance.Presentation

    var body: some View {
        HStack {
            Text("Funds (shipping book)")
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.hint)
            Spacer(minLength: 8)
            if glance.isLit {
                Text(glance.freeText)
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
            } else if let honesty = HonestyStatus.fromWire(glance.status) {
                HonestyChip(status: honesty)
            } else {
                Text("—")
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("Shipping book funds \(glance.freeText)")
    }
}
