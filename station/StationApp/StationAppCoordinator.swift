import Foundation
import Notch

@MainActor
public final class StationAppCoordinator: ObservableObject {
    @Published public private(set) var activeRoute: StationRoute
    @Published public private(set) var agentHealthWarning: AgentHealthWarning?
    public let notchViewModel: NotchViewModel
    @Published public private(set) var inputMonitoringWarning: InputMonitoringWarning?
    @Published public private(set) var launchAtLoginEnabled = false
    @Published public private(set) var showLoginItemPrompt = false

    /// Shell navigation stays enabled even when the agent is unhealthy (TRD §6.3).
    public var isShellNavigable: Bool { true }

    /// Pulse strip shows em-dash placeholders when agent data is unavailable (TRD §11.2).
    public var isPulseStripDegraded: Bool { !agentSupervisor.isHealthy }

    private let agentSupervisor: AgentSupervising
    private let statusItemController: StatusItemControlling
    private let hotkeyRegistrar: HotkeyRegistering
    private let inputMonitoringChecker: InputMonitoringChecking
    private let notchHost: NotchHosting
    private let notchPolling: NotchPollingControlling
    private let windowController: StationWindowControlling
    private let launchStore: StationLaunchStoring
    private let loginItemService: LoginItemServicing
    private let phaseProvider: BarSurfacePhaseProviding
    private let deskRouteStore: DeskRouteStoring
    private let dateProvider: () -> Date

    private var manualSessionPickAt: Date?

    public init(
        agentSupervisor: AgentSupervising,
        statusItemController: StatusItemControlling,
        hotkeyRegistrar: HotkeyRegistering,
        notchHost: NotchHosting,
        notchPolling: NotchPollingControlling,
        windowController: StationWindowControlling,
        launchStore: StationLaunchStoring,
        phaseProvider: BarSurfacePhaseProviding,
        loginItemService: LoginItemServicing = LoginItemService(),
        notchViewModel: NotchViewModel? = nil,
        deskRouteStore: DeskRouteStoring = UserDefaultsDeskRouteStore(),
        dateProvider: @escaping () -> Date = Date.init,
        inputMonitoringChecker: InputMonitoringChecking = DefaultInputMonitoringChecker()
    ) {
        self.agentSupervisor = agentSupervisor
        self.statusItemController = statusItemController
        self.hotkeyRegistrar = hotkeyRegistrar
        self.inputMonitoringChecker = inputMonitoringChecker
        self.notchViewModel = notchViewModel ?? NotchViewModel()
        self.notchHost = notchHost
        self.notchPolling = notchPolling
        self.windowController = windowController
        self.launchStore = launchStore
        self.loginItemService = loginItemService
        self.phaseProvider = phaseProvider
        self.deskRouteStore = deskRouteStore
        self.dateProvider = dateProvider

        self.activeRoute = NavigationPolicy.launchRoute(
            saved: deskRouteStore.savedDeskRoute,
            phase: phaseProvider.barSurfacePhase
        )

        phaseProvider.onPhaseChange = { [weak self] phase in
            self?.handlePhaseChange(phase)
        }

        agentSupervisor.onHealthChange = { [weak self] isHealthy in
            Task { @MainActor in
                self?.handleAgentHealthChange(isHealthy: isHealthy)
            }
        }
    }

    public func navigateTo(_ route: StationRoute) {
        if route.isSession {
            manualSessionPickAt = dateProvider()
        }
        if route.isDesk {
            deskRouteStore.saveDeskRoute(route)
        }
        activeRoute = route
    }

    public func openLiveTradeFromPulseStrip() {
        navigateTo(.liveTrade)
        windowController.showAndActivate()
    }

    public func launch() async {
        windowController.restoreFrame()
        loginItemService.syncStatusOnLaunch()
        launchAtLoginEnabled = loginItemService.isRegistered
        await agentSupervisor.start()
        syncAgentHealthFromSupervisor()
        statusItemController.install(coordinator: self)
        statusItemController.updateLaunchAtLoginEnabled(launchAtLoginEnabled)

        if agentSupervisor.isHealthy {
            await startNotchAndPolling()
        }

        registerHotkeys()
        syncInputMonitoringWarning()
        presentWindowOnLaunchIfNeeded()
    }

