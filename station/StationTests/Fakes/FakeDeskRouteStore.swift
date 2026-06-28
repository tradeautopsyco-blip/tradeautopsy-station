import Foundation
@testable import Station

@MainActor
final class FakeDeskRouteStore: DeskRouteStoring {
    private(set) var savedDeskRoute: StationRoute?
    private(set) var saveCallCount = 0

    func saveDeskRoute(_ route: StationRoute) {
        saveCallCount += 1
        savedDeskRoute = route
    }
}
