import Foundation
@testable import Station

@MainActor
final class FakeLaunchStore: StationLaunchStoring {
    var isFirstLaunchCompleted = false
    var wasWindowVisibleBeforeQuit = false
    var loginAtBoot = false

    private(set) var setFirstLaunchCompletedCallCount = 0
    private(set) var setWasWindowVisibleCallCount = 0

    func setFirstLaunchCompleted() {
        setFirstLaunchCompletedCallCount += 1
        isFirstLaunchCompleted = true
    }

    func setWasWindowVisibleBeforeQuit(_ visible: Bool) {
        setWasWindowVisibleCallCount += 1
        wasWindowVisibleBeforeQuit = visible
    }
}
