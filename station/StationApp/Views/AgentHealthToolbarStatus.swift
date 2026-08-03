import SwiftUI

/// Persistent agent-connectivity indicator for the toolbar's trailing zone —
/// replaces the old always-visible WarningBannerView-in-a-stack with the native
/// pattern (small status glyph, full detail on demand), matching the same
/// .systemRed convention StatusItemController already uses for the menu-bar icon.
public struct AgentHealthToolbarStatus: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @State private var showingDetail = false

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        Button {
            if coordinator.agentHealthWarning != nil {
                showingDetail = true
            }
        } label: {
            Circle()
                .fill(Color(nsColor: coordinator.agentHealthWarning == nil ? .systemGreen : .systemRed))
                .frame(width: 8, height: 8)
                .frame(width: 22, height: 22)
        }
        .buttonStyle(.plain)
        .help(coordinator.agentHealthWarning == nil ? "Agent healthy" : "Agent unavailable — click for details")
        .popover(isPresented: $showingDetail) {
            if let warning = coordinator.agentHealthWarning {
                WarningBannerView(warning: warning) {
                    Task { await coordinator.retryAgent() }
                }
                .frame(width: 320)
            }
        }
    }
}
