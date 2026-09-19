import AppKit
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
    private var isShuttingDown = false

    func applicationDidFinishLaunching(_ notification: Notification) {
        // Dock + Cmd-Tab like a normal app (Info.plist LSUIElement is false).
        NSApp.setActivationPolicy(.regular)
        ApplicationMainMenu.install()
        let inputMonitoringChecker = DefaultInputMonitoringChecker()
        let launchStore = UserDefaultsLaunchStore()
        let windowController = StationWindowController()
        let phaseProvider = DefaultSessionSurfacePhaseProvider()
        let sessionModel = SessionModel()
        let sessionHost = SessionPollingHost(sessionModel: sessionModel)
        let floatingNotch = FloatingNotchHost()
        let daemonSecret = AgentDaemonSecret.resolveForSession()
        let port = AgentLoopback.port
        let base = "http://127.0.0.1:\(port)"
        sessionHost.configure(secret: daemonSecret, port: port, webBase: base)
        floatingNotch.configure(secret: daemonSecret, port: port, webBase: base)
        let coordinator = StationAppCoordinator(
            agentSupervisor: AgentSupervisor(daemonSecret: daemonSecret),
            statusItemController: StatusItemController(),
            hotkeyRegistrar: HotkeyRegistrar(inputMonitoringChecker: inputMonitoringChecker),
            sessionHost: sessionHost,
            sessionPolling: sessionHost,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: phaseProvider,
            sessionModel: sessionModel,
            inputMonitoringChecker: inputMonitoringChecker,
            daemonSecret: daemonSecret,
            floatingNotch: floatingNotch
        )
        windowController.install(coordinator: coordinator)
        self.coordinator = coordinator

        floatingNotch.setBrokerBridge(
            onConnect: { [weak coordinator, weak floatingNotch] slug in
                guard let coordinator else { return }
                coordinator.navigateTo(.brokers)
                coordinator.openStation()
                Task { @MainActor in
                    await coordinator.brokersViewModel.beginConnect(for: slug)
                    let sheet = coordinator.brokersViewModel.isConnectSheetPresented
                    let msg = coordinator.brokersViewModel.syncActionMessage
                    if sheet {
                        floatingNotch?.reportBrokerBridgeOutcome(
                            result: "Opened Station Brokers — complete Connect / TOTP there.",
                            error: nil
                        )
                    } else if let msg, msg.localizedCaseInsensitiveContains("kill switch") {
                        floatingNotch?.reportBrokerBridgeOutcome(result: nil, error: msg)
                    } else if let msg {
                        floatingNotch?.reportBrokerBridgeOutcome(result: msg, error: nil)
                    } else {
                        floatingNotch?.reportBrokerBridgeOutcome(result: nil, error: nil)
                    }
                }
            },
            onReauth: { [weak coordinator] slug in
                guard let coordinator else { return }
                coordinator.navigateTo(.brokers)
                coordinator.openStation()
                coordinator.brokersViewModel.presentKotakTotpRemint(for: slug)
            }
        )

        floatingNotch.setDeviceLoginBridge(onOpen: { [weak coordinator] in
            guard let coordinator else { return }
            coordinator.navigateTo(.settings)
            coordinator.openStation()
        })

        Task {
            await coordinator.launch()
        }
    }

    /// Delay quit until agent shutdown finishes — fire-and-forget `willTerminate` races the OS.
    func applicationShouldTerminate(_ sender: NSApplication) -> NSApplication.TerminateReply {
        if isShuttingDown {
            return .terminateNow
        }
        isShuttingDown = true
        Task { @MainActor in
            await coordinator?.quit()
            NSApp.reply(toApplicationShouldTerminate: true)
        }
        return .terminateLater
    }
 
    func applicationWillTerminate(_ notification: Notification) {
        // Best-effort if something bypassed shouldTerminate (e.g. forced kill path).
        // Quit already ran in shouldTerminate for normal exits.
    }
}
