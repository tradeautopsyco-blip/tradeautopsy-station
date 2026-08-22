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
        let cg: CGImage
        do {
            cg = try await captureDisplayImage()
        } catch {
            if !hasScreenRecordingAccess() {
                await MainActor.run { openScreenRecordingSettings() }
                throw JournalInteractiveScreenshotError.permissionDenied
            }
            throw JournalInteractiveScreenshotError.failed
        }
        let still = NSImage(cgImage: cg, size: NSSize(width: cg.width, height: cg.height))
        let screen: NSScreen? = await MainActor.run {
            NSScreen.main ?? NSScreen.screens.first
        }
        guard let screen else { throw JournalInteractiveScreenshotError.failed }
        let cropped = try await JournalRegionPicker.crop(from: still, on: screen)
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

    private static func captureDisplayImage() async throws -> CGImage {
        let content = try await SCShareableContent.excludingDesktopWindows(false, onScreenWindowsOnly: true)
        guard !content.displays.isEmpty else { throw JournalInteractiveScreenshotError.failed }
        let displayID = await MainActor.run { () -> CGDirectDisplayID? in
            NSScreen.main?.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? CGDirectDisplayID
        }
        let display = content.displays.first(where: { $0.displayID == displayID }) ?? content.displays[0]
        let bundle = Bundle.main.bundleIdentifier
        let excluded = content.applications.filter { $0.bundleIdentifier == bundle }
        let filter = SCContentFilter(
            display: display,
            excludingApplications: excluded,
            exceptingWindows: []
        )
        let config = SCStreamConfiguration()
        config.width = display.width
        config.height = display.height
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
