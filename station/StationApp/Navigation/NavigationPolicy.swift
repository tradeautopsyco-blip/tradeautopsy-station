import Foundation
import Notch

public enum NavigationPolicy {
    public static let sessionStickinessSeconds: TimeInterval = 60

    /// Map `BarSurfacePhase` → default Session route.
    public static func routeForPhase(_ phase: BarSurfacePhase) -> StationRoute {
        switch phase {
        case .declaration: return .preTrade
        case .armed, .livePlan: return .liveTrade
        case .debrief: return .postTrade
        }
    }

    /// Launch route: restore Desk if saved; else phase-derived Session route.
    public static func launchRoute(saved: StationRoute?, phase: BarSurfacePhase) -> StationRoute {
        if let saved, saved.isDesk {
            return saved
        }
        return routeForPhase(phase)
    }

    /// Returns new route if auto-follow should apply; nil = no change.
    public static func shouldAutoFollowPhase(
        active: StationRoute,
        phase: BarSurfacePhase,
        manualSessionPickAt: Date?,
        now: Date
    ) -> StationRoute? {
        guard active.isSession else { return nil }

        if let manualSessionPickAt {
            let elapsed = now.timeIntervalSince(manualSessionPickAt)
            if elapsed <= sessionStickinessSeconds {
                return nil
            }
        }

        let phaseRoute = routeForPhase(phase)
        return phaseRoute == active ? nil : phaseRoute
    }
}
