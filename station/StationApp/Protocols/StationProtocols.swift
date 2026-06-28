import Foundation

@MainActor
public protocol AgentSupervising: AnyObject {
    var onHealthChange: ((Bool) -> Void)? { get set }
    func start() async
    func retry() async
    func shutdown() async
    var isHealthy: Bool { get }
    var ownsSpawnedAgent: Bool { get }
    var currentWarning: AgentHealthWarning? { get }
}

@MainActor
public protocol HotkeyRegistering: AnyObject {
    func registerToggleNotch(_ handler: @escaping () -> Void)
    func registerOpenStation(_ handler: @escaping () -> Void)
    func unregisterAll()
}

@MainActor
public protocol StatusItemControlling: AnyObject {
    func install(coordinator: StationAppCoordinator)
    func updateAgentStatus(isHealthy: Bool)
    func updateLaunchAtLoginEnabled(_ enabled: Bool)
}

@MainActor
public protocol NotchHosting: AnyObject {
    func start() async
    func dismiss()
    func toggle()
}

@MainActor
public protocol NotchPollingControlling: AnyObject {
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
