import Foundation
@testable import Station

@MainActor
final class FakeFloatingNotchHost: FloatingNotchHosting {
    private(set) var configureCallCount = 0
    private(set) var startCallCount = 0
    private(set) var dismissCallCount = 0
    private(set) var toggleCallCount = 0
    private(set) var lastSecret: String?
    private(set) var lastPort: UInt16?
    private(set) var lastWebBase: String?

    private(set) var brokerConnectHandler: ((String) -> Void)?
    private(set) var brokerReauthHandler: ((String) -> Void)?
    private(set) var deviceLoginHandler: (() -> Void)?
    private(set) var lastBridgeResult: String?
    private(set) var lastBridgeError: String?
    private(set) var reportBridgeOutcomeCallCount = 0

    func configure(secret: String, port: UInt16, webBase: String) {
        configureCallCount += 1
        lastSecret = secret
        lastPort = port
        lastWebBase = webBase
    }

    func start() {
        startCallCount += 1
    }

    func dismiss() {
        dismissCallCount += 1
    }

    func toggle() {
        toggleCallCount += 1
    }

    func setBrokerBridge(onConnect: @escaping (String) -> Void, onReauth: @escaping (String) -> Void) {
        brokerConnectHandler = onConnect
        brokerReauthHandler = onReauth
    }

    func setDeviceLoginBridge(onOpen: @escaping () -> Void) {
        deviceLoginHandler = onOpen
    }

    func reportBrokerBridgeOutcome(result: String?, error: String?) {
        reportBridgeOutcomeCallCount += 1
        lastBridgeResult = result
        lastBridgeError = error
    }
}
