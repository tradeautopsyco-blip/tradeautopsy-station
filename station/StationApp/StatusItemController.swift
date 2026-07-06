import AppKit

@MainActor
public final class StatusItemController: StatusItemControlling {
    private var statusItem: NSStatusItem?
    private var launchAtLoginItem: NSMenuItem?
    private weak var coordinator: StationAppCoordinator?

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
        statusItem?.button?.contentTintColor = isHealthy ? nil : .systemRed
    }

    public func updateLaunchAtLoginEnabled(_ enabled: Bool) {
        launchAtLoginItem?.state = enabled ? .on : .off
    }

    private func buildMenu() -> NSMenu {
        let menu = NSMenu()

        let openItem = NSMenuItem(title: "Open Station", action: #selector(openStation(_:)), keyEquivalent: "")
        openItem.target = self
        menu.addItem(openItem)

        let toggleItem = NSMenuItem(title: "Toggle Notch", action: #selector(toggleNotch(_:)), keyEquivalent: "")
        toggleItem.target = self
        menu.addItem(toggleItem)

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

    @objc private func toggleNotch(_ sender: Any?) {
        coordinator?.toggleNotch()
    }

    @objc private func toggleLaunchAtLogin(_ sender: Any?) {
        coordinator?.toggleLaunchAtLogin()
    }

    @objc private func quit(_ sender: Any?) {
        Task { @MainActor in
            await coordinator?.quit()
            NSApp.terminate(nil)
        }
    }
}
