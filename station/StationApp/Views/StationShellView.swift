import SwiftUI

public struct StationShellView: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @State private var columnVisibility: NavigationSplitViewVisibility = .all

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public var body: some View {
        VStack(spacing: 0) {
            SessionPulseStrip(
                viewModel: coordinator.sessionModel,
                coordinator: coordinator
            )

            notificationBanner

            NavigationSplitView(columnVisibility: $columnVisibility) {
                StationSidebarList(coordinator: coordinator)
                    .navigationSplitViewColumnWidth(min: 180, ideal: 220, max: 320)
            } detail: {
                routeContent
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(StationDS.Fill.appPanel)
                    .navigationTitle(coordinator.activeRoute.rawValue)
            }
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    AgentHealthToolbarStatus(coordinator: coordinator)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(StationDS.Fill.appPanel)
        .tint(StationDS.Accent.teal)
    }

    @ViewBuilder
    private var notificationBanner: some View {
        switch coordinator.currentNotification {
        case .inputMonitoringWarning(let warning):
            InputMonitoringWarningView(warning: warning) {
                coordinator.dismissInputMonitoringWarning()
            }
        case .inputMonitoringRestartReminder(let reminder):
            InputMonitoringRestartReminderView(message: reminder) {
                coordinator.dismissInputMonitoringRestartReminder()
            }
        case .loginItemPrompt:
            LoginItemPromptView(
                onEnable: { coordinator.enableLaunchAtLoginFromPrompt() },
                onSkip: { coordinator.skipLoginItemPrompt() }
            )
        case nil:
            EmptyView()
        }
    }

    @ViewBuilder
    private var routeContent: some View {
        switch coordinator.activeRoute {
        // Phase 0 exception: BrokersView and SettingsView are the only non-placeholder routes.
        // Decision recorded in issue #5, #7, and #8.
        case .brokers:
            BrokersView(viewModel: coordinator.brokersViewModel)
        case .health:
            HealthPanelView(viewModel: coordinator.healthPanelViewModel)
        case .marketData:
            MarketDataKeysView(viewModel: coordinator.marketDataKeysViewModel)
        case .aiWorkflow:
            AIWorkflowKeysView(viewModel: coordinator.aiWorkflowKeysViewModel)
        case .today:
            TodayView(
                viewModel: coordinator.todayViewModel,
                journalViewModel: coordinator.journalViewModel,
                onOpenNotch: { coordinator.toggleNotch() }
            )
        case .journal:
            JournalView(viewModel: coordinator.journalViewModel, onOpenNotch: { coordinator.toggleNotch() })
        case .settings:
            SettingsView(coordinator: coordinator)
        }
    }
}
