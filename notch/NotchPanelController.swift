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

/// Transparent click-catcher under the expanded notch — no Input Monitoring required.
@MainActor
private final class NotchCollapseBackdropPanel: NSPanel {
    override var canBecomeKey: Bool { false }

    override init(
        contentRect: NSRect,
        styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType,
        defer flag: Bool
    ) {
        super.init(contentRect: contentRect, styleMask: style, backing: backingStoreType, defer: flag)
        isFloatingPanel = true
        // Just below the notch panel (`.floating`).
        level = NSWindow.Level(rawValue: NSWindow.Level.floating.rawValue - 1)
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
        isReleasedWhenClosed = false
        backgroundColor = .clear
        isOpaque = false
        hasShadow = false
        ignoresMouseEvents = false
        isMovable = false
        hidesOnDeactivate = false
        titleVisibility = .hidden
        titlebarAppearsTransparent = true
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }
}

@MainActor
private final class NotchBackdropClickView: NSView {
    var onClick: (() -> Void)?

    override func hitTest(_ point: NSPoint) -> NSView? { self }

    override func mouseDown(with event: NSEvent) {
        onClick?()
    }

    override func acceptsFirstMouse(for event: NSEvent?) -> Bool { true }
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
    /// Full-screen click-away layer under the expanded notch (primary outside-dismiss; no IM).
    private var backdropPanel: NotchCollapseBackdropPanel?
    private var escCollapseMonitor: Any?
    /// Expanded panel body size — locked for session to avoid resize flicker on content changes.
    private var sessionLockedExpandedContentSize: CGSize?
    /// Pending frame snap back to the pill — held while the collapse exit fade plays.
    /// Cancelled when a second ⌥Space re-expands mid-flight (interruptible summon).
    private var collapseFrameWorkItem: DispatchWorkItem?
    /// Screen-space grab for collapsed-pill drag (not SwiftUI translation — window would fight itself).
    private var pillDragStartMouse: NSPoint?
    private var pillDragStartOrigin: NSPoint?
    private var pillDragHistory: [(t: CFTimeInterval, p: NSPoint)] = []
    private var isDraggingCollapsedPill = false
    private var islandSpring: CollapsedIslandMotion.SpringState?
    private var islandSpringTimer: Timer?
    private var lastSpringTick: CFTimeInterval = 0
    /// True while `screencapture -i` runs — do not orderFront the HUD over the region picker.
    private var chromeHiddenForCapture = false
    private var windowsHiddenForCapture: [NSWindow] = []

    private let hostedExpandedContent: (() -> AnyView)?

