import Notch
import Testing
@testable import Station

struct StationRouteBarNotchScreenTests {
    @Test func allBarNotchScreensMapToExpectedRoutes() {
        let mappings: [(BarNotchScreen, StationRoute)] = [
            (.morning, .today),
            (.pretrade, .preTrade),
            (.live, .liveTrade),
            (.posttrade, .postTrade),
            (.escrow, .escrowMatch),
            (.patterns, .patterns),
            (.fidelity, .fidelityScore),
            (.triage, .journal),
            (.settings, .settings),
        ]

        for (screen, expected) in mappings {
            #expect(StationRoute(barNotchScreen: screen) == expected)
        }
    }

    @Test func allBarNotchScreensRoundTripThroughStationRoute() {
        for screen in BarNotchScreen.allCases {
            guard let route = StationRoute(barNotchScreen: screen) else {
                Issue.record("Expected route for \(screen)")
                continue
            }
            #expect(BarNotchScreen(stationRoute: route) == screen)
        }
    }

    @Test func brokersRouteHasNoBarNotchScreenEquivalent() {
        #expect(BarNotchScreen(stationRoute: .brokers) == nil)
    }

    @Test func journalRouteMapsToTriageScreen() {
        #expect(BarNotchScreen(stationRoute: .journal) == .triage)
    }
}
