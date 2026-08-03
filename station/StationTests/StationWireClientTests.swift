import CryptoKit
import Foundation
import Testing
@testable import Station

struct StationWireClientTests {
    /// Fixed inputs → signature must match the documented Wire v1 canonical HMAC
    /// (`METHOD\npath\ntimestamp\nrequest_id\nhex_sha256(body)` → base64 HMAC-SHA256).
    @Test func signatureMatchesDocumentedCanonicalHMAC() {
        let method = "POST"
        let path = "/api/daemon/brokers/start"
        let timestamp = "2024-06-15T12:34:56.789Z"
        let requestID = "01ARZ3NDEKTSV4RRFFQ69G5FAV"
        let body = Data(#"{"broker_slug":"binance_com"}"#.utf8)
        let secret = "phase3-wire-signing-fixture-secret"

        let bodyHash = SHA256.hash(data: body).map { String(format: "%02x", $0) }.joined()
        let canonical = "\(method)\n\(path)\n\(timestamp)\n\(requestID)\n\(bodyHash)"
        let key = SymmetricKey(data: Data(secret.utf8))
        let mac = HMAC<SHA256>.authenticationCode(for: Data(canonical.utf8), using: key)
        let expected = Data(mac).base64EncodedString()

        let actual = StationWireClient.signature(
            method: method,
            path: path,
            timestamp: timestamp,
            requestID: requestID,
            body: body,
            secret: secret
        )
        #expect(actual == expected)
    }

    @Test func emptyBodySignatureMatchesCanonicalHMAC() {
        let method = "GET"
        let path = "/api/daemon/health"
        let timestamp = "2025-01-01T00:00:00.000Z"
        let requestID = "01JF0TESTULID000000000000"
        let body = Data()
        let secret = "identical-to-notch-secret"

        let bodyHash = SHA256.hash(data: body).map { String(format: "%02x", $0) }.joined()
        let canonical = "\(method)\n\(path)\n\(timestamp)\n\(requestID)\n\(bodyHash)"
        let key = SymmetricKey(data: Data(secret.utf8))
        let mac = HMAC<SHA256>.authenticationCode(for: Data(canonical.utf8), using: key)
        let expected = Data(mac).base64EncodedString()

        let actual = StationWireClient.signature(
            method: method,
            path: path,
            timestamp: timestamp,
            requestID: requestID,
            body: body,
            secret: secret
        )
        #expect(actual == expected)
    }

    @Test func loopbackWireUserIdIsFixedHintUUID() {
        #expect(StationWireClient.loopbackWireUserId == "00000000-0000-4000-8000-000000000002")
        #expect(StationWireClient.protoVersion == "1")
    }
}