    init(viewModel: NotchViewModel, hostedExpandedContent: (() -> AnyView)? = nil) {
        self.viewModel = viewModel
        self.hostedExpandedContent = hostedExpandedContent
        let host = NSHostingController(
            rootView: NotchRootView(vm: viewModel, hostedExpandedContent: hostedExpandedContent)
        )
        // The pre-warmed expanded layer is laid out at final size while the window is the
        // collapsed pill — never let SwiftUI fitting size drive the panel frame.
        host.sizingOptions = []
        self.hosting = host

        let panel = TradeAutopsyNotchPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.contentViewController = host
        self.panel = panel

        viewModel.onRequestOrderFront = { [weak self] in
            guard let self, !self.chromeHiddenForCapture else { return }
            if self.viewModel.isExpanded {
                self.showCollapseBackdropIfNeeded()
            }
            self.panel.orderFrontRegardless()
            if let view = self.panel.contentView {
                _ = self.panel.makeFirstResponder(view)
            }
        }

        viewModel.onRequestHidePill = { [weak self] in
            self?.hide()
        }

        viewModel.onCollapsedPillDragFromScreen = { [weak self] in
            self?.applyCollapsedPillDragFromScreen()
        }

        viewModel.onCollapsedPillDragEnded = { [weak self] in
            self?.endCollapsedPillDrag()
        }

        // No `.receive(on:)` hop — the view model is @MainActor, so the frame snap lands on
        // the same run-loop turn as the hotkey (Spotlight: respond on the event).
        viewModel.$isExpanded
            .dropFirst()
            .sink { [weak self] expanded in
                guard let self, !self.chromeHiddenForCapture else { return }
                self.panel.acceptsMouseMovedEvents = !expanded
                self.collapseFrameWorkItem?.cancel()
                self.collapseFrameWorkItem = nil
                self.cancelIslandSpring(applySlot: false)
                self.viewModel.collapsedPillPressed = false
                if expanded {
                    // Snap to the final frame instantly; only opacity + scale animate (SwiftUI).
                    self.layoutPanel(expanded: true)
                    self.viewModel.summonPanelAtExpandedFrame = true
                    self.showCollapseBackdropIfNeeded()
                    self.installEscCollapseMonitorIfNeeded()
                } else {
                    self.hideCollapseBackdrop()
                    self.removeEscCollapseMonitor()
                    // Hold the expanded frame while the exit fade plays, then snap to the pill.
                    let work = DispatchWorkItem { [weak self] in
                        guard let self, !self.viewModel.isExpanded else { return }
                        self.viewModel.summonPanelAtExpandedFrame = false
                        self.layoutPanel(expanded: false)
                    }
                    self.collapseFrameWorkItem = work
                    DispatchQueue.main.asyncAfter(
                        deadline: .now() + NotchTheme.summonFrameHoldDuration,
                        execute: work
                    )
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
                guard let self, !self.chromeHiddenForCapture else { return }
                self.layoutPanel(expanded: viewModel.isExpanded)
                if self.viewModel.isExpanded {
                    self.layoutCollapseBackdrop()
                }
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
        if let m = escCollapseMonitor {
            NSEvent.removeMonitor(m)
        }
        if let o = displayObserver {
            NotificationCenter.default.removeObserver(o)
        }
        if let o = moveObserver {
            NotificationCenter.default.removeObserver(o)
        }
        saveWorkItem?.cancel()
        collapseFrameWorkItem?.cancel()
        islandSpringTimer?.invalidate()
        if let backdrop = backdropPanel {
            Task { @MainActor in
                backdrop.orderOut(nil)
            }
        }
    }

    func show() {
        guard !chromeHiddenForCapture else { return }
        panel.acceptsMouseMovedEvents = !viewModel.isExpanded
        layoutPanel(expanded: viewModel.isExpanded)
        panel.orderFrontRegardless()
        if let view = panel.contentView {
            _ = panel.makeFirstResponder(view)
        }
        // Second layout pass: `NSScreen.main` / frames can be wrong on first launch tick.
        DispatchQueue.main.async { [weak self] in
            guard let self else { return }
            self.layoutPanel(expanded: self.viewModel.isExpanded)
        }
        // Pre-warm the click-away backdrop so first ⌥Space doesn't allocate a window.
        if backdropPanel == nil {
            backdropPanel = makeCollapseBackdropPanel()
        }
    }

    /// The pill/panel window is on screen. Station-hosted ⌥Space must never turn this off —
    /// it expands and collapses PLAN, unlike the standalone `toggleVisibility` path.
    var isPanelVisible: Bool { panel.isVisible }

    var panelFrame: NSRect { panel.frame }

    /// Force the always-mounted expanded layer through a real layout + display pass while it is
    /// still invisible, so the first ⌥Space composites an already-built tree instead of paying
    /// for the PLAN mount inside the first frames of the summon spring.
    func prewarmExpandedSurface() {
        guard !chromeHiddenForCapture else { return }
        hosting.view.layoutSubtreeIfNeeded()
        hosting.view.displayIfNeeded()
    }

    /// Keep the HUD above the focused app without stealing activation.
    func bringToFront() {
        if !panel.isVisible {
            show()
        } else {
            if viewModel.isExpanded {
                showCollapseBackdropIfNeeded()
            }
            panel.orderFrontRegardless()
        }
    }

    func hide() {
        hideCollapseBackdrop()
        removeEscCollapseMonitor()
        panel.orderOut(nil)
    }

    func hideChromeForInteractiveCapture() {
        chromeHiddenForCapture = true
        hideCollapseBackdrop()
        windowsHiddenForCapture = NSApp.windows.filter(\.isVisible)
        for w in windowsHiddenForCapture {
            w.orderOut(nil)
        }
    }

    func restoreChromeAfterInteractiveCapture() {
        chromeHiddenForCapture = false
        for w in windowsHiddenForCapture {
            w.orderFrontRegardless()
        }
        windowsHiddenForCapture = []
        layoutPanel(expanded: viewModel.isExpanded)
        if viewModel.isExpanded {
            showCollapseBackdropIfNeeded()
            installEscCollapseMonitorIfNeeded()
        }
        panel.orderFrontRegardless()
        panel.acceptsMouseMovedEvents = !viewModel.isExpanded
    }

    private func showCollapseBackdropIfNeeded() {
        let backdrop = backdropPanel ?? makeCollapseBackdropPanel()
        backdropPanel = backdrop
        layoutCollapseBackdrop()
        backdrop.orderFrontRegardless()
        panel.orderFrontRegardless()
    }

    private func makeCollapseBackdropPanel() -> NotchCollapseBackdropPanel {
        let backdrop = NotchCollapseBackdropPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        let clickView = NotchBackdropClickView(frame: .zero)
        clickView.wantsLayer = true
        clickView.layer?.backgroundColor = NSColor.black.withAlphaComponent(0.08).cgColor
        clickView.autoresizingMask = [.width, .height]
        clickView.onClick = { [weak self] in
            self?.viewModel.collapseExpandedFromOutsideClick()
        }
        backdrop.contentView = clickView
        return backdrop
    }

    private func layoutCollapseBackdrop() {
        guard let backdrop = backdropPanel else { return }
        let screen = panel.screen ?? NSScreen.main
        let frame = screen?.frame ?? .zero
        backdrop.setFrame(frame, display: true)
    }

    private func hideCollapseBackdrop() {
        backdropPanel?.orderOut(nil)
    }

    private func installEscCollapseMonitorIfNeeded() {
        guard escCollapseMonitor == nil else { return }
        escCollapseMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, self.viewModel.isExpanded, event.keyCode == 53 else { return event }
            DispatchQueue.main.async {
                self.viewModel.collapseExpandedFromOutsideClick()
            }
            return nil
        }
    }

    private func removeEscCollapseMonitor() {
        if let monitor = escCollapseMonitor {
            NSEvent.removeMonitor(monitor)
            escCollapseMonitor = nil
        }
    }

    /// Standalone Notch: show/hide the whole HUD.
    func toggleVisibility() {
        if panel.isVisible {
            hide()
        } else {
            show()
        }
    }

    /// Station-hosted: expand/collapse PLAN surface while keeping the pill above other apps.
    /// Flip state first — first pixels move on the same run loop as the hotkey; expand
    /// orderFronts via `onRequestOrderFront`, so a separate bringToFront is pure extra
    /// compositor work before anything visible changes.
    func toggleExpandedSurface() {
        if viewModel.isExpanded {
            viewModel.collapseExpandedFromChromeTap()
        } else {
            viewModel.expandFromCollapsedChromeTap()
            if !panel.isVisible {
                show()
            }
        }
    }

    /// Legacy name — visibility toggle (standalone hotkey path).
    func toggle() {
        toggleVisibility()
    }

    private func handlePanelMoved() {
        guard !isApplyingSnappedFrame, !isDraggingCollapsedPill else { return }
        guard !viewModel.isExpanded else { return }
        if viewModel.hasPhysicalNotch { return }
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

    private func applyCollapsedPillDragFromScreen() {
        guard !viewModel.isExpanded else { return }
        cancelIslandSpring(applySlot: false)
        let mouse = NSEvent.mouseLocation
        if viewModel.hasPhysicalNotch {
            applyNotchedIslandDrag(mouse: mouse)
            return
        }
        if pillDragStartMouse == nil {
            pillDragStartMouse = mouse
            pillDragStartOrigin = panel.frame.origin
            isDraggingCollapsedPill = true
        }
        guard let startMouse = pillDragStartMouse, let startOrigin = pillDragStartOrigin else { return }
        var f = panel.frame
        f.origin = NSPoint(
            x: startOrigin.x + (mouse.x - startMouse.x),
            y: startOrigin.y + (mouse.y - startMouse.y)
        )
        if let screen = panel.screen ?? NSScreen.main {
            f = clampFrame(f, to: screen.visibleFrame)
        }
        isApplyingSnappedFrame = true
        panel.setFrame(f, display: true)
        isApplyingSnappedFrame = false
    }

    /// Tug the island 1:1, rubber-band past the volume slot. Reduce Motion: stay home.
    private func applyNotchedIslandDrag(mouse: NSPoint) {
        if NSWorkspace.shared.accessibilityDisplayShouldReduceMotion { return }
        guard let screen = panel.screen ?? NSScreen.main else { return }
        let slot = collapsedSlotFrame(on: screen)
        if pillDragStartMouse == nil {
            pillDragStartMouse = mouse
            pillDragStartOrigin = panel.frame.origin
            isDraggingCollapsedPill = true
            pillDragHistory = [(CACurrentMediaTime(), mouse)]
        }
        pillDragHistory.append((CACurrentMediaTime(), mouse))
        if pillDragHistory.count > 6 {
            pillDragHistory.removeFirst()
        }
        guard let startMouse = pillDragStartMouse, let startOrigin = pillDragStartOrigin else { return }
        let unconstrained = CGPoint(
            x: startOrigin.x + (mouse.x - startMouse.x),
            y: startOrigin.y + (mouse.y - startMouse.y)
        )
        var f = slot
        f.origin = CollapsedIslandMotion.rubberbandedOrigin(unconstrained: unconstrained, slot: slot)
        isApplyingSnappedFrame = true
        panel.setFrame(f, display: true)
        isApplyingSnappedFrame = false
    }

    private func endCollapsedPillDrag() {
        let notched = viewModel.hasPhysicalNotch
        let grabbed = pillDragStartMouse != nil
        let history = pillDragHistory
        isDraggingCollapsedPill = false
        pillDragStartMouse = nil
        pillDragStartOrigin = nil
        pillDragHistory = []
        if notched {
            // Never persist a parked origin on a notched Mac — that is a second blob.
            if grabbed {
                startIslandSpringHome(history: history)
            } else {
                snapIslandToSlot()
            }
            return
        }
        handlePanelMoved()
        persistOriginNow(panel.frame.origin)
    }

    private func persistOriginNow(_ origin: NSPoint) {
        saveWorkItem?.cancel()
        UserDefaults.standard.set(origin.x, forKey: Persistence.originX)
        UserDefaults.standard.set(origin.y, forKey: Persistence.originY)
    }

    private func collapsedSlotFrame(on screen: NSScreen) -> NSRect {
        let frame = screen.frame
        let vf = screen.visibleFrame
        let inset = screen.safeAreaInsets
        if inset.top > 0 {
            let housing = BarNotchVolumeSlot.hardwareNotch(
                screenFrame: frame,
                leftMenu: screen.auxiliaryTopLeftArea ?? .zero,
                rightMenu: screen.auxiliaryTopRightArea ?? .zero,
            )
            return BarNotchVolumeSlot.collapsedFrame(
                screenFrame: frame,
                visibleFrame: vf,
                notchLeft: housing?.minX ?? (frame.midX - BarNotchChrome.collapsedPillWidth / 2),
                notchWidth: housing?.width ?? BarNotchChrome.collapsedPillWidth,
                notchInset: inset.top,
                fallbackWidth: BarNotchChrome.collapsedPillWidth,
            )
        }
        let h = BarNotchChrome.collapsedStripHeight
        let w = BarNotchChrome.collapsedPillWidth
        return NSRect(x: vf.midX - w / 2, y: vf.maxY - 8 - h, width: w, height: h)
    }

    private func snapIslandToSlot() {
        guard let screen = panel.screen ?? NSScreen.main else { return }
        let slot = collapsedSlotFrame(on: screen)
        isApplyingSnappedFrame = true
        panel.setFrame(slot, display: true)
        isApplyingSnappedFrame = false
    }

    private func cancelIslandSpring(applySlot: Bool) {
        islandSpringTimer?.invalidate()
        islandSpringTimer = nil
        islandSpring = nil
        if applySlot {
            snapIslandToSlot()
        }
    }

    private func startIslandSpringHome(history: [(t: CFTimeInterval, p: NSPoint)]) {
        guard let screen = panel.screen ?? NSScreen.main else { return }
        let slot = collapsedSlotFrame(on: screen)
        let target = CollapsedIslandMotion.springHomeTarget(slot: slot)
        if NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
            snapIslandToSlot()
            return
        }
        let vel = dragReleaseVelocity(history)
        let speed = hypot(vel.dx, vel.dy)
        let state = CollapsedIslandMotion.SpringState(
            x: panel.frame.origin.x,
            y: panel.frame.origin.y,
            vx: vel.dx,
            vy: vel.dy,
            target: target,
            dampingRatio: CollapsedIslandMotion.dampingRatio(releaseSpeed: speed)
        )
        islandSpring = state
        lastSpringTick = CACurrentMediaTime()
        islandSpringTimer?.invalidate()
        let timer = Timer(timeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.stepIslandSpring()
            }
        }
        RunLoop.main.add(timer, forMode: .common)
        islandSpringTimer = timer
    }

