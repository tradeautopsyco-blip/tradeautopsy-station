import SwiftUI

/// Pre-trade cockpit for Kotak NFO **options** (OPT*). Mosaic + 6-cell glance strip + Plan rail.
/// Futures and unknown kind stay on `BarOptionsDeclareView` (three-zone).
/// Confirm is LiveBook intent — TRADE parked, no COM ticket.
struct BarNfoOptionsCockpitView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarOptionsGlanceStrip(viewModel: viewModel)
            HStack(alignment: .top, spacing: 10) {
                BarNfoCockpitMosaic(viewModel: viewModel)
                BarNfoCockpitPlanRail(
                    viewModel: viewModel,
                    sideBuy: $sideBuy,
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
