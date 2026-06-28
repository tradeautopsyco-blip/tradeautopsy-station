import Foundation
@testable import Station

@MainActor
final class FakeStationWindowController: StationWindowControlling {
    private(set) var showCallCount = 0
    private(set) var showAndActivateCallCount = 0
    private(set) var hideCallCount = 0
    private(set) var persistFrameCallCount = 0
    private(set) var restoreFrameCallCount = 0
    private(set) var lastShowOrderFrontOnly: Bool?
    private(set) var windowsCreated = 0

    private(set) var isVisible = false

    func show(orderFrontOnly: Bool) {
        showCallCount += 1
        lastShowOrderFrontOnly = orderFrontOnly
        if !isVisible {
            windowsCreated += 1
        }
        isVisible = true
    }

    func showAndActivate() {
        showAndActivateCallCount += 1
        if !isVisible {
            windowsCreated += 1
        }
        isVisible = true
    }

    func hide() {
        hideCallCount += 1
        isVisible = false
    }

    func persistFrame() {
        persistFrameCallCount += 1
    }

    func restoreFrame() {
        restoreFrameCallCount += 1
    }
}
