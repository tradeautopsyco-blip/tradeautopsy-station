import Foundation

@MainActor
public final class StationAppCoordinator: ObservableObject {
    @Published public private(set) var activeRoute: StationRoute
    @Published public private(set) var agentHealthWarning: AgentHealthWarning?
    public let sessionModel: SessionModel
    public let brokersViewModel: BrokersViewModel
    public let marketDataKeysViewModel: MarketDataKeysViewModel
    public let aiWorkflowKeysViewModel: AIWorkflowKeysViewModel
    public let todayViewModel: TodayViewModel
    public let deviceLoginViewModel: DeviceLoginViewModel
    @Published public private(set) var inputMonitoringWarning: InputMonitoringWarning?
    @Published public private(set) var inputMonitoringRestartReminder: String?
    @Published public private(set) var launchAtLoginEnabled = false
    @Published public private(set) var showLoginItemPrompt = false

    /// Shell navigation stays enabled even when the agent is unhealthy (TRD §6.3).
    public var isShellNavigable: Bool { true }

    /// The single onboarding notification to show, if any (agent health has its own
    /// permanent home in the toolbar and isn't part of this).
    public var currentNotification: StationNotification? {
        StationNotification.current(
            inputMonitoringWarning: inputMonitoringWarning,
            inputMonitoringRestartReminder: inputMonitoringRestartReminder,
            showLoginItemPrompt: showLoginItemPrompt
        )
    }

    /// Pulse strip shows em-dash placeholders when agent data is unavailable (TRD §11.2).
    public var isPulseStripDegraded: Bool { !agentSupervisor.isHealthy }

    private let agentSupervisor: AgentSupervising
    private let statusItemController: StatusItemControlling
    private let hotkeyRegistrar: HotkeyRegistering
    private let inputMonitoringChecker: InputMonitoringChecking
    private let sessionHost: SessionHosting
    private let floatingNotch: FloatingNotchHosting
    private let sessionPolling: SessionPollingControlling
    private let windowController: StationWindowControlling
    private let launchStore: StationLaunchStoring
    private let loginItemService: LoginItemServicing
    private let phaseProvider: SessionSurfacePhaseProviding
    private let deskRouteStore: DeskRouteStoring
    private let dateProvider: () -> Date

    private var manualSessionPickAt: Date?
    private var notchAndPollingStarted = false
    private var pollingStoppedForUnhealthyAgent = false
    private var inputMonitoringWarningShownAt: Date?
    private var inputMonitoringPollTask: Task<Void, Never>?
    private var didShowInputMonitoringRestartReminder = false

