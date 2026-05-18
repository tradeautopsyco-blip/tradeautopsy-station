import Foundation

/// Swing-specific keys merged into `declaration_payload.s1` (Bar v1 allows arbitrary `s1` records).
struct BarSwingDeclarationS1Augmentation: Equatable {
    var acceptsOvernightRisk: Bool
    var dailyCheckInRequired: Bool
    /// Intended holding horizon in trading days (step 3 chips / numeric input).
    var holdingDays: Int?
    /// Max adverse swing drawdown the trader accepts before thesis review, percent (e.g. 12.5).
    var maxDrawdownPct: Double?

    func merged(into s1: [String: Any]) -> [String: Any] {
        var out = s1
        out["accepts_overnight_risk"] = acceptsOvernightRisk
        out["daily_check_in_required"] = dailyCheckInRequired
        if let holdingDays, holdingDays > 0 {
            out["swing_holding_days"] = holdingDays
        }
        if let maxDrawdownPct, maxDrawdownPct > 0 {
            out["swing_max_drawdown_pct"] = maxDrawdownPct
        }
        return out
    }
}
