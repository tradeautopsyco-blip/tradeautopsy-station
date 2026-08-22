import Combine
import Foundation
import Testing
@testable import Notch

@MainActor
struct NotchEnergyPublishTests {
    @Test func setIfChangedSkipsEqualPublishedWrite() {
        let vm = NotchViewModel()
        var fires = 0
        let sub = vm.objectWillChange.sink { _ in fires += 1 }
        vm.setIfChanged(\.daemonConnectionState, .idle)
        #expect(fires == 0)
        vm.setIfChanged(\.daemonConnectionState, .connected)
        #expect(fires == 1)
        vm.setIfChanged(\.daemonConnectionState, .connected)
        #expect(fires == 1)
        _ = sub
    }

    @Test func waveformGateDropsIdenticalAndThrottles() {
        var gate = WaveformPublishGate(minInterval: 0.1)
        let a: [Float] = [0.1, 0.2, 0, 0, 0, 0, 0, 0, 0]
        let first = gate.shouldPublish(a, now: 1.0)
        let same = gate.shouldPublish(a, now: 1.05)
        let b: [Float] = [0.3, 0.2, 0, 0, 0, 0, 0, 0, 0]
        let tooSoon = gate.shouldPublish(b, now: 1.05)
        let later = gate.shouldPublish(b, now: 1.12)
        #expect(first)
        #expect(!same)
        #expect(!tooSoon)
        #expect(later)
    }

    @Test func waveformGateForcePublishesEvenWhenEqualAfterReset() {
        var gate = WaveformPublishGate(minInterval: 0.1)
        let zeros: [Float] = Array(repeating: 0, count: 9)
        let first = gate.shouldPublish(zeros, now: 1.0)
        gate.reset()
        let forced = gate.shouldPublish(zeros, now: 1.01, force: true)
        #expect(first)
        #expect(forced)
    }
}
