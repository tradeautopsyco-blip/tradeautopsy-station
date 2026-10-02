import Combine
import Foundation
import Notch

@MainActor
public final class StationAppCoordinator: ObservableObject {
    @Published public private(set) var activeRoute: StationRoute
    @Published public private(set) var agentHealthWarning: AgentHealthWarning?
    /// The agent is healthy but holds no usable Station session — drives the
    /// status-item nudge and the window banner so a revoked device login is
    /// never silent.
    @Published public private(set) var stationLoginRequired = false
    public let sessionModel: SessionModel
    public let brokersViewModel: BrokersViewModel
    public let marketDataKeysViewModel: MarketDataKeysViewModel
    public let healthPanelViewModel: HealthPanelViewModel
    public let aiWorkflowKeysViewModel: AIWorkflowKeysViewModel
    public let todayViewModel: TodayViewModel
    public let journalViewModel: JournalViewModel
    public let deviceLoginViewModel: DeviceLoginViewModel
    @Published public private(set) var inputMonitoringWarning: InputMonitoringWarning?
    @Published public private(set) var inputMonitoringRestartReminder: String?
    @Published public private(set) var launchAtLoginEnabled = false
    @Published public private(set) var showLoginItemPrompt = false

    public var automaticallyChecksForUpdates: Bool {
        softwareUpdateController.automaticallyChecksForUpdates
    }

    public var automaticallyDownloadsUpdates: Bool {
        softwareUpdateController.automaticallyDownloadsUpdates
    }

    public var canCheckForSoftwareUpdates: Bool {
        softwareUpdateController.canCheckForUpdates
    }

    /// Shell navigation stays enabled even when the agent is unhealthy (TRD §6.3).
    public var isShellNavigable: Bool { true }