    private func dragReleaseVelocity(
        _ history: [(t: CFTimeInterval, p: NSPoint)],
    ) -> CGVector {
        guard history.count >= 2 else { return .zero }
        let a = history[0]
        let b = history[history.count - 1]
        let dt = b.t - a.t
        guard dt > 0 else { return .zero }
        return CGVector(dx: (b.p.x - a.p.x) / dt, dy: (b.p.y - a.p.y) / dt)
    }

    private func stepIslandSpring() {
        guard var spring = islandSpring else {
            cancelIslandSpring(applySlot: false)
            return
        }
        let now = CACurrentMediaTime()
        let dt = CGFloat(min(0.032, now - lastSpringTick))
        lastSpringTick = now
        spring.step(dt: dt)
        var f = panel.frame
        f.origin = CGPoint(x: spring.x, y: spring.y)
        isApplyingSnappedFrame = true
        panel.setFrame(f, display: true)
        isApplyingSnappedFrame = false
        if spring.isSettled {
            cancelIslandSpring(applySlot: true)
        } else {
            islandSpring = spring
        }
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

    /// Spotlight rule: the window is never size-animated. Expand snaps to the final rect and
    /// SwiftUI opacity/scale is the only motion; collapse snaps after the exit fade.
    private func layoutPanel(expanded: Bool) {
        guard !chromeHiddenForCapture else { return }
        guard !isDraggingCollapsedPill, islandSpring == nil else { return }
        guard let screen = NSScreen.main else { return }
        let frame = screen.frame
        let vf = screen.visibleFrame
        let inset = screen.safeAreaInsets
        let hasNotch = inset.top > 0
        let notchTopInset = inset.top

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

        let collapsed = collapsedSlotFrame(on: screen)

        let expandedX: CGFloat
        let expandedY: CGFloat
        if hasNotch {
            expandedX = max(vf.minX, min(frame.midX - expandedW / 2, vf.maxX - expandedW))
            expandedY = min(vf.maxY, frame.maxY) - expandedH
        } else {
            expandedX = vf.midX - expandedW / 2
            expandedY = vf.maxY - 8 - expandedH
        }

        // Clamp the expanded rect in both branches: the pre-warmed SwiftUI layer is laid out at
        // this size while the window is still the pill, so it has to be the size the window will
        // actually take — a small display clamps `expandedW`/`expandedH` down to `visibleFrame`.
        let expandedRect = clampFrame(
            NSRect(x: expandedX, y: expandedY, width: expandedW, height: expandedH),
            to: vf
        )
        if viewModel.expandedSurfaceSize != expandedRect.size {
            viewModel.expandedSurfaceSize = expandedRect.size
        }

        var appliedRect: NSRect
        if expanded {
            appliedRect = expandedRect
        } else {
            appliedRect = hasNotch ? collapsed : clampFrame(collapsed, to: vf)
        }
        // Notched collapsed chip stays in the volume HUD slot. Saved drag origin
        // used to park it under the menu bar — a second blob.
        if !expanded, !hasNotch, let saved = savedOrigin() {
            var candidate = appliedRect
            candidate.origin = snapOrigin(saved, in: vf)
            candidate = clampFrame(candidate, to: vf)
            if frameReasonablyOnScreen(candidate) {
                appliedRect = candidate
            }
        }

        // Guarded: this runs inside `$isExpanded`'s willSet, so an unconditional assignment
        // would publish a redundant change on every summon.
        let resolvedTopInset = hasNotch ? notchTopInset : 0
        if viewModel.notchTopInset != resolvedTopInset {
            viewModel.notchTopInset = resolvedTopInset
        }
        panel.level = hasNotch && !expanded ? .statusBar : .floating

        panel.setFrame(appliedRect, display: true)

        if expanded {
            layoutCollapseBackdrop()
        }
    }
}
