import Foundation

/// Session surface phase for Station navigation (declaration → armed → live → debrief).
public enum SessionSurfacePhase: String {
    case livePlan
    case armed
    case declaration
    case debrief
}
