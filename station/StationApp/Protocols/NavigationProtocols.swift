import Foundation
import Notch

@MainActor
public protocol BarSurfacePhaseProviding: AnyObject {
    var barSurfacePhase: BarSurfacePhase { get }
    var onPhaseChange: ((BarSurfacePhase) -> Void)? { get set }
}

@MainActor
public protocol DeskRouteStoring: AnyObject {
    var savedDeskRoute: StationRoute? { get }
    func saveDeskRoute(_ route: StationRoute)
}
