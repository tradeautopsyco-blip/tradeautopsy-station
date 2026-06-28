import Foundation

@MainActor
public protocol AgentSupervising: AnyObject {
    var onHealthChange: ((Bool) -> Void)? { get set }
    func start() async
    func retry() async
    func shutdown() async
    var isHealthy: Bool { get }
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
}

@MainActor
public protocol NotchHosting: AnyObject {
    func start() async
    func dismiss()
}

@MainActor
public protocol NotchPollingControlling: AnyObject {
    func startPolling()
    func stopPolling()
}
