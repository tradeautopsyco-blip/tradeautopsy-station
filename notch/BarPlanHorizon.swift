import Foundation

/// Style chip → default horizon days. Product defaults, not a market formula (DTE unknown until 3A).
enum BarPlanHorizon {
    static let dayRange: ClosedRange<Int> = 1 ... 365

    enum Mode: String, Equatable {
        case today
        case threeSessions
        case expiry
        case custom

        var chipLabel: String {
            switch self {
            case .today: return "To close today"
            case .threeSessions: return "3 sessions"
            case .expiry: return "To expiry"
            case .custom: return "Custom"
            }
        }
    }

    /// DTE is unknown until the NFO master exists — do not invent 33.
    static func days(for mode: Mode, dte: Int?) -> Int? {
        switch mode {
        case .today: return 1
        case .threeSessions: return 3
        case .expiry: return dte.flatMap { $0 >= 1 ? $0 : nil }
        case .custom: return nil
        }
    }

    static func mode(forDays days: Int, dte: Int?) -> Mode {
        if days == 1 { return .today }
        if days == 3 { return .threeSessions }
        if let dte, days == dte { return .expiry }
        return .custom
    }

    static func defaultFor(_ kind: String) -> Int {
        switch kind {
        case "intraday", "scalper_session", "pre_market":
            return 1
        case "swing":
            return 5
        case "positional":
            return 20
        default:
            return 1
        }
    }

    static func clamp(_ days: Int) -> Int {
        min(max(days, dayRange.lowerBound), dayRange.upperBound)
    }
}
