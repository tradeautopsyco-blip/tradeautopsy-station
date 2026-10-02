import AppKit

@MainActor
public final class StatusItemController: StatusItemControlling {
    private var statusItem: NSStatusItem?
    private var launchAtLoginItem: NSMenuItem?
    private var stationLoginItem: NSMenuItem?
    private weak var coordinator: StationAppCoordinator?
    private var agentHealthy = false
    private var stationLoginRequired = false

    public init() {}

    public func install(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
        let item = NSStatusBar.system.statusItem(withLength: NSStatusItem.variableLength)
        statusItem = item

        if let button = item.button {
            if let image = Bundle.module.image(forResource: NSImage.Name("AppLogo")) {
                image.isTemplate = false
                image.size = NSSize(width: 18, height: 18)
                button.image = image
            }
            button.title = ""
            button.toolTip = "TradeAutopsy Station"
        }

        item.menu = buildMenu()
    }

    public func updateAgentStatus(isHealthy: Bool) {
        agentHealthy = isHealthy
        applyTint()
    }

    public func updateStationLoginRequired(_ required: Bool) {
        stationLoginRequired = required
        stationLoginItem?.isHidden = !required
        applyTint()
    }

    /// Red is reserved for an unhealthy agent; orange marks "agent up, session
    /// dead" so the two failure shapes stay distinguishable at a glance.
    private func applyTint() {
        let tint: NSColor? = if !agentHealthy {
            .systemRed
        } else if stationLoginRequired {
            .systemOrange
        } else {
            nil
        }
        statusItem?.button?.contentTintColor = tint
    }

    public func updateLaunchAtLoginEnabled(_ enabled: Bool) {
        launchAtLoginItem?.state = enabled ? .on : .off
    }

    private func buildMenu() -> NSMenu {
        let menu = NSMenu()

        let loginItem = NSMenuItem(
            title: "Device login required — Open Station to sign in",
            action: #selector(openStationForDeviceLogin(_:)),
            keyEquivalent: ""
        )
        loginItem.target = self
        loginItem.isHidden = true
        menu.addItem(loginItem)
        stationLoginItem = loginItem

        let openItem = NSMenuItem(title: "Open Station", action: #selector(openStation(_:)), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)

        menu.addItem(.separator())

        let launchAtLogin = NSMenuItem(
            title: "Launch at Login",
            action: #selector(toggleLaunchAtLogin(_:)),
            keyEquivalent: ""
        )
        launchAtLogin.target = self
        menu.addItem(launchAtLogin)
        launchAtLoginItem = launchAtLogin

        menu.addItem(.separator())

        let quitItem = NSMenuItem(title: "Quit", action: #selector(quit(_:)), keyEquivalent: "q")
        quitItem.target = self
        menu.addItem(quitItem)

        return menu
    }

    @objc private func openStation(_ sender: Any?) {
        coordinator?.openStation()
    }

    @objc private func openStationForDeviceLogin(_ sender: Any?) {
        coordinator?.openStationForDeviceLogin()
    }

    @objc private func toggleLaunchAtLogin(_ sender: Any?) {
        coordinator?.toggleLaunchAtLogin()
    }

    @objc private func quit(_ sender: Any?) {
        // Goes through applicationShouldTerminate so agent shutdown completes before exit.
        NSApp.terminate(nil)
    }
}
