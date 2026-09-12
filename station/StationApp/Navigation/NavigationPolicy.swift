import Foundation

public enum NavigationPolicy {
    public static let sessionStickinessSeconds: TimeInterval = 60

    /// Map `SessionSurfacePhase` → default Session route.
    public static func routeForPhase(_ phase: SessionSurfacePhase) -> StationRoute {
        switch phase {
        case .declaration, .armed, .livePlan, .debrief:
            return .today
        }
    }

    /// Launch route: restore Desk if saved; else phase-derived Session route.
    public static func launchRoute(saved: StationRoute?, phase: SessionSurfacePhase) -> StationRoute {
        if let saved, saved.isDesk {
            return saved
        }
        return routeForPhase(phase)
    }

    /// Returns new route if auto-follow should apply; nil = no change.
    public static func shouldAutoFollowPhase(
        active: StationRoute,
        phase: SessionSurfacePhase,
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
