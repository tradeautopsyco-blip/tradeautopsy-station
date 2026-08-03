import Foundation

/// Four-metric strip copy + desk-honest money formatting — SwiftUI-free (#113 slice 2 / T2.5).
enum BarLivePlanMetricStripFormatting {
    /// Prefer server declared max loss when present; otherwise fall back to composite worst-case with a honesty prefix.
    /// `formatWholeAbs` must return a full desk-currency string (e.g. `$1,250` / `₹1,250`) — never hardcode INR.
    static func maxLossDeclaredDisplayText(
        declaredMaxLossInr: Double?,
        compositeWorstCaseFallback: Double?,
        formatWholeAbs: (Double) -> String,
    ) -> String {
        if let d = declaredMaxLossInr, d > 0 {
            return formatWholeAbs(d)
        }
        if let w = compositeWorstCaseFallback, w != 0 {
            return "~\(formatWholeAbs(abs(w)))"
        }
        return "—"
    }

    /// Legacy INR number-only formatter path (tests / older call sites).
    static func maxLossDeclaredDisplayText(
        declaredMaxLossInr: Double?,
        compositeWorstCaseFallback: Double?,
        formatWholeInrAbs: (Double) -> String,
    ) -> String {
        maxLossDeclaredDisplayText(
            declaredMaxLossInr: declaredMaxLossInr,
            compositeWorstCaseFallback: compositeWorstCaseFallback,
            formatWholeAbs: { "₹\(formatWholeInrAbs($0))" }
        )
    }

    /// True when the label should read as inferred / proxy (fallback `~` styling in UI optional).
    static func maxLossUsesDeclaredOnly(declaredMaxLossInr: Double?) -> Bool {
        declaredMaxLossInr != nil && declaredMaxLossInr! > 0
    }
}
