import AppKit
import Foundation
import Sparkle

@MainActor
public final class SparkleSoftwareUpdateController: SoftwareUpdateControlling {
    private let updaterController: SPUStandardUpdaterController

    public init(startingUpdater: Bool = true) {
        updaterController = SPUStandardUpdaterController(
            startingUpdater: startingUpdater,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
    }

    public var automaticallyChecksForUpdates: Bool {
        get { updaterController.updater.automaticallyChecksForUpdates }
        set { updaterController.updater.automaticallyChecksForUpdates = newValue }
    }

    public var automaticallyDownloadsUpdates: Bool {
        get { updaterController.updater.automaticallyDownloadsUpdates }
        set { updaterController.updater.automaticallyDownloadsUpdates = newValue }
    }

    public var canCheckForUpdates: Bool {
        updaterController.updater.canCheckForUpdates
    }

    public func checkForUpdates(_ sender: Any?) {
        updaterController.checkForUpdates(sender)
    }
}
