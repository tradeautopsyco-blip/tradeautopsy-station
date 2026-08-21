import Foundation
import Testing
@testable import Notch

struct JournalImageEncoderTests {
    /// 1×1 PNG.
    private var onePxPng: Data {
        Data(
            base64Encoded: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg=="
        )!
    }

    @Test func encodesPngHintToUnderTwoMegabytes() throws {
        let out = try JournalImageEncoder.encode(onePxPng, hintedType: "image/png")
        #expect(out.contentType == "image/png" || out.contentType == "image/jpeg")
        #expect(out.data.count <= JournalImageEncoder.maxBytes)
        #expect(!out.data.isEmpty)
    }

    @Test func rejectsPdf() {
        let pdf = Data("%PDF-1.4".utf8)
        do {
            _ = try JournalImageEncoder.encode(pdf, hintedType: "application/pdf")
            Issue.record("PDF should not encode")
        } catch {
            #expect(error is JournalImageEncoderError)
        }
    }
}
