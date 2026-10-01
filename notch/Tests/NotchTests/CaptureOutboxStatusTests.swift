import Foundation
import Testing
@testable import Notch

@Suite struct CaptureOutboxStatusTests {
    @Test func decodesStatusSnapshotWithDeadLetters() throws {
        let json = """
        {
          "success": true,
          "data": {
            "counts": { "enqueued": 1, "inflight": 0, "acked": 2, "dead_letter": 1 },
            "dead_letters": [{
              "outboxId": 9,
              "requestId": "req-1",
              "attempts": 1,
              "reason": "validation",
              "idempotencyKey": "idem-1",
              "draftText": "test",
              "updatedAtMs": 1
            }],
            "active_deliveries": [{
              "state": "ACKED",
              "idempotencyKey": "idem-2",
              "lastError": null,
              "pendingCaptureId": "11111111-2222-4333-8444-555555555555"
            }]
          }
        }
        """.data(using: .utf8)!
        let snap = CaptureOutboxStatusClient.decode(from: json)
        #expect(snap != nil)
        #expect(snap?.counts.deadLetter == 1)
        #expect(snap?.deadLetters.first?.reason == "validation")
        #expect(snap?.activeDeliveries.first?.pendingCaptureId?.hasPrefix("11111111") == true)
    }
}
