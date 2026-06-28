import AppKit
import Notch
import SwiftUI

@MainActor
public final class StationWindowController: NSObject, StationWindowControlling, NSWindowDelegate {
    public static let defaultSize = NSSize(width: 1100, height: 700)

    private weak var coordinator: StationAppCoordinator?
    private let framePersistence: StationWindowFramePersistence
    private var window: NSWindow?

    public private(set) var isVisible = false

    public init(framePersistence: StationWindowFramePersistence = StationWindowFramePersistence()) {
        self.framePersistence = framePersistence
        super.init()
    }

    public func install(coordinator: StationAppCoordinator) {
        self.coordinator = coordinator
    }

    public func show(orderFrontOnly: Bool) {
        let window = ensureWindow()
        if orderFrontOnly {
            window.orderFront(nil)
        } else {
            window.makeKeyAndOrderFront(nil)
        }
        isVisible = true
    }

    public func showAndActivate() {
        let window = ensureWindow()
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        isVisible = true
    }

    public func hide() {
        window?.orderOut(nil)
        isVisible = false
    }

    public func persistFrame() {
        guard let window else { return }
        framePersistence.persist(frame: window.frame)
    }

    public func restoreFrame() {
        guard let window else { return }
        if let restored = framePersistence.restoreFrame() {
            window.setFrame(restored, display: false)
        }
    }

    public func windowShouldClose(_ sender: NSWindow) -> Bool {
        hide()
        coordinator?.closeWindow()
        return false
    }

    public func windowDidMove(_ notification: Notification) {
        persistFrame()
    }

    public func windowDidResize(_ notification: Notification) {
        persistFrame()
    }

    private func ensureWindow() -> NSWindow {
        if let window {
            return window
        }

        guard let coordinator else {
            fatalError("StationWindowController.install(coordinator:) must be called before showing the window")
        }

        let contentView = NSHostingView(
            rootView: StationShellView(coordinator: coordinator)
        )

        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: Self.defaultSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        window.title = "TradeAutopsy Station"
        window.titleVisibility = .hidden
        window.titlebarAppearsTransparent = true
        window.titlebarSeparatorStyle = .none
        window.styleMask.insert(.fullSizeContentView)
        window.toolbarStyle = .unified
        window.isReleasedWhenClosed = false
        window.contentView = contentView
        window.delegate = self
        window.minSize = Self.defaultSize
        window.backgroundColor = NSColor(BarDS.Fill.sidebar)

        if let restored = framePersistence.restoreFrame() {
            window.setFrame(restored, display: false)
        } else {
            window.setContentSize(Self.defaultSize)
            window.center()
        }

        self.window = window
        return window
    }
}
