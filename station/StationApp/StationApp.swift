import AppKit
import Notch
import Station

@main
struct StationAppMain {
    static func main() {
        let app = NSApplication.shared
        let delegate = StationAppDelegate()
        app.delegate = delegate
        app.run()
    }
}

@MainActor
final class StationAppDelegate: NSObject, NSApplicationDelegate {
    private var coordinator: StationAppCoordinator?

    func applicationDidFinishLaunching(_ notification: Notification) {
        let inputMonitoringChecker = DefaultInputMonitoringChecker()
        let launchStore = UserDefaultsLaunchStore()
        let windowController = StationWindowController()
        let phaseProvider = DefaultBarSurfacePhaseProvider()
        let notchViewModel = NotchViewModel()
        let notchHost = HostedNotchLauncher(viewModel: notchViewModel)
        let daemonSecret = AgentDaemonSecret.resolveForSession()
        let port = AgentLoopback.port
        let base = "http://127.0.0.1:\(port)"
        notchHost.configure(secret: daemonSecret, port: port, webBase: base)
        let coordinator = StationAppCoordinator(
            agentSupervisor: AgentSupervisor(daemonSecret: daemonSecret),
            statusItemController: StatusItemController(),
            hotkeyRegistrar: HotkeyRegistrar(inputMonitoringChecker: inputMonitoringChecker),
            notchHost: notchHost,
            notchPolling: notchHost,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: phaseProvider,
            notchViewModel: notchViewModel,
            inputMonitoringChecker: inputMonitoringChecker,
            daemonSecret: daemonSecret
        )
        windowController.install(coordinator: coordinator)
        if let hostedNotch = notchHost as? HostedNotchLauncher {
            hostedNotch.bindCoordinator(coordinator)
        }
        self.coordinator = coordinator

        Task {
            await coordinator.launch()
        }
    }

    func applicationWillTerminate(_ notification: Notification) {
        Task {
            await coordinator?.quit()
        }
    }
}

