import Foundation

@MainActor
public protocol SoftwareUpdateControlling: AnyObject {
    var automaticallyChecksForUpdates: Bool { get set }
    var automaticallyDownloadsUpdates: Bool { get set }
    var canCheckForUpdates: Bool { get }
    func checkForUpdates(_ sender: Any?)
}

public final class NoOpSoftwareUpdateController: SoftwareUpdateControlling {
    public var automaticallyChecksForUpdates = true
    public var automaticallyDownloadsUpdates = false
    public var canCheckForUpdates = false

    public init() {}

    public func checkForUpdates(_ sender: Any?) {}
}
