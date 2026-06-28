import Notch
import SwiftUI

public struct StationShellView: View {
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        VStack(spacing: 0) {
            SessionPulseStrip(
                viewModel: coordinator.notchViewModel,
                coordinator: coordinator
            )

            if let warning = coordinator.agentHealthWarning {
                WarningBannerView(warning: warning) {
                    Task { await coordinator.retryAgent() }
                }
            }

            if let warning = coordinator.inputMonitoringWarning {
                InputMonitoringWarningView(warning: warning) {
                    coordinator.dismissInputMonitoringWarning()
                }
            }

            if coordinator.showLoginItemPrompt {
                LoginItemPromptView(
                    onEnable: { coordinator.enableLaunchAtLoginFromPrompt() },
                    onSkip: { coordinator.skipLoginItemPrompt() }
                )
            }

            HStack(spacing: 0) {
                StationSidebar(coordinator: coordinator)
                StationPlaceholderView(route: coordinator.activeRoute)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
    }
}
