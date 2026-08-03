import Foundation
@testable import Station

@MainActor
final class TrackingSessionHost: SessionHosting {
    let injectedSessionModel: SessionModel
    private(set) var startCallCount = 0
    private(set) var dismissCallCount = 0
    private(set) var toggleCallCount = 0

    init(sessionModel: SessionModel) {
        self.injectedSessionModel = sessionModel
    }

    func start() async {
        startCallCount += 1
    }

    func dismiss() {
        dismissCallCount += 1
    }

    func toggle() {
        toggleCallCount += 1
    }
}
