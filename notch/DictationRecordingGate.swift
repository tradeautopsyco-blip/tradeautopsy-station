import Foundation

/// Push-to-talk (`fn` hold) vs latched recording (UI toggle). §6.5 interaction map.
struct DictationRecordingGate: Equatable {
    private(set) var latchEngaged: Bool = false
    private(set) var fnHeld: Bool = false

    var shouldRecord: Bool { latchEngaged || fnHeld }

    mutating func toggleLatch() {
        latchEngaged.toggle()
    }

    mutating func setFnHeld(_ down: Bool) {
        fnHeld = down
    }
}
