import CoreGraphics
import Foundation

/// Click vs move for the collapsed pill.
/// Translation under `hysteresis` is a click (expand). Crossing it starts a move
/// that stays a move even if the pointer returns — Apple: don't reverse the decision mid-gesture.
struct CollapsedPillDragReducer: Equatable {
    static let hysteresis: CGFloat = 10

    private(set) var passedHysteresis = false

    enum Action: Equatable {
        case none
        case move
        case expand
        case moveEnded
    }

    mutating func changed(translation: CGSize) -> Action {
        if passedHysteresis || Self.distance(translation) >= Self.hysteresis {
            passedHysteresis = true
            return .move
        }
        return .none
    }

    mutating func ended(translation: CGSize) -> Action {
        let moving = passedHysteresis || Self.distance(translation) >= Self.hysteresis
        passedHysteresis = false
        return moving ? .moveEnded : .expand
    }

    mutating func reset() {
        passedHysteresis = false
    }

    static func distance(_ translation: CGSize) -> CGFloat {
        hypot(translation.width, translation.height)
    }
}
