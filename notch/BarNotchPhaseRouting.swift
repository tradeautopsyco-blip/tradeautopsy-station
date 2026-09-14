public enum BarNotchPhaseRouting {
    public static func screen(for phase: BarSurfacePhase) -> BarNotchScreen {
        switch phase {
        case .declaration: return .pretrade
        case .livePlan, .armed: return .live
        case .debrief: return .posttrade
        }
    }
}
