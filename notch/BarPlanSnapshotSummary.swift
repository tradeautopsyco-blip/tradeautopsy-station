import Foundation

/// PLAN SNAPSHOT narrative fields (`plan_snapshot` on `pending_declaration`, #113).
struct BarPlanSnapshotSummary: Codable, Equatable {
    let setupLabel: String?
    let invalidationLine: String?
    let calmScale: Double?
    let confidenceScale: Double?

    enum CodingKeys: String, CodingKey {
        case setupLabel = "setup_label"
        case invalidationLine = "invalidation_line"
        case calmScale = "calm_scale"
        case confidenceScale = "confidence_scale"
    }
}
