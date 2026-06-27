import Foundation

/// Maps RMS samples to nine waveform dot levels (0…1). Reduce Motion uses a flat snapshot (§6.7).
struct WaveformNineDotRing: Equatable {
    static let dotCount = 9

    private var slots: [Float]
    private var smoothed: Float = 0
    private let smoothAlpha: Float

    init(smoothAlpha: Float = 0.45) {
        self.smoothAlpha = min(1, max(0.05, smoothAlpha))
        slots = Array(repeating: 0, count: Self.dotCount)
    }

    mutating func push(rms: Float, reduceMotion: Bool) -> [Float] {
        let n = min(1, max(0, rms))
        if reduceMotion {
            smoothed = n
            slots = Array(repeating: smoothed, count: Self.dotCount)
        } else {
            smoothed += smoothAlpha * (n - smoothed)
            if slots.count == Self.dotCount {
                slots.removeFirst()
            }
            slots.append(smoothed)
            while slots.count < Self.dotCount {
                slots.insert(smoothed, at: 0)
            }
        }
        return slots
    }
}
