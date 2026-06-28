import Foundation
@testable import Station

@MainActor
final class FakeNotchPolling: NotchPollingControlling {
    private(set) var startPollingCallCount = 0
    private(set) var stopPollingCallCount = 0

    func startPolling() {
        startPollingCallCount += 1
    }

    func stopPolling() {
        stopPollingCallCount += 1
    }
}
