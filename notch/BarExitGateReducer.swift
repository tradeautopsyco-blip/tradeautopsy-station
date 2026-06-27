import Foundation

// MARK: - #123 — Exit gate FSM (pure reducer + countable terminal paths)

/// Three sequential yes/no gates. **Cannot** answer out of order (`apply` is a no-op).
struct BarExitGateState: Equatable {
    var q1: Bool?
    var q2: Bool?
    var q3: Bool?
    var flow: BarExitGateFlow

    static let initial = BarExitGateState(q1: nil, q2: nil, q3: nil, flow: .questions)
}

enum BarExitGateFlow: Equatable {
    case questions
    /// Single adverse-candle path — user must read mirror before cool-off (#123).
    case scareCandleMirror
    /// All attestations yes — proceed to broker exit tooling immediately.
    case cleanExitReady
    /// Verdict + optional chip + mandatory countdown (seconds remaining).
    case coolOffVerdict(countdown: Int, chip: String?)
}

enum BarExitGateAction: Equatable {
    /// Answer question index `0...2`. Ignored if not the active next slot or flow is not `.questions`.
    case answerQuestion(index: Int, value: Bool)
    case selectVerdictChip(String)
    /// User finished reading scare-candle mirror → mandatory cool-off.
    case acknowledgeScareMirror
    /// Countdown tick (e.g. 1 Hz timer). No-op when not in `.coolOffVerdict`.
    case tickCountdown
}

enum BarExitGateReducer {
    static func reduce(state: BarExitGateState, action: BarExitGateAction) -> BarExitGateState {
        var next = state
        switch action {
        case let .answerQuestion(index, value):
            guard next.flow == .questions else { return state }
            switch index {
            case 0:
                guard next.q1 == nil else { return state }
                next.q1 = value
            case 1:
                guard next.q1 != nil, next.q2 == nil else { return state }
                next.q2 = value
            case 2:
                guard next.q2 != nil, next.q3 == nil else { return state }
                next.q3 = value
                next = resolveTerminal(after: next)
            default:
                return state
            }
        case let .selectVerdictChip(chip):
            guard case let .coolOffVerdict(c, _) = next.flow, c >= 0 else { return state }
            next.flow = .coolOffVerdict(countdown: c, chip: chip)
        case .acknowledgeScareMirror:
            guard next.flow == .scareCandleMirror else { return state }
            next.flow = .coolOffVerdict(countdown: 8, chip: nil)
        case .tickCountdown:
            guard case let .coolOffVerdict(remaining, chip) = next.flow, remaining > 0 else { return state }
            next.flow = .coolOffVerdict(countdown: remaining - 1, chip: chip)
        }
        return next
    }

    /// After `q3` is set, branch terminal flow (still inside `reduce` for `answerQuestion`).
    private static func resolveTerminal(after state: BarExitGateState) -> BarExitGateState {
        guard let a1 = state.q1, let a2 = state.q2, let a3 = state.q3 else { return state }
        var s = state
        if a1, a2, a3 {
            s.flow = .cleanExitReady
        } else if a1, a2, !a3 {
            s.flow = .scareCandleMirror
        } else if !a1, !a2, !a3 {
            s.flow = .coolOffVerdict(countdown: 8, chip: nil)
        } else {
            s.flow = .coolOffVerdict(countdown: 8, chip: nil)
        }
        return s
    }
}
