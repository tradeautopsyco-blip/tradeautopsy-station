import Foundation
import Testing
@testable import Station

struct NavigationPolicyTests {
    private let stickinessWindow: TimeInterval = 60

    // MARK: - chrome lists

    @Test func sessionRoutesAreTodayOnly() {
        #expect(StationRoute.sessionRoutes == [.today])
    }

    @Test func deskRoutesAreJournalAndSettings() {
        #expect(StationRoute.deskRoutes == [.journal, .settings])
    }

    // MARK: - routeForPhase

    @Test func routeForPhaseDeclarationMapsToToday() {
        #expect(NavigationPolicy.routeForPhase(.declaration) == .today)
    }

    @Test func routeForPhaseArmedMapsToToday() {
        #expect(NavigationPolicy.routeForPhase(.armed) == .today)
    }

    @Test func routeForPhaseLivePlanMapsToToday() {
        #expect(NavigationPolicy.routeForPhase(.livePlan) == .today)
    }

    @Test func routeForPhaseDebriefMapsToToday() {
        #expect(NavigationPolicy.routeForPhase(.debrief) == .today)
    }

    // MARK: - launchRoute

    @Test func launchRouteRestoresSavedDeskRoute() {
        let route = NavigationPolicy.launchRoute(saved: .journal, phase: .declaration)
        #expect(route == .journal)
    }

    @Test func launchRouteIgnoresSavedSessionRoute() {
        let route = NavigationPolicy.launchRoute(saved: .today, phase: .armed)
        #expect(route == .today)
    }

    @Test func launchRouteUsesPhaseWhenSavedIsNil() {
        let route = NavigationPolicy.launchRoute(saved: nil, phase: .debrief)
        #expect(route == .today)
    }

    @Test func leftoverRemovedDeskRawValuesFailClosedToToday() {
        for raw in ["Escrow match", "Patterns", "Fidelity score"] {
            #expect(StationRoute(rawValue: raw) == nil)
            #expect(NavigationPolicy.launchRoute(saved: StationRoute(rawValue: raw), phase: .armed) == .today)
        }
    }

    // MARK: - shouldAutoFollowPhase

    @Test func shouldAutoFollowPhaseDeskActiveReturnsNil() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .settings,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseSessionActiveStaysOnToday() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == nil)
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

    @Test func shouldAutoFollowPhaseManualPickAfter60sStaysOnToday() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let pick = now.addingTimeInterval(-(stickinessWindow + 0.001))
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .armed,
            manualSessionPickAt: pick,
            now: now
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseExactly60sRemainsSticky() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let pick = now.addingTimeInterval(-stickinessWindow)
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .livePlan,
            manualSessionPickAt: pick,
            now: now
        )
        #expect(result == nil)
    }

    @Test func shouldAutoFollowPhaseSamePhaseRouteReturnsNil() {
        let result = NavigationPolicy.shouldAutoFollowPhase(
            active: .today,
            phase: .armed,
            manualSessionPickAt: nil,
            now: Date()
        )
        #expect(result == nil)
    }
}
