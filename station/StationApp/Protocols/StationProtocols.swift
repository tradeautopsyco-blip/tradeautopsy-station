import Foundation

@MainActor
public protocol AgentSupervising: AnyObject {
    var onHealthChange: ((Bool) -> Void)? { get set }
    func start() async
    func retry() async
    func shutdown() async
    func isKillLatched() async -> Bool
    func restartAgent() async -> AgentManualRestartOutcome
    var isHealthy: Bool { get }
    var ownsSpawnedAgent: Bool { get }
    var currentWarning: AgentHealthWarning? { get }
}

/// One saved row from Notch prefs. Carbon still owns the two shell defaults when this list is empty.
public struct DeskHotkeyRegistration: Equatable, Sendable {
    public var actionId: String
    public var keyCode: UInt32
    public var carbonModifiers: UInt32

    public init(actionId: String, keyCode: UInt32, carbonModifiers: UInt32) {
        self.actionId = actionId
        self.keyCode = keyCode
        self.carbonModifiers = carbonModifiers
    }
}

@MainActor
public protocol HotkeyRegistering: AnyObject {
    func registerToggleNotch(_ handler: @escaping () -> Void)
    func registerOpenStation(_ handler: @escaping () -> Void)
    /// Notch actions other than toggle / open Station. `toggle_notch` and `open_station` stay on their own handlers.
    func registerDeskActions(_ handler: @escaping (String) -> Void)
    /// Empty list keeps ⌥Space and ⌥⇧Space. A saved `toggle_notch` or `open_station` replaces that default.
    func reloadSavedBindings(_ bindings: [DeskHotkeyRegistration])
    func unregisterAll()
    /// Install global monitor after Input Monitoring is granted mid-session.
    func refreshGlobalMonitorIfNeeded()
}

@MainActor
public protocol StatusItemControlling: AnyObject {
    func install(coordinator: StationAppCoordinator)
    func updateAgentStatus(isHealthy: Bool)
    func updateLaunchAtLoginEnabled(_ enabled: Bool)
}

@MainActor
public protocol SessionHosting: AnyObject {
    func start() async
    func dismiss()
    func toggle()
}

@MainActor
public protocol FloatingNotchHosting: AnyObject {
    func configure(secret: String, port: UInt16, webBase: String)
    func start()
    func dismiss()
    func toggle()
    func hide()
    func show()
    func setBrokerBridge(onConnect: @escaping (String) -> Void, onReauth: @escaping (String) -> Void)
    func setDeviceLoginBridge(onOpen: @escaping () -> Void)
    func reportBrokerBridgeOutcome(result: String?, error: String?)
    /// Desk action ids from `docs/design/hotkey-actions.md`, other than `toggle_notch` and `open_station`.
    func performHotkey(_ actionId: String)
}

@MainActor
public protocol SessionPollingControlling: AnyObject {
    func startPolling()
    func stopPolling()
}

@MainActor
public protocol StationWindowControlling: AnyObject {
    func show(orderFrontOnly: Bool)
    func showAndActivate()
    func hide()
    func persistFrame()
    func restoreFrame()
    var isVisible: Bool { get }
}

@MainActor
public protocol StationLaunchStoring: AnyObject {
    var isFirstLaunchCompleted: Bool { get }
    func setFirstLaunchCompleted()
    var wasWindowVisibleBeforeQuit: Bool { get }
    func setWasWindowVisibleBeforeQuit(_ visible: Bool)
    var loginAtBoot: Bool { get }
}

@MainActor
public protocol LoginItemServicing: AnyObject {
    var isRegistered: Bool { get }
    func setRegistered(_ enabled: Bool) throws
    func syncStatusOnLaunch()
    var isPromptDismissed: Bool { get }
    func markPromptDismissed()
}
