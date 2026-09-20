import Foundation

/// OpenAlgo 7-box as a Confirm strip. Empty box → size = 0 this wave (Confirm blocked).
struct BarPlanGateStripState: Equatable, Sendable {
    var capitalAck: Bool = false
    var onePercentAck: Bool = false
    var maxLossAck: Bool = false
    var hedgeAck: Bool = false
    var reviewAck: Bool = false
}

enum BarPlanGateStrip {
    static func emotionFilled(calm: Int, confidence: Int, frustration: Int, excitement: Int) -> Bool {
        (1 ... 5).contains(calm)
            && (1 ... 5).contains(confidence)
            && (1 ... 5).contains(frustration)
            && (1 ... 5).contains(excitement)
    }

    static func isComplete(
        state: BarPlanGateStripState,
        emotionFilled: Bool,
        exitFilled: Bool
    ) -> Bool {
        state.capitalAck
            && state.onePercentAck
            && state.maxLossAck
            && state.hedgeAck
            && state.reviewAck
            && emotionFilled
            && exitFilled
    }

    /// Names the first empty box for the Confirm hint.
    static func emptyHint(
        state: BarPlanGateStripState,
        emotionFilled: Bool,
        exitFilled: Bool
    ) -> String? {
        if !state.capitalAck { return "Ack capital — risk money, not rent." }
        if !state.onePercentAck { return "Ack size is at the stop." }
        if !state.maxLossAck { return "Ack you are still inside the day." }
        if !state.hedgeAck { return "Ack hedge decided / none." }
        if !emotionFilled { return "Set all four emotion sliders." }
        if !exitFilled { return "Write invalidation and a target." }
        if !state.reviewAck { return "Ack you will debrief this." }
        return nil
    }

    static func asJSONObject(_ state: BarPlanGateStripState) -> [String: Bool] {
        [
            "capital": state.capitalAck,
            "one_percent": state.onePercentAck,
            "max_loss": state.maxLossAck,
            "hedge": state.hedgeAck,
            "review": state.reviewAck,
        ]
    }
}
