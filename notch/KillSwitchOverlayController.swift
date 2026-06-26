import AppKit
import Combine
import SwiftUI

/// Borderless fullscreen panel above browsers (screensaver level) for L2/L3 overlay (#189).
@MainActor
final class KillSwitchOverlayPanel: NSPanel {
    override var canBecomeKey: Bool { true }

    override init(
        contentRect: NSRect,
        styleMask style: NSWindow.StyleMask,
        backing backingStoreType: NSWindow.BackingStoreType,
        defer flag: Bool
    ) {
        super.init(contentRect: contentRect, styleMask: style, backing: backingStoreType, defer: flag)
        isFloatingPanel = true
        level = .screenSaver
        collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .moveToActiveSpace]
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

/// Shows/hides the kill-switch overlay when `NotchViewModel.killSwitchOverlayVisible` changes.
@MainActor
final class KillSwitchOverlayController {
    private let viewModel: NotchViewModel
    private var panel: KillSwitchOverlayPanel?
    private var hosting: NSHostingController<KillSwitchOverlayView>?
    private var subs = Set<AnyCancellable>()

    init(viewModel: NotchViewModel) {
        self.viewModel = viewModel
        viewModel.$killSwitchOverlayVisible
            .receive(on: DispatchQueue.main)
            .sink { [weak self] visible in
                if visible {
                    self?.show()
                } else {
                    self?.hide()
                }
            }
            .store(in: &subs)
    }

    func show() {
        guard panel == nil else {
            layoutToMainScreen()
            panel?.orderFrontRegardless()
            return
        }

        let host = NSHostingController(rootView: KillSwitchOverlayView(viewModel: viewModel))
        hosting = host

        let panel = KillSwitchOverlayPanel(
            contentRect: .zero,
            styleMask: [.borderless, .nonactivatingPanel],
            backing: .buffered,
            defer: false
        )
        panel.contentViewController = host
        self.panel = panel
        layoutToMainScreen()
        panel.orderFrontRegardless()
        _ = panel.makeFirstResponder(host.view)
    }

    func hide() {
        panel?.orderOut(nil)
    }

    private func layoutToMainScreen() {
        guard let panel, let screen = NSScreen.main else { return }
        panel.setFrame(screen.frame, display: true)
    }
}
