import SwiftUI

/// Pre-trade cockpit for spot / equity / USDM / Coin-M (and leftover Options on `.standardForm`).
/// Glance strip + session/depth mosaic + Plan rail. Confirm is LiveBook — TRADE parked.
struct BarCashCockpitView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarCashGlanceStrip(viewModel: viewModel)
            HStack(alignment: .top, spacing: 10) {
                BarCashCockpitMosaic(
                    viewModel: viewModel,
                    sideBuy: $sideBuy,
                    quantityText: $quantityText,
                    stopLossText: $stopLossText,
                    targetPriceText: $targetPriceText,
                )
                BarCashCockpitPlanRail(
                    viewModel: viewModel,
                    sideBuy: $sideBuy,
                    quantityText: $quantityText,
                    stopLossText: $stopLossText,
                    targetPriceText: $targetPriceText,
                    submitReady: submitReady,
                    submitHint: submitHint,
                    onConfirm: onConfirm,
                )
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
