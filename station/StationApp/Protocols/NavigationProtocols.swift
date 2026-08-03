import Foundation

@MainActor
public protocol SessionSurfacePhaseProviding: AnyObject {
    var sessionSurfacePhase: SessionSurfacePhase { get }
    var onPhaseChange: ((SessionSurfacePhase) -> Void)? { get set }
}

@MainActor
public protocol DeskRouteStoring: AnyObject {
    var savedDeskRoute: StationRoute? { get }
    func saveDeskRoute(_ route: StationRoute)
}
