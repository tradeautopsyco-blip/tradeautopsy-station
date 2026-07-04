import Notch
import SwiftUI

/// Expanded Notch surface when hosted by Station — same shell tree as the main window.
public struct HostedExpandedNotchShell: View {
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        VStack(spacing: 0) {
            Color.clear
                .frame(height: chromeTapStripHeight)
                .frame(maxWidth: .infinity)
                .contentShape(Rectangle())
                .onTapGesture {
                    coordinator.notchViewModel.collapseExpandedFromChromeTap()
                }

            StationShellView(coordinator: coordinator)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }

    private var chromeTapStripHeight: CGFloat {
        let inset = coordinator.notchViewModel.notchTopInset
        return inset > 0 ? inset : 28
    }
}
