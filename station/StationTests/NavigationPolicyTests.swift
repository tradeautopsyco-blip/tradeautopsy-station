import Foundation
import Notch
import Testing
@testable import Station

struct NavigationPolicyTests {
    private let stickinessWindow: TimeInterval = 60

    // MARK: - routeForPhase

    @Test func routeForPhaseDeclarationMapsToPreTrade() {
        #expect(NavigationPolicy.routeForPhase(.declaration) == .preTrade)
    }

    @Test func routeForPhaseArmedMapsToLiveTrade() {
        #expect(NavigationPolicy.routeForPhase(.armed) == .liveTrade)
    }

    @Test func routeForPhaseLivePlanMapsToLiveTrade() {
        #expect(NavigationPolicy.routeForPhase(.livePlan) == .liveTrade)
    }

    @Test func routeForPhaseDebriefMapsToPostTrade() {
        #expect(NavigationPolicy.routeForPhase(.debrief) == .postTrade)
    }

    // MARK: - launchRoute

    @Test func launchRouteRestoresSavedDeskRoute() {
        let route = NavigationPolicy.launchRoute(saved: .journal, phase: .declaration)
        #expect(route == .journal)
    }

    @Test func launchRouteIgnoresSavedSessionRoute() {
        let route = NavigationPolicy.launchRoute(saved: .today, phase: .armed)
        #expect(route == .liveTrade)
    }

    @Test func launchRouteUsesPhaseWhenSavedIsNil() {
        let route = NavigationPolicy.launchRoute(saved: nil, phase: .debrief)
        #expect(route == .postTrade)
    }

    // MARK: - shouldAutoFollowPhase

    @Test func shouldAutoFollowPhaseDeskActiveReturnsNil() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .patterns,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseSessionActiveNoManualPickReturnsPhaseRoute() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .preTrade,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == .liveTrade)
    }

    @Test func shouldAutoFollowPhaseManualPickWithin60sReturnsNil() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let pick = now.addingTimeInterval(-30)
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .armed,
            manualSessionPickAt: pick,
            now: now
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseManualPickAfter60sReturnsPhaseRoute() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let pick = now.addingTimeInterval(-(stickinessWindow + 0.001))
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .armed,
            manualSessionPickAt: pick,
            now: now
        )
        #expect(result == .liveTrade)
    }

    @Test func shouldAutoFollowPhaseExactly60sRemainsSticky() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let pick = now.addingTimeInterval(-stickinessWindow)
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .preTrade,
            phase: .livePlan,
            manualSessionPickAt: pick,
            now: now
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseSamePhaseRouteReturnsNil() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .liveTrade,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == nil)
    }
}
