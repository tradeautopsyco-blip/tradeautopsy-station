import Foundation
import ServiceManagement

public enum LoginItemRegistrationStatus: Equatable, Sendable {
    case notRegistered
    case enabled
    case requiresApproval
    case notFound
}

public protocol LoginItemBackend: Sendable {
    var registrationStatus: LoginItemRegistrationStatus { get }
    func register() throws
    func unregister() throws
}

public final class SMAppServiceLoginItemBackend: LoginItemBackend {
    public init() {}

    public var registrationStatus: LoginItemRegistrationStatus {
        switch SMAppService.mainApp.status {
        case .notRegistered:
            return .notRegistered
        case .enabled:
            return .enabled
        case .requiresApproval:
            return .requiresApproval
        case .notFound:
            return .notFound
        @unknown default:
            return .notFound
        }
    }

    public func register() throws {
        try SMAppService.mainApp.register()
    }

    public func unregister() throws {
        try SMAppService.mainApp.unregister()
    }
}

@MainActor
public final class LoginItemService: LoginItemServicing {
    public static let promptDismissedKey = "station.loginItem.promptDismissed"

    private let backend: LoginItemBackend
    private let defaults: UserDefaults
    private var cachedRegistered: Bool

    public init(
        backend: LoginItemBackend = SMAppServiceLoginItemBackend(),
        defaults: UserDefaults = .standard
    ) {
        self.backend = backend
        self.defaults = defaults
        self.cachedRegistered = backend.registrationStatus == .enabled
    }

    public var isRegistered: Bool {
        cachedRegistered
    }

    public var isPromptDismissed: Bool {
        defaults.bool(forKey: Self.promptDismissedKey)
    }

    public func setRegistered(_ enabled: Bool) throws {
        if enabled {
            try backend.register()
        } else {
            try backend.unregister()
        }
        cachedRegistered = backend.registrationStatus == .enabled
    }

    public func syncStatusOnLaunch() {
        cachedRegistered = backend.registrationStatus == .enabled
    }

    public func markPromptDismissed() {
        defaults.set(true, forKey: Self.promptDismissedKey)
    }
}
