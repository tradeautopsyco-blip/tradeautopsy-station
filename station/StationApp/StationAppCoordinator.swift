import Foundation

@MainActor
public final class StationAppCoordinator: ObservableObject {
    @Published public private(set) var agentHealthWarning: AgentHealthWarning?

    /// Shell navigation stays enabled even when the agent is unhealthy (TRD §6.3).
    public var isShellNavigable: Bool { true }

    private let agentSupervisor: AgentSupervising
    private let statusItemController: StatusItemControlling
    private let hotkeyRegistrar: HotkeyRegistering
    private let notchHost: NotchHosting
    private let notchPolling: NotchPollingControlling

    public init(
        agentSupervisor: AgentSupervising,
        statusItemController: StatusItemControlling,
        hotkeyRegistrar: HotkeyRegistering,
        notchHost: NotchHosting,
        notchPolling: NotchPollingControlling
    ) {
        self.agentSupervisor = agentSupervisor
        self.statusItemController = statusItemController
        self.hotkeyRegistrar = hotkeyRegistrar
        self.notchHost = notchHost
        self.notchPolling = notchPolling
    }

    public func launch() async {
        await agentSupervisor.start()
        syncAgentHealthFromSupervisor()
        statusItemController.install(coordinator: self)

        if agentSupervisor.isHealthy {
            await startNotchAndPolling()
        }

        registerHotkeys()
    }

    public func retryAgent() async {
        await agentSupervisor.retry()
        syncAgentHealthFromSupervisor()

        if agentSupervisor.isHealthy {
            await startNotchAndPolling()
        }
    }

    public func quit() async {
        hotkeyRegistrar.unregisterAll()
        await agentSupervisor.shutdown()
        notchPolling.stopPolling()
        notchHost.dismiss()
    }

    private func startNotchAndPolling() async {
        await notchHost.start()
        notchPolling.startPolling()
    }

    private func syncAgentHealthFromSupervisor() {
        agentHealthWarning = agentSupervisor.currentWarning
        statusItemController.updateAgentStatus(isHealthy: agentSupervisor.isHealthy)
    }

    private func registerHotkeys() {
        hotkeyRegistrar.registerToggleNotch { }
        hotkeyRegistrar.registerOpenStation { }
    }
}
