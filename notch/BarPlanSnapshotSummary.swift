import Foundation

struct BarPlanEmotionInSnapshot: Codable, Equatable, Sendable {
    let calm: Double?
    let confidence: Double?
    let frustration: Double?
    let excitement: Double?
}

struct BarPlanInvalidationSnapshot: Codable, Equatable, Sendable {
    let kind: String?
    let price: Double?
    let line: String?
}

/// Frozen Working compare at session end (`plan_snapshot.condition_at_close`, Wave 2).
struct BarPlanConditionAtClose: Codable, Equatable, Sendable {
    let last: Double?
    let lastStatus: String?
    let invalidationState: String?
    let targetState: String?

    enum CodingKeys: String, CodingKey {
        case last
        case lastStatus = "last_status"
        case invalidationState = "invalidation_state"
        case targetState = "target_state"
    }

    var wasCaptured: Bool {
        let status = lastStatus?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        return status != "not_captured" && status != ""
    }
}

/// PLAN SNAPSHOT narrative fields (`plan_snapshot` on `pending_declaration`, #113).
struct BarPlanSnapshotSummary: Codable, Equatable {
    let setupLabel: String?
    let invalidationLine: String?
    let calmScale: Double?
    let confidenceScale: Double?
    let frustrationScale: Double?
    let excitementScale: Double?
    let stance: String?
    let intent: String?
    let invalidationKind: String?
    let invalidationPrice: Double?
    let product: String?
    let targetPrice: Double?
    let symbol: String?
    let side: String?
    let quantity: Double?
    let stopLoss: Double?
    let bookId: String?
    let entryPrice: Double?
    let emotionIn: BarPlanEmotionInSnapshot?
    let invalidation: BarPlanInvalidationSnapshot?
    let conditionAtClose: BarPlanConditionAtClose?

    enum CodingKeys: String, CodingKey {
        case setupLabel = "setup_label"
        case invalidationLine = "invalidation_line"
        case calmScale = "calm_scale"
        case confidenceScale = "confidence_scale"
        case frustrationScale = "frustration_scale"
        case excitementScale = "excitement_scale"
        case stance, intent, product, symbol, side, quantity
        case invalidationKind = "invalidation_kind"
        case invalidationPrice = "invalidation_price"
        case targetPrice = "target_price"
        case stopLoss = "stop_loss"
        case bookId = "book_id"
        case entryPrice = "entry_price"
        case emotionIn = "emotion_in"
        case invalidation
        case conditionAtClose = "condition_at_close"
    }

    var resolvedInvalidationKind: String? {
        let nested = invalidation?.kind?.trimmingCharacters(in: .whitespacesAndNewlines)
        if let nested, !nested.isEmpty { return nested }
        let flat = invalidationKind?.trimmingCharacters(in: .whitespacesAndNewlines)
        return (flat?.isEmpty == false) ? flat : nil
    }

    var resolvedInvalidationPrice: Double? {
        if let p = invalidation?.price, p > 0 { return p }
        if let p = invalidationPrice, p > 0 { return p }
        return nil
    }

    var resolvedInvalidationLine: String {
        let nested = invalidation?.line?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !nested.isEmpty { return nested }
        return invalidationLine?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
    }

    var resolvedCalm: Double? { emotionIn?.calm ?? calmScale }
    var resolvedConfidence: Double? { emotionIn?.confidence ?? confidenceScale }
    var resolvedFrustration: Double? { emotionIn?.frustration ?? frustrationScale }
    var resolvedExcitement: Double? { emotionIn?.excitement ?? excitementScale }
}
