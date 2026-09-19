import AppKit
import CoreGraphics
import Foundation
import ImageIO
import ScreenCaptureKit
import UniformTypeIdentifiers

enum JournalInteractiveScreenshotError: Error, Equatable {
    case permissionDenied
    case cancelled
    case failed
}

extension JournalInteractiveScreenshotError {
    var userMessage: String {
        switch self {
        case .permissionDenied:
            return "Allow Screen Recording for TradeAutopsy Station in System Settings → Privacy & Security, then reopen Station."
        case .cancelled:
            return "Screenshot cancelled."
        case .failed:
            return "Screenshot failed."
        }
    }
}

/// Region grab: ScreenCaptureKit still of the display (excludes Station), then drag-to-crop.
enum JournalInteractiveScreenshot {
    static var hasScreenRecordingAccess: () -> Bool = { CGPreflightScreenCaptureAccess() }
    static var requestScreenRecordingAccess: () -> Bool = { CGRequestScreenCaptureAccess() }
    static var openScreenRecordingSettings: () -> Void = {
        let urls = [
            "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture",
            "x-apple.systempreferences:com.apple.settings.PrivacySecurity.extension?Privacy_ScreenCapture",
        ]
        for s in urls {
            if let u = URL(string: s) {
                NSWorkspace.shared.open(u)
                return
            }
        }
    }

    static func captureRegionToTempPNG() async throws -> URL {
        await MainActor.run {
            promptForScreenRecordingAccessIfNeeded()
        }
        let target = await MainActor.run { currentCaptureTarget() }
        guard let target else { throw JournalInteractiveScreenshotError.failed }
        let cg: CGImage
        do {
            cg = try await captureDisplayImage(target: target)
        } catch {
            if !hasScreenRecordingAccess() {
                await MainActor.run { openScreenRecordingSettings() }
                throw JournalInteractiveScreenshotError.permissionDenied
            }
            throw JournalInteractiveScreenshotError.failed
        }
        let still = NSImage(
            cgImage: cg,
            size: JournalCaptureGeometry.pointSize(
                pixelWidth: cg.width,
                pixelHeight: cg.height,
                scale: target.scale
            )
        )
        let cropped = try await JournalRegionPicker.crop(from: still, screenFrame: target.frame)
        return try writeTempPNG(cropped)
    }

    static func promptForScreenRecordingAccessIfNeeded() {
        if hasScreenRecordingAccess() { return }
        _ = requestScreenRecordingAccess()
    }

    static func ensureScreenRecordingAccess() throws {
        promptForScreenRecordingAccessIfNeeded()
        if hasScreenRecordingAccess() { return }
        throw JournalInteractiveScreenshotError.permissionDenied
    }

    static func interpretCaptureResult(
        exitStatus: Int32,
        fileExists: Bool,
        fileSize: Int = 1024,
        stderr: String = ""
    ) -> JournalInteractiveScreenshotError? {
        if exitStatus == 0, fileExists, fileSize > 32 { return nil }
        if fileExists, fileSize > 32 { return nil }
        let lower = stderr.lowercased()
        if lower.contains("not authorized")
            || lower.contains("not permitted")
            || lower.contains("tcc")
            || lower.contains("screen capture")
        {
            return .permissionDenied
        }
        if exitStatus != 0 { return .cancelled }
        return .failed
    }

    static func removeTempFile(at url: URL) {
        try? FileManager.default.removeItem(at: url)
    }

    private struct CaptureTarget {
        let frame: NSRect
        let displayID: CGDirectDisplayID
        let scale: CGFloat
    }

    @MainActor
    private static func currentCaptureTarget() -> CaptureTarget? {
        guard let screen = NSScreen.main ?? NSScreen.screens.first else { return nil }
        guard let displayID = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")]
            as? CGDirectDisplayID
        else { return nil }
        return CaptureTarget(
            frame: screen.frame,
            displayID: displayID,
            scale: max(screen.backingScaleFactor, 1)
        )
    }

    private static func captureDisplayImage(target: CaptureTarget) async throws -> CGImage {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard !content.displays.isEmpty else { throw JournalInteractiveScreenshotError.failed }
        let display = content.displays.first(where: { $0.displayID == target.displayID }) ?? content.displays[0]
        let bundle = Bundle.main.bundleIdentifier
        let excluded = content.applications.filter { $0.bundleIdentifier == bundle }
        let filter = SCContentFilter(
            display: display,
            excludingApplications: excluded,
            exceptingWindows: []
        )
        let config = SCStreamConfiguration()
        config.width = max(1, Int((target.frame.width * target.scale).rounded()))
        config.height = max(1, Int((target.frame.height * target.scale).rounded()))
        config.showsCursor = false
        return try await SCScreenshotManager.captureImage(contentFilter: filter, configuration: config)
    }

    private static func writeTempPNG(_ image: NSImage) throws -> URL {
        guard let cg = image.cgImage(forProposedRect: nil, context: nil, hints: nil) else {
            throw JournalInteractiveScreenshotError.failed
        }
        let tmp = FileManager.default.temporaryDirectory
            .appendingPathComponent("ta-toolbar-shot-\(UUID().uuidString).png")
        let dest = CGImageDestinationCreateWithURL(tmp as CFURL, UTType.png.identifier as CFString, 1, nil)
        guard let dest else { throw JournalInteractiveScreenshotError.failed }
        CGImageDestinationAddImage(dest, cg, nil)
        guard CGImageDestinationFinalize(dest) else {
            try? FileManager.default.removeItem(at: tmp)
            throw JournalInteractiveScreenshotError.failed
        }
        return tmp
    }
}

enum CapturePasteboard {
    static func containsImage(_ pb: NSPasteboard) -> Bool {
        let types = Set(pb.types ?? [])
        if types.contains(.png) || types.contains(.tiff) { return true }
        if types.contains(NSPasteboard.PasteboardType("public.jpeg")) { return true }
        if types.contains(.fileURL),
           let url = pb.readObjects(forClasses: [NSURL.self], options: nil)?.first as? URL
        {
            let ext = url.pathExtension.lowercased()
            return ext == "png" || ext == "jpg" || ext == "jpeg"
        }
        return false
    }
}
