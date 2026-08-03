import AppKit
import SwiftUI

@MainActor
public final class StationWindowController: NSObject, StationWindowControlling, NSWindowDelegate {
    /// Fallback when no screen metrics are available (tests / headless).
    public static let defaultSize = NSSize(width: 1100, height: 700)
    public static let minimumSize = NSSize(width: 820, height: 560)
    public static let defaultVisibleFraction: CGFloat = 0.9

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
        applyFrame(to: window)
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

    /// ~90% of the screen's visible frame, clamped to `minimumSize`, centered.
    public static func defaultFrame(
        in visibleFrame: CGRect,
        fraction: CGFloat = defaultVisibleFraction,
        minimumSize: NSSize = minimumSize
    ) -> CGRect {
        guard visibleFrame.width > 0, visibleFrame.height > 0 else {
            return CGRect(origin: .zero, size: defaultSize)
        }
        let width = min(visibleFrame.width, max(minimumSize.width, visibleFrame.width * fraction))
        let height = min(visibleFrame.height, max(minimumSize.height, visibleFrame.height * fraction))
        let x = visibleFrame.midX - width / 2
        let y = visibleFrame.midY - height / 2
        return CGRect(x: x, y: y, width: width, height: height)
    }

    /// Reject tiny or off-screen restored frames so we fall back to the large default.
    public static func isFrameUsable(
        _ frame: CGRect,
        onScreenFrames screens: [CGRect],
        minimumSize: NSSize = minimumSize
    ) -> Bool {
        guard frame.width >= minimumSize.width, frame.height >= minimumSize.height else {
            return false
        }
        guard !screens.isEmpty else {
            return true
        }
        let overlapThreshold = min(frame.width, frame.height) * 0.25
        return screens.contains { screen in
            let intersection = frame.intersection(screen)
            return intersection.width >= overlapThreshold && intersection.height >= overlapThreshold
        }
    }

    private func ensureWindow() -> NSWindow {
        if let window {
            return window
        }

        guard let coordinator else {
            fatalError("StationWindowController.install(coordinator:) must be called before showing the window")
        }

        let hostingController = NSHostingController(
            rootView: StationShellView(coordinator: coordinator)
        )
        // SwiftUI's layout has its own opinions about min/ideal size (e.g. from
        // .navigationSplitViewColumnWidth); don't let it fight our manual
        // minSize/frame-restore management below.
        hostingController.sizingOptions = []

        let initialSize = Self.defaultSize
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: initialSize),
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false
        )
        // Title fallback for the brief instant before the hosting controller's first
        // layout pass; once laid out, the toolbar title is driven by whichever route's
        // .navigationTitle is active (e.g. "Today"), matching Mail/Notes/Finder.
        window.title = "TradeAutopsy Station"
        window.titleVisibility = .visible
        window.titlebarAppearsTransparent = true
        // .automatic draws the native hairline only once content is actually scrolled
        // under the toolbar — same as Mail/Notes/Xcode.
        window.titlebarSeparatorStyle = .automatic
        window.styleMask.insert(.fullSizeContentView)
        window.toolbarStyle = .unified
        window.isReleasedWhenClosed = false
        // contentViewController (not contentView) is required for SwiftUI's .toolbar{}
        // to bridge to a real NSToolbar, and for NavigationSplitView's sidebar-toggle
        // button to appear automatically.
        window.contentViewController = hostingController
        window.delegate = self
        window.minSize = Self.minimumSize
        // Lets the sidebar's .listStyle(.sidebar) vibrancy actually blend against
        // something instead of rendering as a flat opaque color.
        window.isOpaque = false
        window.backgroundColor = NSColor(StationDS.Fill.sidebar)
        // Green traffic-light offers native fullscreen (do not set .fullScreen styleMask at init).
        window.collectionBehavior.insert(.fullScreenPrimary)

        applyFrame(to: window)

        self.window = window
        return window
    }

    private func applyFrame(to window: NSWindow) {
        let screens = NSScreen.screens.map(\.visibleFrame)
        if let restored = framePersistence.restoreFrame(),
           Self.isFrameUsable(restored, onScreenFrames: screens) {
            window.setFrame(restored, display: false)
            return
        }
        let visible = NSScreen.main?.visibleFrame
            ?? screens.first
            ?? CGRect(origin: .zero, size: Self.defaultSize)
        window.setFrame(Self.defaultFrame(in: visible), display: false)
    }
}
