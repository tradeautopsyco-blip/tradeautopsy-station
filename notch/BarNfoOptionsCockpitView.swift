import SwiftUI

/// Pre-trade cockpit for Kotak NFO **options** (OPT*). Mosaic + Plan rail.
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
    }
}
