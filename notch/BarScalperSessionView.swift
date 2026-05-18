import SwiftUI

/// Scalper session declaration surface (#117). Ships as a routed shell first; expand with session-level fields in that issue.
struct BarScalperSessionView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        BarDeclarationFlowView(viewModel: viewModel)
    }
}
