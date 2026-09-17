import Testing
@testable import Station

struct BackendBoxRouteTests {
    @Test func backendBoxRouteHasFourChildRoutes() {
        #expect(BackendBoxRoute.allCases.count == 4)
        #expect(BackendBoxRoute.allCases.contains(.brokers))
        #expect(BackendBoxRoute.allCases.contains(.health))
        #expect(BackendBoxRoute.allCases.contains(.marketData))
        #expect(BackendBoxRoute.allCases.contains(.aiWorkflow))
    }

    @Test func backendBoxRoutesMapToStationRoutes() {
        #expect(BackendBoxRoute.brokers.stationRoute == .brokers)
        #expect(BackendBoxRoute.health.stationRoute == .health)
        #expect(BackendBoxRoute.marketData.stationRoute == .marketData)
        #expect(BackendBoxRoute.aiWorkflow.stationRoute == .aiWorkflow)
    }

    @Test func stationSidebarBackendBoxListsHealth() {
        #expect(StationRoute.backendBoxRoutes.contains(.health))
        #expect(StationRoute.deskRoutes.contains(.health) == false)
    }
}
