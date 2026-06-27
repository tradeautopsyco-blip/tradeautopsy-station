import Foundation

/// Accumulates quiet RMS intervals; returns `true` once silence exceeds `requiredQuietDuration`.
struct SilenceAutoStopWatch: Equatable {
    let rmsThreshold: Float
    let requiredQuietDuration: TimeInterval

    private(set) var quietAccumulated: TimeInterval = 0

    init(
        rmsThreshold: Float,
        requiredQuietDuration: TimeInterval
    ) {
        self.rmsThreshold = rmsThreshold
        self.requiredQuietDuration = requiredQuietDuration
    }

    /// - Returns: `true` when caller should end dictation (silence budget exceeded).
    mutating func feed(rms: Float, deltaTime: TimeInterval) -> Bool {
        if rms < rmsThreshold {
            quietAccumulated += deltaTime
        } else {
            quietAccumulated = 0
        }
        return quietAccumulated >= requiredQuietDuration
    }

    mutating func reset() {
        quietAccumulated = 0
    }
}
