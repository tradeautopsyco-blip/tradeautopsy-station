import Foundation

/// Hydrated daemon `modify_sl` / `cancel_sl` **`existing`** record from notch live-state (#113 slice 6).
struct BarProtectiveExistingSlSnapshot: Codable, Equatable {
    let brokerOrderId: String
    let declarationId: String
    let positionKey: String
    let triggerPrice: Double
    let limitPrice: Double
    let protectedQty: Double
    let placementMode: String
    let placedAtMs: Double
    let status: String
    let userId: String
    let broker: String
    let declaredStop: Double?
    let escrowFlag: Bool?
}
