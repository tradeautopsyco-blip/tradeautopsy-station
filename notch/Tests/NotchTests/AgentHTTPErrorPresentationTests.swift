import Testing
@testable import Notch
import Foundation

struct AgentHTTPErrorPresentationTests {
    @Test func jsonErrorClassAndMessageBecomeStableUserCopyWithStatus() {
        let body = Data("""
        {"error_class":"SERVER_DOWN","message":"connection refused","request_id":"r-secret","retry_after_ms":null}
        """.utf8)
        let msg = AgentHTTPErrorPresentation.message(httpStatus: 502, body: body)
        #expect(msg.contains("502"))
        #expect(msg.localizedCaseInsensitiveContains("connection refused"))
        #expect(msg.localizedCaseInsensitiveContains("SERVER_DOWN") || msg.contains("SERVER_DOWN"))
        #expect(!msg.contains("r-secret"))
        #expect(!msg.contains("retry_after"))
    }

    @Test func bearerTokenAndApiSecretNeverAppearInUserCopy() {
        let jsonBody = Data("""
        {"error_class":"AUTH","message":"rejected","api_secret":"super-secret-value","Authorization":"Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.sig"}
        """.utf8)
        let fromJSON = AgentHTTPErrorPresentation.message(httpStatus: 401, body: jsonBody)
        #expect(!fromJSON.contains("Bearer"))
        #expect(!fromJSON.contains("eyJhbGciOi"))
        #expect(!fromJSON.contains("api_secret"))
        #expect(!fromJSON.contains("super-secret-value"))

        let rawBody = Data("upstream dump Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.x.y api_secret=leak\n".utf8)
        let fromRaw = AgentHTTPErrorPresentation.message(httpStatus: 502, body: rawBody)
        #expect(!fromRaw.contains("Bearer"))
        #expect(!fromRaw.contains("eyJhbGciOi"))
        #expect(!fromRaw.contains("api_secret"))
        #expect(!fromRaw.contains("leak"))
        #expect(fromRaw.contains("502"))
    }

    @Test func malformedBodyUsesStatusOnlyFallbackWithoutRawSnippet() {
        let body = Data("<html>upstream 502 stack Bearer eyJhbGciOi…</html>".utf8)
        let msg = AgentHTTPErrorPresentation.message(httpStatus: 502, body: body)
        #expect(msg == "Request failed (502) — try again" || msg.hasSuffix("failed (502) — try again"))
        #expect(msg.contains("502"))
        #expect(msg.localizedCaseInsensitiveContains("try again"))
        #expect(!msg.contains("<html>"))
        #expect(!msg.contains("Bearer"))
        #expect(!msg.contains("stack"))
    }

    @Test func allowListedErrorAndCodeFieldsBecomeUserCopy() {
        let errorStringBody = Data("""
        {"error":"loss limits not acknowledged","debug_dump":"should-not-leak"}
        """.utf8)
        let fromError = AgentHTTPErrorPresentation.message(httpStatus: 403, body: errorStringBody)
        #expect(fromError.contains("403"))
        #expect(fromError.localizedCaseInsensitiveContains("loss limits"))
        #expect(!fromError.contains("debug_dump"))
        #expect(!fromError.contains("should-not-leak"))

        let codeBody = Data("""
        {"code":"validation_error","trace":"raw-should-not-leak"}
        """.utf8)
        let fromCode = AgentHTTPErrorPresentation.message(httpStatus: 400, body: codeBody)
        #expect(fromCode.contains("400"))
        #expect(fromCode.contains("validation_error"))
        #expect(!fromCode.contains("trace"))
        #expect(!fromCode.contains("raw-should-not-leak"))
    }

    @Test func declarePresentationNeverPastesRawBodySnippet() {
        let body = Data("Declaration boom Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.x.y api_secret=leak\n".utf8)
        let msg = BarDeclareHTTPErrorPresentation.message(httpStatus: 502, body: body)
        #expect(msg.contains("502"))
        #expect(!msg.contains("Bearer"))
        #expect(!msg.contains("eyJhbGciOi"))
        #expect(!msg.contains("api_secret"))
        #expect(!msg.contains("leak"))
        #expect(!msg.contains("Declaration boom"))
    }

    @Test func declareActivationRequiredStillMapsThroughSharedSeam() {
        let body = Data("""
        {"error":{"code":"bar_activation_required","keys":["bar.loss_limits.not_acknowledged"]}}
        """.utf8)
        let msg = BarDeclareHTTPErrorPresentation.message(httpStatus: 403, body: body)
        #expect(msg.localizedCaseInsensitiveContains("loss limits"))
        #expect(msg.localizedCaseInsensitiveContains("acknowledge"))
    }
}
