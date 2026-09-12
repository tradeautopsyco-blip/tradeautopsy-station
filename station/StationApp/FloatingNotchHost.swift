import Foundation
import Notch

/// Thin Station host for the floating Notch panel (PLAN-only when `isHostedByStation`).
/// Does not inject Station desk content into the expanded panel.
@MainActor
public final class FloatingNotchHost: FloatingNotchHosting {
    private let launcher: NotchLauncher

    public init(launcher: NotchLauncher = NotchLauncher(isHostedByStation: true)) {
        self.launcher = launcher
    }

    public func configure(secret: String, port: UInt16, webBase: String) {
        launcher.configure(secret: secret, port: port, webBase: webBase)
    }

    public func start() {
        launcher.start()
    }

    public func dismiss() {
        launcher.dismiss()
    }

    public func toggle() {
        launcher.toggle()
    }

    public func hide() {
        launcher.hide()
    }

    public func show() {
        launcher.show()
    }

    public func setBrokerBridge(onConnect: @escaping (String) -> Void, onReauth: @escaping (String) -> Void) {
        launcher.viewModel.onRequestOpenBrokerConnect = onConnect
        launcher.viewModel.onRequestOpenBrokerReauth = onReauth
    }

    /// Separate from `setBrokerBridge` — device login (Station Caller tokens) is a distinct
    /// subsystem from broker Connect/Reauth and must not be conflated in the bridge callback.
    public func setDeviceLoginBridge(onOpen: @escaping () -> Void) {
        launcher.viewModel.onRequestOpenDeviceLogin = onOpen
    }

    public func reportBrokerBridgeOutcome(result: String?, error: String?) {
        launcher.viewModel.reportBrokerBridgeOutcome(result: result, error: error)
    }
}
