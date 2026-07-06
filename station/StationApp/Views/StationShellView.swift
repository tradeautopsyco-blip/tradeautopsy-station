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

            if let reminder = coordinator.inputMonitoringRestartReminder {
                InputMonitoringRestartReminderView(message: reminder) {
                    coordinator.dismissInputMonitoringRestartReminder()
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
                routeContent
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(BarDS.Fill.sidebar)
        .onAppear {
            syncDeclarationFormForActiveRoute()
        }
        .onChange(of: coordinator.activeRoute) { _, _ in
            syncDeclarationFormForActiveRoute()
        }
    }

    private func syncDeclarationFormForActiveRoute() {
        coordinator.notchViewModel.showingDeclarationForm = (coordinator.activeRoute == .preTrade)
    }

    @ViewBuilder
    private var routeContent: some View {
        switch coordinator.activeRoute {
        // Phase 0 exception: BrokersView and SettingsView are the only non-placeholder routes.
        // Decision recorded in issue #5, #7, and #8.
        case .brokers:
            BrokersView(viewModel: coordinator.brokersViewModel)
        case .today:
            TodayView(viewModel: coordinator.todayViewModel)
        case .settings:
            SettingsView(coordinator: coordinator)
        default:
            StationPlaceholderView(route: coordinator.activeRoute)
        }
    }
}
