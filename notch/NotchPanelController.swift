import AppKit
import Combine
import QuartzCore
import SwiftUI

/// Borderless floating HUD (`.nonactivatingPanel`) — stays above all apps; text fields work via
/// `makeFirstResponder` on mouse-down instead of stealing key from the active app.
@MainActor
final class TradeAutopsyNotchPanel: NSPanel {
    override var canBecomeKey: Bool { true }

    override init(
        contentRect: NSRect,
        styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType,
        defer flag: Bool
    ) {
        super.init(contentRect: contentRect, styleMask: style, backing: backingStoreType, defer: flag)
        isFloatingPanel = true
        level = .floating
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
        isReleasedWhenClosed = false
        backgroundColor = .clear
        isOpaque = false
        hasShadow = false
        ignoresMouseEvents = false
        isMovable = true
        isMovableByWindowBackground = false
        acceptsMouseMovedEvents = true
        hidesOnDeactivate = false
        titleVisibility = .hidden
        titlebarAppearsTransparent = true
    }

    override func sendEvent(_ event: NSEvent) {
        if event.type == .leftMouseDown {
            // Don't call `makeKey()` — it fights `.nonactivatingPanel` and steals activation.
            if firstResponder === self || firstResponder == nil {
                makeFirstResponder(contentView)
            }
        }
        super.sendEvent(event)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

/// Hosts the notch UI in a borderless `NSPanel` (DynamicNotchKit-free for portable `swift build`).
@MainActor
final class NotchPanelController {
    private enum Persistence {
        static let originX = "tradeautopsy.notch.origin.x"
        static let originY = "tradeautopsy.notch.origin.y"
    }

    private let panel: TradeAutopsyNotchPanel
    private let hosting: NSHostingController<NotchRootView>
    private let viewModel: NotchViewModel
    private var subs = Set<AnyCancellable>()
    private var displayObserver: NSObjectProtocol?
    private var moveObserver: NSObjectProtocol?
    private var saveWorkItem: DispatchWorkItem?
    private var isApplyingSnappedFrame = false
    /// Tracks off-app mouse downs to collapse expanded panel (global monitor ignores same-app clicks).
    private var outsideCollapseMouseMonitor: Any?
    /// Expanded panel body size — locked for session to avoid resize flicker on content changes.
    private var sessionLockedExpandedContentSize: CGSize?

    private let hostedExpandedContent: (() -> AnyView)?

    init(viewModel: NotchViewModel, hostedExpandedContent: (() -> AnyView)? = nil) {
        self.viewModel = viewModel
        self.hostedExpandedContent = hostedExpandedContent
        let host = NSHostingController(
            rootView: NotchRootView(vm: viewModel, hostedExpandedContent: hostedExpandedContent)
        )
        self.hosting = host

        let panel = TradeAutopsyNotchPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.contentViewController = host
        self.panel = panel

        viewModel.onRequestOrderFront = { [weak panel] in
            panel?.orderFrontRegardless()
            if let view = panel?.contentView {
                _ = panel?.makeFirstResponder(view)
            }
        }

        viewModel.$isExpanded
            .receive(on: DispatchQueue.main)
            .sink { [weak self] expanded in
                guard let self else { return }
                self.layoutPanel(animated: true)
                if expanded {
                    self.installOutsideCollapseMouseMonitorIfNeeded()
                } else {
                    self.removeOutsideCollapseMouseMonitor()
                }
            }
            .store(in: &subs)

        // Expanded width/height follow `visibleFrame`; refresh when screen geometry changes.
        displayObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.layoutPanel(animated: false)
            }
        }

        moveObserver = NotificationCenter.default.addObserver(
            forName: NSWindow.didMoveNotification,
            object: panel,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.handlePanelMoved()
            }
        }
    }

    deinit {
        if let m = outsideCollapseMouseMonitor {
            NSEvent.removeMonitor(m)
        }
        if let o = displayObserver {
            NotificationCenter.default.removeObserver(o)
        }
        if let o = moveObserver {
            NotificationCenter.default.removeObserver(o)
        }
        saveWorkItem?.cancel()
    }

    func show() {
        layoutPanel(animated: false)
        panel.orderFrontRegardless()
        if let view = panel.contentView {
            _ = panel.makeFirstResponder(view)
        }
        // Second layout pass: `NSScreen.main` / frames can be wrong on first launch tick.
        DispatchQueue.main.async { [weak self] in
            self?.layoutPanel(animated: false)
        }
    }

    func hide() {
        removeOutsideCollapseMouseMonitor()
        panel.orderOut(nil)
    }

    /// `addGlobalMonitorForEvents` only delivers mouse downs from processes other than this app —
    /// equivalent to clicking "outside" the notch surface for dismiss.
    private func installOutsideCollapseMouseMonitorIfNeeded() {
        guard outsideCollapseMouseMonitor == nil else { return }
        outsideCollapseMouseMonitor = NSEvent.addGlobalMonitorForEvents(matching: .leftMouseDown) { [weak self] _ in
            DispatchQueue.main.async {
                self?.viewModel.collapseExpandedFromOutsideClick()
            }
        }
    }

    private func removeOutsideCollapseMouseMonitor() {
        if let monitor = outsideCollapseMouseMonitor {
            NSEvent.removeMonitor(monitor)
            outsideCollapseMouseMonitor = nil
        }
    }

    func toggle() {
        if panel.isVisible {
            hide()
        } else {
            show()
        }
    }

    private func handlePanelMoved() {
        guard !isApplyingSnappedFrame else { return }
        guard let screen = panel.screen ?? NSScreen.main else { return }
        let vf = screen.visibleFrame
        var f = panel.frame
        f.origin = snapOrigin(f.origin, in: vf)
        f = clampFrame(f, to: vf)
        if f.origin != panel.frame.origin || f.size != panel.frame.size {
            isApplyingSnappedFrame = true
            panel.setFrame(f, display: true)
            isApplyingSnappedFrame = false
        }
        schedulePersist(origin: f.origin)
    }

    private func schedulePersist(origin: NSPoint) {
        saveWorkItem?.cancel()
        let work = DispatchWorkItem {
            UserDefaults.standard.set(origin.x, forKey: Persistence.originX)
            UserDefaults.standard.set(origin.y, forKey: Persistence.originY)
        }
        saveWorkItem = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.25, execute: work)
    }

    private func savedOrigin() -> NSPoint? {
        let d = UserDefaults.standard
        guard d.object(forKey: Persistence.originX) != nil,
              d.object(forKey: Persistence.originY) != nil
        else { return nil }
        return NSPoint(
            x: d.double(forKey: Persistence.originX),
            y: d.double(forKey: Persistence.originY)
        )
    }

    private func snapOrigin(_ origin: NSPoint, in visibleFrame: NSRect) -> NSPoint {
        let threshold: CGFloat = 20
        let w = panel.frame.width
        let h = panel.frame.height
        var out = origin
        if abs(out.x - visibleFrame.minX) <= threshold {
            out.x = visibleFrame.minX
        }
        if abs(out.x - (visibleFrame.maxX - w)) <= threshold {
            out.x = visibleFrame.maxX - w
        }
        if abs(out.y - visibleFrame.minY) <= threshold {
            out.y = visibleFrame.minY
        }
        if abs(out.y - (visibleFrame.maxY - h)) <= threshold {
            out.y = visibleFrame.maxY - h
        }
        return out
    }

    /// Keep the full panel frame inside the visible usable rect (menu bar / dock aware).
    private func clampFrame(_ rect: NSRect, to visible: NSRect) -> NSRect {
        var r = rect
        if r.width > visible.width {
            r.origin.x = visible.minX
            r.size.width = min(r.width, visible.width)
        }
        if r.height > visible.height {
            r.origin.y = visible.minY
            r.size.height = min(r.height, visible.height)
        }
        if r.maxX > visible.maxX {
            r.origin.x = visible.maxX - r.width
        }
        if r.minX < visible.minX {
            r.origin.x = visible.minX
        }
        if r.maxY > visible.maxY {
            r.origin.y = visible.maxY - r.height
        }
        if r.minY < visible.minY {
            r.origin.y = visible.minY
        }
        return r
    }

    /// True if a meaningful portion of the window intersects some screen (catch stale multi-monitor saves).
    private func frameReasonablyOnScreen(_ rect: NSRect) -> Bool {
        let area = rect.width * rect.height
        guard area > 1 else { return false }
        for scr in NSScreen.screens {
            let inter = rect.intersection(scr.frame)
            if inter.width * inter.height > area * 0.25 {
                return true
            }
        }
        return false
    }

    private func layoutPanel(animated: Bool) {
        guard let screen = NSScreen.main else { return }
        let frame = screen.frame
        let vf = screen.visibleFrame
        let inset = screen.safeAreaInsets
        let hasNotch = inset.top > 0
        let notchTopInset = inset.top
        let notchWidth = max(0, frame.width - inset.left - inset.right)

        let expanded = viewModel.isExpanded
        let expandedW: CGFloat
        let expandedContentH: CGFloat
        if let locked = sessionLockedExpandedContentSize {
            expandedW = locked.width
            expandedContentH = locked.height
        } else {
            let w = NotchPanelLayout.expandedWidth(in: vf)
            let ch = NotchPanelLayout.expandedContentHeight(in: vf)
            sessionLockedExpandedContentSize = CGSize(width: w, height: ch)
            expandedW = w
            expandedContentH = ch
        }
        let expandedH = expandedContentH + (hasNotch ? notchTopInset : 0)

        let collapsedH: CGFloat
        let collapsedW: CGFloat
        let collapsedX: CGFloat
        let collapsedY: CGFloat
        if hasNotch {
            collapsedH = max(notchTopInset, 26)
            collapsedW = max(notchWidth + 16, 120)
            // Align with safe-area “notch strip”; clamp X so we never sit off-screen after inset math.
            collapsedX = max(vf.minX, min(frame.minX + inset.left - 8, vf.maxX - collapsedW))
            // Use visible top so the pill stays in the live menu-bar region (avoids sitting under camera housing math drift).
            collapsedY = min(vf.maxY, frame.maxY) - collapsedH
        } else {
            collapsedH = 34
            collapsedW = max(200, 280)
            collapsedX = vf.midX - collapsedW / 2
            collapsedY = vf.maxY - 8 - collapsedH
        }

        let expandedX: CGFloat
        let expandedY: CGFloat
        if hasNotch {
            expandedX = max(vf.minX, min(frame.midX - expandedW / 2, vf.maxX - expandedW))
            expandedY = min(vf.maxY, frame.maxY) - expandedH
        } else {
            expandedX = vf.midX - expandedW / 2
            expandedY = vf.maxY - 8 - expandedH
        }

        let rect: NSRect
        if expanded {
            rect = NSRect(x: expandedX, y: expandedY, width: expandedW, height: expandedH)
        } else {
            rect = NSRect(x: collapsedX, y: collapsedY, width: collapsedW, height: collapsedH)
        }
        var appliedRect = rect
        if let saved = savedOrigin() {
            var candidate = appliedRect
            candidate.origin = snapOrigin(saved, in: vf)
            candidate = clampFrame(candidate, to: vf)
            if frameReasonablyOnScreen(candidate) {
                appliedRect = candidate
            }
            // else: stale save (e.g. unplugged monitor) — use default `rect` above
        } else {
            appliedRect = clampFrame(appliedRect, to: vf)
        }

        viewModel.notchTopInset = hasNotch ? notchTopInset : 0

        if animated {
            NSAnimationContext.runAnimationGroup { ctx in
                ctx.duration = 0.4
                ctx.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
                panel.animator().setFrame(appliedRect, display: true)
            }
        } else {
            panel.setFrame(appliedRect, display: true)
        }
    }
}
