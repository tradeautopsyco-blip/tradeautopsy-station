import Foundation

/// Confirm stance — reactive is allowed; Journal tags it impulsive-adjacent.
enum BarPlanStance: String, Equatable, CaseIterable, Identifiable, Sendable {
    case planned
    case reactive

    var id: String { rawValue }

    var chipTitle: String {
        switch self {
        case .planned: return "Planned"
        case .reactive: return "Reactive"
        }
    }
}
