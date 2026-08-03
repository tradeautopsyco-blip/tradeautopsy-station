import Foundation

/// Station-owned desk poller host. Drives `SessionModel` for Session P&L / Today.
/// Floating Notch is owned separately by `FloatingNotchHost` (PLAN-only panel).
@MainActor
public final class SessionPollingHost: SessionHosting, SessionPollingControlling {
    public let sessionModel: SessionModel
    /// Legacy toggle hook — unused when Station routes ⌥Space to `FloatingNotchHost`.
    public var onToggle: (() -> Void)?

    public init(sessionModel: SessionModel) {
        self.sessionModel = sessionModel
    }

    public func configure(secret: String, port: UInt16, webBase: String) {
        sessionModel.configure(secret: secret, port: port, webBase: webBase)
    }

    public func start() async {
        sessionModel.startPolling()
    }

    public func dismiss() {
        sessionModel.stopPolling()
    }

    public func startPolling() {
        sessionModel.startPolling()
    }

    public func stopPolling() {
        sessionModel.stopPolling()
    }

    public func toggle() {
        onToggle?()
    }
}
