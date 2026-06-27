import AppKit
import Foundation

enum JournalInteractiveScreenshotError: Error {
    case userCancelledOrCaptureFailed
}

/// Native interactive capture via `/usr/sbin/screencapture -i` (design doc 6.2 / 6.5).
/// Temp files live under the user temp dir (APFS-encrypted volume on Apple Silicon / FileVault Macs).
/// Returns a temporary PNG file URL; caller **must** delete the file after upload (`removeTempFile`).
enum JournalInteractiveScreenshot {
    static func captureRegionToTempPNG() async throws -> URL {
        try await Task.detached(priority: .userInitiated) {
            let tmp = FileManager.default.temporaryDirectory
                .appendingPathComponent("ta-toolbar-shot-\(UUID().uuidString).png")
            let proc = Process()
            proc.executableURL = URL(fileURLWithPath: "/usr/sbin/screencapture")
            proc.arguments = ["-i", "-t", "png", tmp.path]
            do {
                try proc.run()
            } catch {
                try? FileManager.default.removeItem(at: tmp)
                throw error
            }
            proc.waitUntilExit()
            let ok = proc.terminationStatus == 0
                && FileManager.default.fileExists(atPath: tmp.path)
            if !ok {
                if FileManager.default.fileExists(atPath: tmp.path) {
                    try? FileManager.default.removeItem(at: tmp)
                }
                throw JournalInteractiveScreenshotError.userCancelledOrCaptureFailed
            }
            return tmp
        }.value
    }

    static func removeTempFile(at url: URL) {
        try? FileManager.default.removeItem(at: url)
    }
}