    /// The single onboarding notification to show, if any (agent health has its own
    /// permanent home in the toolbar and isn't part of this).
    public var currentNotification: StationNotification? {
        StationNotification.current(
            stationLoginRequired: stationLoginRequired,
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
    private let softwareUpdateController: SoftwareUpdateControlling
    private let phaseProvider: SessionSurfacePhaseProviding
    private let deskRouteStore: DeskRouteStoring
    public let deskRulesStore: DeskRulesStore
    public let demoDeskStore: DemoDeskStore
    private let dateProvider: () -> Date

    private var manualSessionPickAt: Date?
    private var notchAndPollingStarted = false
    private var pollingStoppedForUnhealthyAgent = false
    private var stationAuthPollTask: Task<Void, Never>?
    private var phaseObservers = Set<AnyCancellable>()
    private var inputMonitoringWarningShownAt: Date?
    private var inputMonitoringPollTask: Task<Void, Never>?
    private var didShowInputMonitoringRestartReminder = false
    private var hotkeyPrefsObserver: NSObjectProtocol?
    private let hotkeyPrefsRelay = HotkeyPrefsReloadRelay()

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
        softwareUpdateController: SoftwareUpdateControlling = NoOpSoftwareUpdateController(),
        sessionModel: SessionModel? = nil,
        deskRouteStore: DeskRouteStoring = UserDefaultsDeskRouteStore(),
        dateProvider: @escaping () -> Date = Date.init,
        inputMonitoringChecker: InputMonitoringChecking = DefaultInputMonitoringChecker(),
        brokerControl: BrokerControlling? = nil,
        todayClient: TodayAgentClient? = nil,
        journalClient: JournalAgentClient? = nil,
        daemonSecret: String? = nil,
        deviceLoginClient: (any DeviceLoginClient)? = nil,
        deskRulesStore: DeskRulesStore? = nil,
        demoDeskStore: DemoDeskStore? = nil,
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
        self.softwareUpdateController = softwareUpdateController
        self.phaseProvider = phaseProvider
        self.deskRouteStore = deskRouteStore
        self.deskRulesStore = deskRulesStore ?? .shared
        self.demoDeskStore = demoDeskStore ?? .shared
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
        self.marketDataKeysViewModel = MarketDataKeysViewModel(
            bindingClient: LocalVendorBindingClient(daemonSecret: resolvedDaemonSecret)
        )
        let session = self.sessionModel
        self.healthPanelViewModel = HealthPanelViewModel(
            daemonSecret: resolvedDaemonSecret,
            agentHealthy: { agentSupervisor.isHealthy },
            agentMessage: { agentSupervisor.isHealthy ? nil : "Agent unavailable" },
            killActive: { session.killSwitchActive }
        )
        self.aiWorkflowKeysViewModel = AIWorkflowKeysViewModel()

        let resolvedTodayClient = todayClient ?? LocalTodayAgentClient(
            daemonSecret: resolvedDaemonSecret,
            isAgentHealthy: { agentSupervisor.isHealthy }
        )
        let brokers = self.brokersViewModel
        self.todayViewModel = TodayViewModel(
            client: resolvedTodayClient,
            sessionModel: session,
            agentHealthy: { agentSupervisor.isHealthy },
            isBrokerSyncActive: { session.isBrokerSyncActiveForTodayMirror },
            configuredSlugs: { brokers.configuredBrokerSlugs },
            dailyFloor: { [deskRules = self.deskRulesStore] in deskRules.dailyFloor },
            deskRulesStore: self.deskRulesStore,
            demoDeskStore: self.demoDeskStore
        )
        let resolvedJournalClient = journalClient ?? LocalJournalAgentClient(
            daemonSecret: resolvedDaemonSecret,
            isAgentHealthy: { agentSupervisor.isHealthy }
        )
        self.journalViewModel = JournalViewModel(
            client: resolvedJournalClient,
            sessionModel: session,
            agentHealthy: { agentSupervisor.isHealthy },
            demoDeskStore: self.demoDeskStore
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
        deviceLoginViewModel.$phase
            .sink { [weak self] _ in self?.syncStationLoginRequired() }
            .store(in: &phaseObservers)
        hotkeyPrefsRelay.coordinator = self
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
        navigateTo(.today)
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

    public func setAutomaticallyChecksForUpdates(_ enabled: Bool) {
        softwareUpdateController.automaticallyChecksForUpdates = enabled
        objectWillChange.send()
    }

    public func setAutomaticallyDownloadsUpdates(_ enabled: Bool) {
        softwareUpdateController.automaticallyDownloadsUpdates = enabled
        objectWillChange.send()
    }

    public func checkForSoftwareUpdates() {
        softwareUpdateController.checkForUpdates(nil)
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

    /// Status-item / banner entry point: bring the window up and land on the
    /// device-login section of Settings.
    public func openStationForDeviceLogin() {
        openStation()
        navigateTo(.settings)
    }

    public func toggleNotch() {
        floatingNotch.toggle()
    }

    public func setHideNotch(_ hidden: Bool) {
        // Preference is retained so older saved values stay readable. Visibility is
        // shortcut-only: the closed pill is never shown.
        deskRulesStore.setHideNotch(hidden)
    }

    public func setDemoEnabled(_ enabled: Bool) {
        demoDeskStore.setEnabled(enabled)
        Task {
            await todayViewModel.load()
            await journalViewModel.load()
        }
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

    /// One-button agent restart (Wave 6). Does not SIGKILL when kill latch is active.
    public func restartAgentProcess() async {
        let outcome = await agentSupervisor.restartAgent()
        switch outcome {
        case .blockedKillLatched:
            agentHealthWarning = AgentHealthWarning(
                reason: .killSwitchLatched,
                message: "Cannot restart the agent while the Enforcer is holding a kill. Dismiss the kill switch first.",
                logPath: nil,
                canRetry: false
            )
            syncAgentHealthFromSupervisor()
        case .restarted:
            syncAgentHealthFromSupervisor()
            if agentSupervisor.isHealthy {
                await startNotchAndPolling()
            }
        }
    }

    public func quit() async {
        if windowController.isVisible {
            windowController.hide()
        }
        if let hotkeyPrefsObserver {
            NotificationCenter.default.removeObserver(hotkeyPrefsObserver)
            self.hotkeyPrefsObserver = nil
        }
        hotkeyPrefsRelay.coordinator = nil
        hotkeyRegistrar.unregisterAll()
        todayViewModel.stopSessionMirrorPolling()
        await agentSupervisor.shutdown()
        sessionPolling.stopPolling()
        stopStationAuthPolling()
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
        startStationAuthPolling()
        notchAndPollingStarted = true
        pollingStoppedForUnhealthyAgent = false
        await todayViewModel.load()
        await journalViewModel.load()
        todayViewModel.startSessionMirrorPolling()
    }

    private func syncAgentHealthFromSupervisor() {
        agentHealthWarning = agentSupervisor.currentWarning
        statusItemController.updateAgentStatus(isHealthy: agentSupervisor.isHealthy)
        syncStationLoginRequired()
    }

    /// The agent quietly refreshes the Keychain session in the background; when
    /// the refresh family is burned, nothing else looks at the session endpoint
    /// unless Settings is open. Poll it here so a revoked login surfaces in the
    /// status item and window banner within a minute instead of silently.
    private func startStationAuthPolling() {
        guard stationAuthPollTask == nil else { return }
        stationAuthPollTask = Task { [weak self] in
            while !Task.isCancelled {
                await self?.deviceLoginViewModel.refreshSession()
                try? await Task.sleep(for: .seconds(60))
            }
        }
    }

    private func stopStationAuthPolling() {
        stationAuthPollTask?.cancel()
        stationAuthPollTask = nil
    }

    private func syncStationLoginRequired() {
        // Only nag when the agent can actually answer — an unhealthy agent
        // already owns the red tint and its own warning surface.
        let required = agentSupervisor.isHealthy && !deviceLoginViewModel.isSessionEstablished
        if stationLoginRequired != required {
            stationLoginRequired = required
        }
        statusItemController.updateStationLoginRequired(required)
    }

    private func handleAgentHealthChange(isHealthy: Bool) {
        syncAgentHealthFromSupervisor()
        Task { await brokersViewModel.load() }
        Task { await todayViewModel.load() }
        Task { await journalViewModel.load() }

        if isHealthy {
            if !notchAndPollingStarted {
                Task { await startNotchAndPolling() }
            } else if pollingStoppedForUnhealthyAgent {
                sessionPolling.startPolling()
                sessionModel.startPolling()
                todayViewModel.startSessionMirrorPolling()
                startStationAuthPolling()
                pollingStoppedForUnhealthyAgent = false
            }
        } else if notchAndPollingStarted {
            sessionPolling.stopPolling()
            sessionModel.stopPolling()
            todayViewModel.stopSessionMirrorPolling()
            stopStationAuthPolling()
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
        hotkeyRegistrar.registerDeskActions { [weak self] actionId in
            self?.floatingNotch.performHotkey(actionId)
        }
        reloadHotkeyBindings()
        guard hotkeyPrefsObserver == nil else { return }
        hotkeyPrefsObserver = NotificationCenter.default.addObserver(
            forName: DeskHotkeyPreferences.didSaveNotification,
            object: nil,
            queue: .main
        ) { [relay = hotkeyPrefsRelay] _ in
            relay.reload()
        }
    }

    fileprivate func reloadHotkeyBindings() {
        let bindings = DeskHotkeyPreferences.load().map {
            DeskHotkeyRegistration(
                actionId: $0.actionId,
                keyCode: $0.keyCode,
                carbonModifiers: $0.carbonModifiers
            )
        }
        hotkeyRegistrar.reloadSavedBindings(bindings)
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

/// Notification callbacks are `@Sendable`. This box hops back to the main actor without capturing the coordinator in that closure.
private final class HotkeyPrefsReloadRelay: @unchecked Sendable {
    weak var coordinator: StationAppCoordinator?

    func reload() {
        let coordinator = self.coordinator
        if Thread.isMainThread {
            MainActor.assumeIsolated {
                coordinator?.reloadHotkeyBindings()
            }
        } else {
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    coordinator?.reloadHotkeyBindings()
                }
            }
        }
    }
}
