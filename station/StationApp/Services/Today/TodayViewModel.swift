import Foundation
import Notch

@MainActor
public final class TodayViewModel: ObservableObject {
    @Published public private(set) var presentation: TodayScreenPresentation = .build(
        payload: nil,
        agentHealthy: false
    )
    @Published public private(set) var isLoading = false

    private let client: TodayAgentClient
    private let agentHealthy: () -> Bool
    private let notchViewModel: NotchViewModel

    public init(
        client: TodayAgentClient,
        notchViewModel: NotchViewModel,
        agentHealthy: @escaping () -> Bool
    ) {
        self.client = client
        self.notchViewModel = notchViewModel
        self.agentHealthy = agentHealthy
    }

    public var showCircuitBreakerBanner: Bool {
        switch presentation.state {
        case .agentDown:
            return false
        case .syncUnavailable:
            return notchViewModel.killSwitchActive && notchViewModel.killSwitchStateAgeSecs < 60
        case .healthyEmpty, .healthyActive:
            return notchViewModel.killSwitchActive
        }
    }

    public var circuitBreakerCountdownSecs: Int {
        notchViewModel.killSwitchCountdownSecs ?? 0
    }

    public var circuitBreakerResumeEnabled: Bool {
        (notchViewModel.killSwitchCountdownSecs ?? 0) <= 0 && !notchViewModel.killSwitchDismissBusy
    }

    public var sessionPnLUsd: Double? {
        guard presentation.state == .healthyActive else { return nil }
        return lastPayload?.hero.pnlTodayUsd
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

    public func resumeCircuitBreaker() {
        Task { await notchViewModel.dismissKillSwitchFromOverlay() }
    }
}
