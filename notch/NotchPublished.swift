import Foundation

extension NotchViewModel {
    /// Skip no-op `@Published` assignments — they still fire `objectWillChange` and rebuild the HUD.
    func setIfChanged<T: Equatable>(_ keyPath: ReferenceWritableKeyPath<NotchViewModel, T>, _ new: T) {
        guard self[keyPath: keyPath] != new else { return }
        self[keyPath: keyPath] = new
    }

    func clearDeclarationErrorIfNeeded() {
        setIfChanged(\.barDeclarationLastError, nil)
    }
}

/// Caps dictation waveform publishes (~10 Hz) and drops identical frames.
struct WaveformPublishGate {
    var minInterval: CFAbsoluteTime
    private var lastAt: CFAbsoluteTime = 0
    private var lastLevels: [Float] = []

    init(minInterval: CFAbsoluteTime = 0.1) {
        self.minInterval = minInterval
    }

    mutating func shouldPublish(_ levels: [Float], now: CFAbsoluteTime, force: Bool = false) -> Bool {
        if !force, levels == lastLevels { return false }
        if !force, !lastLevels.isEmpty, now - lastAt < minInterval { return false }
        lastAt = now
        lastLevels = levels
        return true
    }

    mutating func reset() {
        lastAt = 0
        lastLevels = []
    }
}

/// One shared 1s clock so screens don't each spawn `Timer.publish` in `body`.
enum NotchOneSecondClock {
    static let publisher = Timer.publish(every: 1, on: .main, in: .common).autoconnect()
}
