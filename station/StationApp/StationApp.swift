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
        let notchHost = NotchABIHost()
        let launchStore = UserDefaultsLaunchStore()
        let windowController = StationWindowController()
        let phaseProvider = DefaultBarSurfacePhaseProvider()
        let coordinator = StationAppCoordinator(
            agentSupervisor: AgentSupervisor(),
            statusItemController: StatusItemController(),
            hotkeyRegistrar: HotkeyRegistrar(),
            notchHost: notchHost,
            notchPolling: notchHost,
            windowController: windowController,
            launchStore: launchStore,
            phaseProvider: phaseProvider
        )
        windowController.install(coordinator: coordinator)
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

/// Hosts the notch via existing C ABI (`tradeautopsy_notch_*`) without requiring Notch type exports.
@MainActor
final class NotchABIHost: NotchHosting, NotchPollingControlling {
    func start() async {
        let secret = ProcessInfo.processInfo.environment["AGENT_SECRET"] ?? ""
        let port = AgentLoopback.port
        let base = "http://127.0.0.1:\(port)"
        secret.withCString { secretPtr in
            base.withCString { basePtr in
                tradeautopsy_notch_launch(secretPtr, port, basePtr)
            }
        }
    }

    func dismiss() {
        tradeautopsy_notch_dismiss()
    }

    func startPolling() {
        // `tradeautopsy_notch_launch` → `NotchLauncher.start()` already starts polling.
    }

    func stopPolling() {
        // `tradeautopsy_notch_dismiss` → `NotchLauncher.dismiss()` stops polling.
    }
}

@MainActor
final class HotkeyRegistrar: HotkeyRegistering {
    func registerToggleNotch(_ handler: @escaping () -> Void) {
        // Placeholder — hotkey centralization is issue #7.
    }

    func registerOpenStation(_ handler: @escaping () -> Void) {
        // Placeholder — hotkey centralization is a later slice.
    }

    func unregisterAll() {
        // Placeholder — hotkey centralization is a later slice.
    }
}

enum AgentLoopback {
    static let port: UInt16 = 9137
}
