import Foundation
import Notch

@MainActor
public final class StationAppCoordinator: ObservableObject {
    @Published public private(set) var activeRoute: StationRoute
    @Published public private(set) var agentHealthWarning: AgentHealthWarning?

    /// Shell navigation stays enabled even when the agent is unhealthy (TRD §6.3).
    public var isShellNavigable: Bool { true }

    /// Pulse strip shows em-dash placeholders when agent data is unavailable (TRD §11.2).
    public var isPulseStripDegraded: Bool { !agentSupervisor.isHealthy }

    private let agentSupervisor: AgentSupervising
    private let statusItemController: StatusItemControlling
    private let hotkeyRegistrar: HotkeyRegistering
    private let notchHost: NotchHosting
    private let notchPolling: NotchPollingControlling
    private let windowController: StationWindowControlling
    private let launchStore: StationLaunchStoring
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
        deskRouteStore: DeskRouteStoring = UserDefaultsDeskRouteStore(),
        dateProvider: @escaping () -> Date = Date.init
    ) {
        self.agentSupervisor = agentSupervisor
        self.statusItemController = statusItemController
        self.hotkeyRegistrar = hotkeyRegistrar
        self.notchHost = notchHost
        self.notchPolling = notchPolling
        self.windowController = windowController
        self.launchStore = launchStore
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

    public func launch() async {
        windowController.restoreFrame()
        await agentSupervisor.start()
        syncAgentHealthFromSupervisor()
        statusItemController.install(coordinator: self)

        if agentSupervisor.isHealthy {
            await startNotchAndPolling()
        }

        registerHotkeys()
        presentWindowOnLaunchIfNeeded()
    }

    public func openStation() {
        windowController.showAndActivate()
        launchStore.setWasWindowVisibleBeforeQuit(true)
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
        hotkeyRegistrar.registerToggleNotch { }
        hotkeyRegistrar.registerOpenStation { [weak self] in
            Task { @MainActor in
                self?.openStation()
            }
        }
    }

    private func presentWindowOnLaunchIfNeeded() {
        if !launchStore.isFirstLaunchCompleted {
            windowController.show(orderFrontOnly: true)
            launchStore.setFirstLaunchCompleted()
            launchStore.setWasWindowVisibleBeforeQuit(true)
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
