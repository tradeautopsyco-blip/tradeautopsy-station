import Foundation

@MainActor
public final class TodayViewModel: ObservableObject {
    @Published public private(set) var presentation: TodayScreenPresentation = .build(
        payload: nil,
        agentHealthy: false
    )
    @Published public private(set) var isLoading = false

    private let client: TodayAgentClient
    private let agentHealthy: () -> Bool
    private let isBrokerSyncActive: () -> Bool
    private let sessionModel: SessionModel
    private let sessionMirrorPollIntervalSeconds: TimeInterval
    private var sessionMirrorPollTask: Task<Void, Never>?

    public init(
        client: TodayAgentClient,
        sessionModel: SessionModel,
        agentHealthy: @escaping () -> Bool,
        isBrokerSyncActive: @escaping () -> Bool = { false },
        sessionMirrorPollIntervalSeconds: TimeInterval = 15
    ) {
        self.client = client
        self.sessionModel = sessionModel
        self.agentHealthy = agentHealthy
        self.isBrokerSyncActive = isBrokerSyncActive
        self.sessionMirrorPollIntervalSeconds = sessionMirrorPollIntervalSeconds
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
        presentation = TodayScreenPresentation.build(
            payload: payload,
            agentHealthy: agentHealthy()
        )
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

    public func stopSessionMirrorPolling() {
        sessionMirrorPollTask?.cancel()
        sessionMirrorPollTask = nil
    }

    public func resumeCircuitBreaker() {
        Task { await sessionModel.dismissKillSwitchFromOverlay() }
    }
}
