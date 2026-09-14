import AppKit
import Foundation

/// Full-screen drag-to-crop on a frozen capture. Esc or a tiny drag cancels.
@MainActor
enum JournalRegionPicker {
    static func crop(from image: NSImage, screenFrame: NSRect) async throws -> NSImage {
        try await withCheckedThrowingContinuation { continuation in
            let panel = TradeAutopsyNotchPanel(
                contentRect: screenFrame,
                styleMask: [.borderless, .nonactivatingPanel],
                backing: .buffered,
                defer: false
            )
            panel.level = .screenSaver
            panel.collectionBehavior = [.canJoinAllSpaces, .fullScreenAuxiliary, .ignoresCycle]
            panel.isOpaque = true
            panel.backgroundColor = .black
            panel.ignoresMouseEvents = false
            panel.acceptsMouseMovedEvents = false

            let view = RegionPickView(frame: NSRect(origin: .zero, size: screenFrame.size))
            view.image = image
            var finished = false
            view.onFinish = { rect in
                guard !finished else { return }
                finished = true
                panel.orderOut(nil)
                panel.contentView = nil
                guard let rect, rect.width >= 8, rect.height >= 8 else {
                    continuation.resume(throwing: JournalInteractiveScreenshotError.cancelled)
                    return
                }
                guard let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
                    continuation.resume(throwing: JournalInteractiveScreenshotError.failed)
                    return
                }
                let pixel = RegionCrop.pixelRect(
                    viewRect: rect,
                    imageWidth: cg.width,
                    imageHeight: cg.height,
                    viewSize: view.bounds.size
                )
                guard pixel.width >= 2, pixel.height >= 2,
                      let sliced = cg.cropping(to: pixel)
                else {
                    continuation.resume(throwing: JournalInteractiveScreenshotError.failed)
                    return
                }
                let out = NSImage(cgImage: sliced, size: NSSize(width: sliced.width, height: sliced.height))
                continuation.resume(returning: out)
            }
            panel.contentView = view
            panel.orderFrontRegardless()
            panel.makeKey()
        }
    }
}

private final class RegionPickView: NSView {
    var image: NSImage?
    var onFinish: ((CGRect?) -> Void)?
    private var anchor: NSPoint?
    private var current: NSPoint?

    override var acceptsFirstResponder: Bool { true }

    override func viewDidMoveToWindow() {
        super.viewDidMoveToWindow()
        window?.makeFirstResponder(self)
    }

    override func resetCursorRects() {
        addCursorRect(bounds, cursor: .crosshair)
    }

    override func mouseDown(with event: NSEvent) {
        let p = convert(event.locationInWindow, from: nil)
        anchor = p
        current = p
        needsDisplay = true
    }

    override func mouseDragged(with event: NSEvent) {
        current = convert(event.locationInWindow, from: nil)
        needsDisplay = true
    }

    override func mouseUp(with event: NSEvent) {
        current = convert(event.locationInWindow, from: nil)
        onFinish?(selectionRect)
    }

    override func cancelOperation(_ sender: Any?) {
        onFinish?(nil)
    }

    override func keyDown(with event: NSEvent) {
        if event.keyCode == 53 {
            onFinish?(nil)
            return
        }
        super.keyDown(with: event)
    }

    private var selectionRect: CGRect? {
        guard let anchor, let current else { return nil }
        return CGRect(
            x: min(anchor.x, current.x),
            y: min(anchor.y, current.y),
            width: abs(current.x - anchor.x),
            height: abs(current.y - anchor.y)
        )
    }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        image?.draw(in: bounds, from: .zero, operation: .copy, fraction: 1)
        NSColor.black.withAlphaComponent(0.45).setFill()
        bounds.fill()
        if let r = selectionRect, r.width > 1, r.height > 1 {
            image?.draw(in: r, from: sourceRect(for: r), operation: .copy, fraction: 1)
            NSColor.white.withAlphaComponent(0.9).setStroke()
            let path = NSBezierPath(rect: r.insetBy(dx: 0.5, dy: 0.5))
            path.lineWidth = 1
            path.stroke()
        }
    }

    private func sourceRect(for viewRect: CGRect) -> NSRect {
        guard let image, image.size.width > 0, image.size.height > 0 else { return .zero }
        let sx = image.size.width / bounds.width
        let sy = image.size.height / bounds.height
        return NSRect(
            x: viewRect.minX * sx,
            y: viewRect.minY * sy,
            width: viewRect.width * sx,
            height: viewRect.height * sy
        )
    }
}
