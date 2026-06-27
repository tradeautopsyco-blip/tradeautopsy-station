import Foundation

/// Pure strings for Notch PLAN SNAPSHOT rows — no SwiftUI.
enum BarLivePlanSnapshotMapping {
    enum StopRowTone: Equatable {
        case placedGreen
        case missingRed
        case neutral
    }

    static func setupDisplay(plan: BarPlanSnapshotSummary?) -> String {
        let t = plan?.setupLabel?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return t.isEmpty ? "—" : t
    }

    static func invalidationDisplay(plan: BarPlanSnapshotSummary?) -> String {
        let t = plan?.invalidationLine?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return t.isEmpty ? "—" : t
    }

    /// Entry state line: `Calm 2 · Conf 4` (Mockup 5).
    static func entryStateDisplay(plan: BarPlanSnapshotSummary?) -> String {
        guard let plan else { return "—" }
        let calm = plan.calmScale
        let conf = plan.confidenceScale
        if calm == nil && conf == nil { return "—" }
        if let c = calm, let f = conf {
            let ci = Int(c.rounded())
            let fi = Int(f.rounded())
            return "Calm \(ci) · Conf \(fi)"
        }
        if let c = calm {
            return "Calm \(Int(c.rounded()))"
        }
        if let f = conf {
            return "Conf \(Int(f.rounded()))"
        }
        return "—"
    }

    /// Right-hand value for the Stop loss row (price + semantic suffix).
    static func stopLossValueDisplay(
        slStatus: String?,
        formattedPrice: String,
    ) -> (text: String, tone: StopRowTone) {
        let st = slStatus?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        if st == "placed" {
            if formattedPrice == "—" {
                return ("placed", .placedGreen)
            }
            return ("\(formattedPrice) — holding", .placedGreen)
        }
        if st == "missing" {
            return ("— missing", .missingRed)
        }
        if formattedPrice != "—" {
            return (formattedPrice, .neutral)
        }
        return ("—", .neutral)
    }

    static func targetDisplay(pendingTarget: Double?, format: (Double) -> String) -> String {
        guard let t = pendingTarget, t > 0 else { return "—" }
        return format(t)
    }
}
