import SwiftUI

/// Routes behavioral-circuit surfaces by phase (declaration → live plan → debrief). No M4 logic — all from `notch`.
struct BarCircuitPanelView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        BarNotchShell(viewModel: viewModel)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
