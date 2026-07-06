import Testing
@testable import Station

struct BackendBoxRouteTests {
    @Test func backendBoxRouteHasThreeChildRoutes() {
        #expect(BackendBoxRoute.allCases.count == 3)
        #expect(BackendBoxRoute.allCases.contains(.brokers))
        #expect(BackendBoxRoute.allCases.contains(.marketData))
        #expect(BackendBoxRoute.allCases.contains(.aiWorkflow))
    }

    @Test func backendBoxRoutesMapToStationRoutes() {
        #expect(BackendBoxRoute.brokers.stationRoute == .brokers)
        #expect(BackendBoxRoute.marketData.stationRoute == .marketData)
        #expect(BackendBoxRoute.aiWorkflow.stationRoute == .aiWorkflow)
    }
}
