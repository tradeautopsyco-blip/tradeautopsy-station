import Foundation

/// Invalidation taxonomy for intraday Bar (#121) — maps to chips in unified reference mockups 3–4.
enum BarInvalidationKind: String, Equatable, CaseIterable, Identifiable {
    case time
    case behaviour
    case context

    var id: String { rawValue }

    var chipTitle: String {
        switch self {
        case .time: return "Time"
        case .behaviour: return "Behaviour"
        case .context: return "Context"
        }
    }

    /// Distinct hint line under the invalidation textarea (pure helper — tested without SwiftUI).
    static func textareaPlaceholderHint(for kind: BarInvalidationKind?) -> String {
        guard let kind else {
            return "Describe invalidation condition…"
        }
        switch kind {
        case .time:
            return "E.g. time-box exit, session cut-off, or minutes until thesis expires…"
        case .behaviour:
            return "E.g. price/structure behaviour that voids the setup (VWAP, swing low, news)…"
        case .context:
            return "E.g. broader context shift (index, sector, volatility regime) that voids the trade…"
        }
    }
}
