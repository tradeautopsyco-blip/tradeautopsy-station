import Testing
@testable import Notch
import Foundation

struct BarLiveStateErrorPresentationTests {
    private func envelope(message: String) -> Data {
        let json = """
        {"error_class":"SERVER_DOWN","message":"\(message)","retry_after_ms":null,"request_id":"r1"}
        """
        return Data(json.utf8)
    }

    @Test func deviceLoginRequiredIsDetectedFromAgentMessage() {
        let body = envelope(message: "No Station Caller tokens in Keychain — complete device login")
        #expect(BarLiveStateErrorPresentation.requiresDeviceLogin(body: body) == true)
        #expect(BarLiveStateErrorPresentation.message(httpStatus: 502, body: body)
            .localizedCaseInsensitiveContains("device login") == true)
    }

    @Test func genericServerDownIsNotMisreportedAsDeviceLogin() {
        let body = envelope(message: "connection refused")
        #expect(BarLiveStateErrorPresentation.requiresDeviceLogin(body: body) == false)
        let msg = BarLiveStateErrorPresentation.message(httpStatus: 502, body: body)
        #expect(msg.contains("502"))
        #expect(msg.localizedCaseInsensitiveContains("device login") == false)
    }

    @Test func malformedBodyFallsBackToGenericMessageWithoutCrashing() {
        let body = Data("not json".utf8)
        #expect(BarLiveStateErrorPresentation.requiresDeviceLogin(body: body) == false)
        #expect(BarLiveStateErrorPresentation.message(httpStatus: 500, body: body).contains("500"))
    }
}