    public init(
        agentSupervisor: AgentSupervising,
        statusItemController: StatusItemControlling,
        hotkeyRegistrar: HotkeyRegistering,
        sessionHost: SessionHosting,
        sessionPolling: SessionPollingControlling,
        windowController: StationWindowControlling,
        launchStore: StationLaunchStoring,
        phaseProvider: SessionSurfacePhaseProviding,
        loginItemService: LoginItemServicing = LoginItemService(),
        sessionModel: SessionModel? = nil,
        deskRouteStore: DeskRouteStoring = UserDefaultsDeskRouteStore(),
        dateProvider: @escaping () -> Date = Date.init,
        inputMonitoringChecker: InputMonitoringChecking = DefaultInputMonitoringChecker(),
        brokerControl: BrokerControlling? = nil,
        todayClient: TodayAgentClient? = nil,
        daemonSecret: String? = nil,
        deviceLoginClient: (any DeviceLoginClient)? = nil,
        floatingNotch: FloatingNotchHosting
    ) {
        let resolvedDaemonSecret = daemonSecret ?? AgentDaemonSecret.resolveForSession()
        self.agentSupervisor = agentSupervisor
        self.statusItemController = statusItemController
        self.hotkeyRegistrar = hotkeyRegistrar
        self.inputMonitoringChecker = inputMonitoringChecker
        self.sessionModel = sessionModel ?? SessionModel()
        self.sessionHost = sessionHost
        self.floatingNotch = floatingNotch
        self.sessionPolling = sessionPolling
        self.windowController = windowController
        self.launchStore = launchStore
        self.loginItemService = loginItemService
        self.phaseProvider = phaseProvider
        self.deskRouteStore = deskRouteStore
        self.dateProvider = dateProvider

        let resolvedCredentialStore = KeychainBrokerCredentialStore()
        let resolvedMetadataStore = UserDefaultsBrokerMetadataStore()
        let resolvedRuntimeClient = LocalAgentBrokerRuntimeClient(daemonSecret: resolvedDaemonSecret)
        let resolvedSyncControl = AgentBrokerSyncControl(
            credentialStore: resolvedCredentialStore,
            runtimeClient: resolvedRuntimeClient
        )
        let resolvedBrokerControl = brokerControl ?? LocalBrokerControlClient(
            agentSupervisor: agentSupervisor,
            credentialStore: resolvedCredentialStore,
            metadataStore: resolvedMetadataStore,
            syncControl: resolvedSyncControl,
            runtimeClient: resolvedRuntimeClient
        )
        self.brokersViewModel = BrokersViewModel(
            brokerControl: resolvedBrokerControl,
            credentialStore: resolvedCredentialStore,
            metadataStore: resolvedMetadataStore,
            syncControl: resolvedSyncControl,
            runtimeClient: resolvedRuntimeClient
        )
        self.marketDataKeysViewModel = MarketDataKeysViewModel()
        self.aiWorkflowKeysViewModel = AIWorkflowKeysViewModel()

        let resolvedTodayClient = todayClient ?? LocalTodayAgentClient(
            daemonSecret: resolvedDaemonSecret,
            isAgentHealthy: { agentSupervisor.isHealthy }
        )
        let session = self.sessionModel
        let brokers = self.brokersViewModel
        self.todayViewModel = TodayViewModel(
            client: resolvedTodayClient,
            sessionModel: session,
            agentHealthy: { agentSupervisor.isHealthy },
            isBrokerSyncActive: { session.isBrokerSyncActiveForTodayMirror },
            configuredSlugs: { brokers.configuredBrokerSlugs }
        )
        let resolvedDeviceLoginClient = deviceLoginClient ?? LocalDeviceLoginAgentClient(
            daemonSecret: resolvedDaemonSecret,
            isAgentHealthy: { agentSupervisor.isHealthy }
        )
        self.deviceLoginViewModel = DeviceLoginViewModel(client: resolvedDeviceLoginClient)

        self.activeRoute = NavigationPolicy.launchRoute(
            saved: deskRouteStore.savedDeskRoute,
            phase: phaseProvider.sessionSurfacePhase
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
        stopInputMonitoringPollingIfNeeded()
    }

    public func dismissInputMonitoringRestartReminder() {
        inputMonitoringRestartReminder = nil
    }

    public func recheckInputMonitoringAccess() {
        if inputMonitoringChecker.isInputMonitoringGranted() {
            inputMonitoringWarning = nil
            hotkeyRegistrar.refreshGlobalMonitorIfNeeded()
            stopInputMonitoringPollingIfNeeded()
            return
        }

        guard inputMonitoringWarning != nil,
              !didShowInputMonitoringRestartReminder,
              let shownAt = inputMonitoringWarningShownAt,
              dateProvider().timeIntervalSince(shownAt) >= 3
        else {
            return
        }

        showInputMonitoringRestartReminderIfNeeded()
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
        floatingNotch.toggle()
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
        todayViewModel.stopSessionMirrorPolling()
        await agentSupervisor.shutdown()
        sessionPolling.stopPolling()
        floatingNotch.dismiss()
        sessionHost.dismiss()
    }

    private func handlePhaseChange(_ phase: SessionSurfacePhase) {
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
        guard !notchAndPollingStarted else { return }
        await sessionHost.start()
        floatingNotch.start()
        sessionPolling.startPolling()
        sessionModel.startPolling()
        notchAndPollingStarted = true
        pollingStoppedForUnhealthyAgent = false
        await todayViewModel.load()
        todayViewModel.startSessionMirrorPolling()
    }

    private func syncAgentHealthFromSupervisor() {
        agentHealthWarning = agentSupervisor.currentWarning
        statusItemController.updateAgentStatus(isHealthy: agentSupervisor.isHealthy)
    }

    private func handleAgentHealthChange(isHealthy: Bool) {
        syncAgentHealthFromSupervisor()
        Task { await brokersViewModel.load() }
        Task { await todayViewModel.load() }

        if isHealthy {
            if !notchAndPollingStarted {
                Task { await startNotchAndPolling() }
            } else if pollingStoppedForUnhealthyAgent {
                sessionPolling.startPolling()
                sessionModel.startPolling()
                todayViewModel.startSessionMirrorPolling()
                pollingStoppedForUnhealthyAgent = false
            }
        } else if notchAndPollingStarted {
            sessionPolling.stopPolling()
            sessionModel.stopPolling()
            todayViewModel.stopSessionMirrorPolling()
            pollingStoppedForUnhealthyAgent = true
        }
    }

    private func registerHotkeys() {
        hotkeyRegistrar.registerToggleNotch { [weak self] in
            self?.floatingNotch.toggle()
        }
        hotkeyRegistrar.registerOpenStation { [weak self] in
            self?.openStation()
        }
    }

    private func syncInputMonitoringWarning() {
        // ⌥Space / ⌥⇧Space use Carbon RegisterEventHotKey — no Input Monitoring required.
        // Do not block traders with a yellow banner for a permission hotkeys no longer need.
        inputMonitoringWarning = nil
        inputMonitoringWarningShownAt = nil
        stopInputMonitoringPollingIfNeeded()
    }

    private func showInputMonitoringRestartReminderIfNeeded() {
        guard !didShowInputMonitoringRestartReminder else { return }
        didShowInputMonitoringRestartReminder = true
        inputMonitoringRestartReminder = InputMonitoringWarning.restartReminderMessage
    }

    private func startInputMonitoringPollingIfNeeded() {
        guard inputMonitoringPollTask == nil else { return }
        inputMonitoringPollTask = Task { [weak self] in
            while !Task.isCancelled {
                try? await Task.sleep(for: .seconds(2))
                guard !Task.isCancelled else { return }
                await MainActor.run {
                    self?.recheckInputMonitoringAccess()
                }
            }
        }
    }

    private func stopInputMonitoringPollingIfNeeded() {
        inputMonitoringPollTask?.cancel()
        inputMonitoringPollTask = nil
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

#if DEBUG
        if !launchStore.loginAtBoot {
            windowController.show(orderFrontOnly: true)
        }
#else
        if launchStore.wasWindowVisibleBeforeQuit {
            windowController.show(orderFrontOnly: true)
            return
        }

        if launchStore.loginAtBoot {
            return
        }
#endif
    }
}
