import Foundation
import Testing
@testable import Notch

struct JournalInteractiveScreenshotTests {
    @Test func permissionDeniedCopyNamesSystemSettings() {
        #expect(
            JournalInteractiveScreenshotError.permissionDenied.userMessage
                .contains("Screen Recording")
        )
    }

    @Test func interpretZeroWithFileIsSuccess() {
        #expect(
            JournalInteractiveScreenshot.interpretCaptureResult(exitStatus: 0, fileExists: true)
                == nil
        )
    }

    @Test func interpretNonzeroWithoutFileIsCancelled() {
        #expect(
            JournalInteractiveScreenshot.interpretCaptureResult(exitStatus: 1, fileExists: false)
                == .cancelled
        )
    }

    @Test func interpretZeroByteFileIsFailed() {
        #expect(
            JournalInteractiveScreenshot.interpretCaptureResult(
                exitStatus: 0,
                fileExists: true,
                fileSize: 0
            ) == .failed
        )
    }

    @Test func interpretDeniedStderrIsPermission() {
        #expect(
            JournalInteractiveScreenshot.interpretCaptureResult(
                exitStatus: 1,
                fileExists: false,
                stderr: "could not capture: not authorized"
            ) == .permissionDenied
        )
    }

    @Test func ensureAccessThrowsWhenTccDenied() {
        let prevHas = JournalInteractiveScreenshot.hasScreenRecordingAccess
        let prevReq = JournalInteractiveScreenshot.requestScreenRecordingAccess
        defer {
            JournalInteractiveScreenshot.hasScreenRecordingAccess = prevHas
            JournalInteractiveScreenshot.requestScreenRecordingAccess = prevReq
        }
        JournalInteractiveScreenshot.hasScreenRecordingAccess = { false }
        JournalInteractiveScreenshot.requestScreenRecordingAccess = { false }
        do {
            try JournalInteractiveScreenshot.ensureScreenRecordingAccess()
            Issue.record("expected permissionDenied")
        } catch let err as JournalInteractiveScreenshotError {
            #expect(err == .permissionDenied)
        } catch {
            Issue.record("wrong error \(error)")
        }
    }
}
