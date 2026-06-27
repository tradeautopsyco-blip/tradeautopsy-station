import Foundation

// MARK: - #124 — when Notch arms post-trade debrief (testable rules)

/// Inputs that can arm **`barDebriefPending`** — documented for `#124` (position poll vs explicit exit signal).
enum BarDebriefArmingEvent: Equatable {
    /// From `GET /api/daemon/positions` polling when the open-position count drops to zero after being positive.
    case positionsPollTransition(previousCount: Int, newCount: Int)
    /// Reserved for SSE / agent when a trade-exit event is emitted (explicit path).
    case explicitTradeExitSignal
}

enum BarDebriefArming {
    /// Apply **position poll** transition. Opening a new position always clears pending.
    static func applyPoll(previousCount: Int, newCount: Int, pending: Bool) -> Bool {
        if newCount > 0 { return false }
        if previousCount > 0, newCount == 0 { return true }
        return pending
    }

    /// Explicit **trade exit** signal always requests debrief (agent/SSE path).
    static func applyExplicitTradeExit() -> Bool {
        true
    }

    static func apply(event: BarDebriefArmingEvent, pending: Bool) -> Bool {
        switch event {
        case let .positionsPollTransition(prev, new):
            applyPoll(previousCount: prev, newCount: new, pending: pending)
        case .explicitTradeExitSignal:
            applyExplicitTradeExit()
        }
    }
}
