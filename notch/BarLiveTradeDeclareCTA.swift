import Foundation

/// Whether Live trade shows the “Declare before trade” CTA vs armed / pending state (#179).
enum BarLiveTradeDeclareCTA {
    static func shouldShow(
        surfacePhase: BarSurfacePhase,
        showingDeclarationForm: Bool,
        matchedDeclarationId: String?,
        hasPendingDeclaration: Bool,
        hasOptimisticArmed: Bool,
    ) -> Bool {
        if showingDeclarationForm { return false }
        if surfacePhase == .armed { return false }
        if hasOptimisticArmed { return false }
        if hasPendingDeclaration { return false }
        let trimmed = matchedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return trimmed.isEmpty
    }
}
