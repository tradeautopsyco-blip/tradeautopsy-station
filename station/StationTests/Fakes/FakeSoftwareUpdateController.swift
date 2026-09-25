import Foundation
@testable import Station

@MainActor
final class FakeSoftwareUpdateController: SoftwareUpdateControlling {
    var automaticallyChecksForUpdates = true
    var automaticallyDownloadsUpdates = false
    var canCheckForUpdates = true
    private(set) var checkForUpdatesCallCount = 0

    func checkForUpdates(_ sender: Any?) {
        checkForUpdatesCallCount += 1
    }
}
