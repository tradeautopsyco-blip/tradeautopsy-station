import SwiftUI

/// Restarts `tradeautopsy-agent` on loopback without bypassing an active kill latch.
public struct AgentRestartToolbarButton: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @State private var isRestarting = false

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        Button {
            guard !isRestarting else { return }
            isRestarting = true
            Task {
                await coordinator.restartAgentProcess()
                isRestarting = false
            }
        } label: {
            if isRestarting {
                ProgressView()
                    .controlSize(.small)
                    .frame(width: 22, height: 22)
            } else {
                Image(systemName: "arrow.clockwise.circle")
                    .frame(width: 22, height: 22)
            }
        }
        .buttonStyle(.plain)
        .disabled(isRestarting)
        .help("Restart agent on port 9137 (refused while kill switch is latched)")
    }
}