    public func dismissInputMonitoringWarning() {
        inputMonitoringWarning = nil
    }

    public func toggleLaunchAtLogin() {
        let next = !launchAtLoginEnabled
        do {
            try loginItemService.setRegistered(next)
            launchAtLoginEnabled = loginItemService.isRegistered
            statusItemController.updateLaunchAtLoginEnabled(launchAtLoginEnabled)
        } catch {
            launchAtLoginEnabled = loginItemService.isRegistered
            statusItemController.updateLaunchAtLoginEnabled(launchAtLoginEnabled)
        }
    }

    public func enableLaunchAtLoginFromPrompt() {
        do {
            try loginItemService.setRegistered(true)
            launchAtLoginEnabled = loginItemService.isRegistered
            statusItemController.updateLaunchAtLoginEnabled(launchAtLoginEnabled)
        } catch {
            launchAtLoginEnabled = loginItemService.isRegistered
            statusItemController.updateLaunchAtLoginEnabled(launchAtLoginEnabled)
        }
        dismissLoginItemPrompt()
    }

    public func skipLoginItemPrompt() {
        dismissLoginItemPrompt()
    }

    private func dismissLoginItemPrompt() {
        loginItemService.markPromptDismissed()
        showLoginItemPrompt = false
    }

    public func openStation() {
        windowController.showAndActivate()
        launchStore.setWasWindowVisibleBeforeQuit(true)
    }

    public func toggleNotch() {
        notchHost.toggle()
    }

    public func closeWindow() {
        windowController.hide()
        launchStore.setWasWindowVisibleBeforeQuit(false)
    }

    public func retryAgent() async {
        agentHealthWarning = nil
        await agentSupervisor.retry()
        syncAgentHealthFromSupervisor()

        if agentSupervisor.isHealthy {
            await startNotchAndPolling()
        }
    }

    public func quit() async {
        if windowController.isVisible {
            windowController.hide()
        }
        hotkeyRegistrar.unregisterAll()
        await agentSupervisor.shutdown()
        notchPolling.stopPolling()
        notchHost.dismiss()
    }

    private func handlePhaseChange(_ phase: BarSurfacePhase) {
        guard let nextRoute = NavigationPolicy.shouldAutoFollowPhase(
            active: activeRoute,
            phase: phase,
            manualSessionPickAt: manualSessionPickAt,
            now: dateProvider()
        ) else {
            return
        }
        activeRoute = nextRoute
    }

    private func startNotchAndPolling() async {
        await notchHost.start()
        notchPolling.startPolling()
    }

    private func syncAgentHealthFromSupervisor() {
        agentHealthWarning = agentSupervisor.currentWarning
        statusItemController.updateAgentStatus(isHealthy: agentSupervisor.isHealthy)
    }

    private func handleAgentHealthChange(isHealthy: Bool) {
        syncAgentHealthFromSupervisor()
        objectWillChange.send()
    }

    private func registerHotkeys() {
        hotkeyRegistrar.registerToggleNotch { [weak self] in
            self?.notchHost.toggle()
        }
        hotkeyRegistrar.registerOpenStation { [weak self] in
            self?.openStation()
        }
    }

    private func syncInputMonitoringWarning() {
        if !inputMonitoringChecker.isInputMonitoringGranted() {
            inputMonitoringWarning = InputMonitoringWarning()
        } else {
            inputMonitoringWarning = nil
        }
    }

    private func presentWindowOnLaunchIfNeeded() {
        if !launchStore.isFirstLaunchCompleted {
            windowController.show(orderFrontOnly: true)
            launchStore.setFirstLaunchCompleted()
            launchStore.setWasWindowVisibleBeforeQuit(true)
            if !loginItemService.isPromptDismissed {
                showLoginItemPrompt = true
            }
            return
        }

        if launchStore.wasWindowVisibleBeforeQuit {
            windowController.show(orderFrontOnly: true)
            return
        }

        if launchStore.loginAtBoot {
            return
        }
    }
}
