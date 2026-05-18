import Foundation

/// Four-metric strip copy + INR formatting — SwiftUI-free (#113 slice 2).
enum BarLivePlanMetricStripFormatting {
    /// Prefer server **declared_max_loss_inr** when present (qty × |entry − stop|); otherwise fall back to composite worst-case ₹ with a honesty prefix.
    static func maxLossDeclaredDisplayText(
        declaredMaxLossInr: Double?,
        compositeWorstCaseFallback: Double?,
        formatWholeInrAbs: (Double) -> String,
    ) -> String {
        if let d = declaredMaxLossInr, d > 0 {
            return "₹\(formatWholeInrAbs(d))"
        }
        if let w = compositeWorstCaseFallback, w != 0 {
            return "~₹\(formatWholeInrAbs(abs(w)))"
        }
        return "—"
    }

    /// True when the label should read as inferred / proxy (fallback `~` styling in UI optional).
    static func maxLossUsesDeclaredOnly(declaredMaxLossInr: Double?) -> Bool {
        declaredMaxLossInr != nil && declaredMaxLossInr! > 0
    }
}
