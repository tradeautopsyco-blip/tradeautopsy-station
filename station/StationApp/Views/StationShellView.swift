import SwiftUI

public struct StationShellView: View {
    @ObservedObject private var coordinator: StationAppCoordinator
    @ObservedObject private var today: TodayViewModel
    @ObservedObject private var brokers: BrokersViewModel
    @State private var railCollapsed = false
    @State private var selectedSymbol: String?
    @State private var selectedSearch: BookAutopsySearch?

    public init(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
        self.today = coordinator.todayViewModel
        self.brokers = coordinator.brokersViewModel
    }

    public var body: some View {
        VStack(spacing: 0) {
            notificationBanner

            HStack(spacing: 0) {
                StationSidebarList(
                    coordinator: coordinator,
                    collapsed: $railCollapsed,
                    selectedSymbol: $selectedSymbol,
                    selectedSearch: $selectedSearch
                )
                .frame(width: railCollapsed ? WorkspaceChrome.railCollapsedWidth : WorkspaceChrome.railWidth)

                Rectangle()
                    .fill(WorkspaceChrome.line)
                    .frame(width: 1)

                routeContent
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
                    .background(WorkspaceChrome.ground)
                    .navigationTitle(coordinator.activeRoute.rawValue)
            }
            .toolbar {
                ToolbarItemGroup(placement: .primaryAction) {
                    AgentRestartToolbarButton(coordinator: coordinator)
                    AgentHealthToolbarStatus(coordinator: coordinator)
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .background(WorkspaceChrome.ground)
        .tint(StationDS.Accent.teal)
    }

    @ViewBuilder
    private var notificationBanner: some View {
        switch coordinator.currentNotification {
        case .deviceLoginRequired:
            DeviceLoginRequiredView {
                coordinator.openStationForDeviceLogin()
            }
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
            HealthPanelView(viewModel: coordinator.healthPanelViewModel, coordinator: coordinator)
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
        case .report:
            BookAutopsyReportView(
                today: today,
                brokers: brokers,
                selectedSymbol: $selectedSymbol,
                selectedSearch: $selectedSearch,
                onOpenJournal: { coordinator.navigateTo(.journal) },
                onOpenNotch: { coordinator.toggleNotch() }
            )
        case .settings:
            SettingsView(coordinator: coordinator)
        }
    }
}
