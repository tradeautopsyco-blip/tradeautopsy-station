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
    private var sessionMirrorPollTask: Task<Void, Never>?
    private var positionsCancellable: AnyCancellable?
    private var deskHonestyCancellable: AnyCancellable?
    private var deskRulesCancellable: AnyCancellable?

    public init(
        client: TodayAgentClient,
        sessionModel: SessionModel,
        agentHealthy: @escaping () -> Bool,
        isBrokerSyncActive: @escaping () -> Bool = { false },
        configuredSlugs: @escaping () -> [String] = { [] },
        sessionMirrorPollIntervalSeconds: TimeInterval = 15,
        dailyFloor: @escaping () -> Double? = { DeskRulesStore.shared.dailyFloor },
        deskRulesStore: DeskRulesStore? = nil
    ) {
        self.client = client
        self.sessionModel = sessionModel
        self.agentHealthy = agentHealthy
        self.isBrokerSyncActive = isBrokerSyncActive
        self.configuredSlugs = configuredSlugs
        self.sessionMirrorPollIntervalSeconds = sessionMirrorPollIntervalSeconds
        self.dailyFloor = dailyFloor
        let rules = deskRulesStore ?? .shared
        deskRulesCancellable = rules.$dailyFloor
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                self?.rebuildPresentation()
            }
        positionsCancellable = sessionModel.$positions
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self, self.lastPayload != nil else { return }
                self.rebuildPresentation()
            }
        deskHonestyCancellable = sessionModel.$activeBrokerSlug
            .receive(on: RunLoop.main)
            .sink { [weak self] _ in
                guard let self, self.lastPayload != nil else { return }
                self.rebuildPresentation()
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
        return lastPayload?.hero.pnlTodayUsd
    }

    /// Last known desk quote currency from Today payload (R7).
    public var lastDeskQuoteCurrency: String? {
        lastPayload?.deskQuoteCurrency
    }

    private var lastPayload: TodayAgentPayload?

    public func load() async {
        isLoading = true
        defer { isLoading = false }
        let payload = await client.fetchToday()
        lastPayload = payload
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
            agentHealthy: agentHealthy(),
            positions: sessionModel.positions,
            dailyFloor: dailyFloor(),
            showShallowImpact: showShallowImpact,
            showActiveMoney: showsActiveMoney()
        )
        presentation = desk.screen
    }

    private func showsActiveMoney() -> Bool {
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
        TodayDetectJoin.input(
            position: sessionModel.positions.first,
            declarations: declarations,
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
