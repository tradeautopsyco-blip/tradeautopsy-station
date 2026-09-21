import Combine
import Foundation
import Notch

@MainActor
public final class TodayViewModel: ObservableObject {
    @Published public private(set) var presentation: TodayScreenPresentation = .build(
        payload: nil,
        agentHealthy: false
    )
    @Published public private(set) var desk: TodayDeskPresentation = .build(
        payload: nil,
        agentHealthy: false
    )
    @Published public private(set) var isLoading = false
    @Published public private(set) var showShallowImpact = false
    @Published public private(set) var advancedStatisticsSymbol: String?

    private let client: TodayAgentClient
    private let agentHealthy: () -> Bool
    private let isBrokerSyncActive: () -> Bool
    private let configuredSlugs: () -> [String]
    private let sessionModel: SessionModel
    private let sessionMirrorPollIntervalSeconds: TimeInterval
    private let dailyFloor: () -> Double?
    private let demoDeskStore: DemoDeskStore
    private var sessionMirrorPollTask: Task<Void, Never>?
    private var positionsCancellable: AnyCancellable?
    private var deskHonestyCancellable: AnyCancellable?
    private var deskRulesCancellable: AnyCancellable?
    private var demoCancellable: AnyCancellable?
    private var lastPayload: TodayAgentPayload?
    private var demoFixture: StationDemoDesk.Fixture?

    public init(
        client: TodayAgentClient,
        sessionModel: SessionModel,
        agentHealthy: @escaping () -> Bool,
        isBrokerSyncActive: @escaping () -> Bool = { false },
        configuredSlugs: @escaping () -> [String] = { [] },
        sessionMirrorPollIntervalSeconds: TimeInterval = 15,
        dailyFloor: @escaping () -> Double? = { DeskRulesStore.shared.dailyFloor },
        deskRulesStore: DeskRulesStore? = nil,
        demoDeskStore: DemoDeskStore? = nil
    ) {
        self.client = client
        self.sessionModel = sessionModel
        self.agentHealthy = agentHealthy
        self.isBrokerSyncActive = isBrokerSyncActive
        self.configuredSlugs = configuredSlugs
        self.sessionMirrorPollIntervalSeconds = sessionMirrorPollIntervalSeconds
        self.dailyFloor = dailyFloor
        let rules = deskRulesStore ?? .shared
        let demo = demoDeskStore ?? .shared
        self.demoDeskStore = demo
        deskRulesCancellable = rules.$dailyFloor
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.rebuildPresentation()
            }
        positionsCancellable = sessionModel.$positions
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self, self.lastPayload != nil else { return }
                guard !self.demoDeskStore.demoEnabled else { return }
                self.rebuildPresentation()
            }
        deskHonestyCancellable = sessionModel.$activeBrokerSlug
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self, self.lastPayload != nil else { return }
                self.rebuildPresentation()
            }
        demoCancellable = demo.$demoEnabled
            .dropFirst()
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                Task { await self?.load() }
            }
    }

    public var showCircuitBreakerBanner: Bool {
        switch presentation.state {
        case .agentDown:
            return false
        case .syncUnavailable:
            return sessionModel.killSwitchActive && sessionModel.killSwitchStateAgeSecs < 60
        case .healthyEmpty, .healthyActive:
            return sessionModel.killSwitchActive
        }
    }

    public var circuitBreakerCountdownSecs: Int {
        sessionModel.killSwitchCountdownSecs ?? 0
    }

    public var circuitBreakerResumeEnabled: Bool {
        (sessionModel.killSwitchCountdownSecs ?? 0) <= 0 && !sessionModel.killSwitchDismissBusy
    }

    public var sessionPnLUsd: Double? {
        guard presentation.state == .healthyActive else { return nil }
        return lastPayload?.hero.deskClosedPnL
    }

    /// Last known desk quote currency from Today payload (R7).
    public var lastDeskQuoteCurrency: String? {
        lastPayload?.deskQuoteCurrency
    }

    public func load() async {
        isLoading = true
        defer { isLoading = false }
        if demoDeskStore.demoEnabled {
            let fixture = StationDemoDesk.build()
            demoFixture = fixture
            lastPayload = fixture.payload
            rebuildPresentation()
            return
        }
        demoFixture = nil
        lastPayload = await client.fetchToday()
        rebuildPresentation()
    }

    public func toggleShallowImpact() {
        showShallowImpact.toggle()
        rebuildPresentation()
    }

    public func openAdvancedStatistics(symbol: String) {
        advancedStatisticsSymbol = symbol
    }

    private func rebuildPresentation() {
        desk = TodayDeskPresentation.build(
            payload: lastPayload,
            agentHealthy: demoDeskStore.demoEnabled || agentHealthy(),
            positions: presentationPositions(),
            dailyFloor: dailyFloor(),
            showShallowImpact: showShallowImpact,
            showActiveMoney: showsActiveMoney(),
            demoLabeled: demoDeskStore.demoEnabled
        )
        presentation = desk.screen
    }

    private func presentationPositions() -> [DeskPosition] {
        if demoDeskStore.demoEnabled {
            return demoFixture?.positions ?? []
        }
        return sessionModel.positions
    }

    private func showsActiveMoney() -> Bool {
        if demoDeskStore.demoEnabled {
            return true
        }
        let configured = configuredSlugs()
        switch DeskHonesty.resolve(activeSlugs: configured) {
        case .dualNoBlend:
            let active = sessionModel.activeBrokerSlug.map { [$0] } ?? []
            return DeskHonesty.heroQuoteCurrency(activeSlugs: active) != nil
        case .single, .none:
            return true
        }
    }

    public func startSessionMirrorPolling() {
        stopSessionMirrorPolling()
        sessionMirrorPollTask = Task { [sessionMirrorPollIntervalSeconds] in
            while !Task.isCancelled {
                try? await Task.sleep(nanoseconds: UInt64(sessionMirrorPollIntervalSeconds * 1_000_000_000))
                guard !Task.isCancelled else { break }
                if isBrokerSyncActive() {
                    await load()
                }
            }
        }
    }

    public func detectCardInput(declarations: [JournalDeclarationCard] = []) -> DetectCardInput? {
        let decls = demoDeskStore.demoEnabled
            ? (demoFixture?.journalPayload.items ?? declarations)
            : declarations
        return TodayDetectJoin.input(
            position: presentationPositions().first,
            declarations: decls,
            quoteCurrency: lastPayload?.deskQuoteCurrency
        )
    }

    public func stopSessionMirrorPolling() {
        sessionMirrorPollTask?.cancel()
        sessionMirrorPollTask = nil
    }

    public func resumeCircuitBreaker() {
        Task { await sessionModel.dismissKillSwitchFromOverlay() }
    }
}
