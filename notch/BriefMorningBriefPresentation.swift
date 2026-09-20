import Foundation

// MARK: - #127 — copy + branching for behavioral morning brief (honest empty)

enum BriefMorningBriefPresentation {
    /// Maps API `confidence` strings into UI buckets (#8 mockup 2 — YOUR PATTERNS vs WATCHING vs Improving).
    enum PatternConfidenceBucket: Equatable {
        case established
        case preliminary
        case improving
    }

    /// Threshold referenced in unified reference (M10 not built — ladder to pattern cards).
    static let profileTradeThreshold = 50

    static func patternConfidenceBucket(_ raw: String) -> PatternConfidenceBucket {
        let t = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if t.contains("improv") { return .improving }
        if t.contains("prelim") || t.contains("watch") || t.contains("early") || t == "beta" {
            return .preliminary
        }
        return .established
    }

    static func showsEstablishedBranch(patterns: [BriefBehavioralPattern]) -> Bool {
        !patterns.isEmpty
    }

    /// Wave 2 Open dropped M10 / pre-M10 stub. Keep the helper as a hard no so callers cannot resurrect it.
    static func shouldShowPreM10PatternsStub(isNewUser: Bool, tradeCount: Int, patterns: [BriefBehavioralPattern]) -> Bool {
        _ = (isNewUser, tradeCount, patterns)
        return false
    }

    static func newUserProgressTitle(tradeCount: Int) -> String {
        let remaining = max(0, profileTradeThreshold - tradeCount)
        return "Building your behavioral profile — \(remaining) more trades until patterns appear."
    }

    static func newUserProgressCounter(tradeCount: Int) -> String {
        "\(max(0, tradeCount)) of \(profileTradeThreshold) trades logged."
    }

    static func ownMetricsStopLine(_ m: BriefOwnMetrics) -> String {
        "Stop respected \(m.stopRespected)/\(m.stopTotal) trades"
    }

    static func ownMetricsExitLine(_ m: BriefOwnMetrics) -> String {
        "Exit plan-driven \(m.exitPlanDriven)/\(m.exitTotal) trades"
    }

    static func confidenceBadgeLabel(_ raw: String) -> String {
        let t = raw.trimmingCharacters(in: .whitespacesAndNewlines)
        if t.isEmpty { return "WATCH" }
        return t.uppercased()
    }
}
