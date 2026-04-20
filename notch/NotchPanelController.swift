import AppKit
import Combine
import QuartzCore
import SwiftUI

/// BoringNotch-style borderless floating panel.
@MainActor
final class TradeAutopsyNotchPanel: NSPanel {
    override init(
        contentRect: NSRect,
        styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType,
        defer flag: Bool
    ) {
        super.init(contentRect: contentRect, styleMask: style, backing: backingStoreType, defer: flag)
        isFloatingPanel = true
        level = .screenSaver
        collectionBehavior = [.fullScreenAuxiliary, .canJoinAllSpaces]
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

/// Hosts the notch UI in a borderless `NSPanel` (DynamicNotchKit-free for portable `swift build`).
@MainActor
final class NotchPanelController {
    private let panel: TradeAutopsyNotchPanel
    private let hosting: NSHostingController<NotchRootView>
    private let viewModel: NotchViewModel
    private var subs = Set<AnyCancellable>()
    private var displayObserver: NSObjectProtocol?

    init(viewModel: NotchViewModel) {
        self.viewModel = viewModel
        let host = NSHostingController(rootView: NotchRootView(vm: viewModel))
        self.hosting = host

        let panel = TradeAutopsyNotchPanel(
            contentRect: .zero,
            styleMask: [.nonactivatingPanel, .borderless, .fullSizeContentView],
            backing: .buffered,
            defer: false
        )
        panel.contentViewController = host
        self.panel = panel

        viewModel.onRequestOrderFront = { [weak panel] in
            panel?.orderFrontRegardless()
        }

        viewModel.$isExpanded
            .receive(on: DispatchQueue.main)
            .sink { [weak self] _ in
                self?.layoutPanel(animated: true)
            }
            .store(in: &subs)

        displayObserver = NotificationCenter.default.addObserver(
            forName: NSApplication.didChangeScreenParametersNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            Task { @MainActor in
                self?.layoutPanel(animated: false)
            }
        }
    }

    deinit {
        if let o = displayObserver {
            NotificationCenter.default.removeObserver(o)
        }
    }

    func show() {
        layoutPanel(animated: false)
        panel.orderFrontRegardless()
    }

    func hide() {
        panel.orderOut(nil)
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
        let expandedW: CGFloat = 940
        let expandedContentH: CGFloat = 200
        let expandedH = expandedContentH + (hasNotch ? notchTopInset : 0)

        let collapsedH: CGFloat
        let collapsedW: CGFloat
        let collapsedX: CGFloat
        let collapsedY: CGFloat
        if hasNotch {
            collapsedH = max(notchTopInset, 26)
            collapsedW = notchWidth + 16
            collapsedX = frame.minX + inset.left - 8
            collapsedY = frame.maxY - collapsedH
        } else {
            collapsedH = 34
            collapsedW = max(200, 280)
            collapsedX = vf.midX - collapsedW / 2
            collapsedY = vf.maxY - 4 - collapsedH
        }

        let expandedX: CGFloat
        let expandedY: CGFloat
        if hasNotch {
            expandedX = frame.midX - expandedW / 2
            expandedY = frame.maxY - expandedH
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

        viewModel.notchTopInset = hasNotch ? notchTopInset : 0

        if animated {
            NSAnimationContext.runAnimationGroup { ctx in
                ctx.duration = 0.4
                ctx.timingFunction = CAMediaTimingFunction(name: .easeInEaseOut)
                panel.animator().setFrame(rect, display: true)
            }
        } else {
            panel.setFrame(rect, display: true)
        }
    }
}
