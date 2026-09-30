import Foundation
import Testing
@testable import Notch

struct ManualFillJournalDraftTests {
    @Test func draftTextLabelsManualSource() {
        let text = ManualFillJournalDraft.buildDraftText(
            symbol: "PNB",
            sideBuy: true,
            quantity: 100,
            price: 95.5,
            filledAtMs: 1_757_000_000_000,
            tradeId: "11111111-2222-4333-8444-555555555555",
            declarationId: "0491f631-78d1-496b-b191-4ade1dacb0df"
        )
        #expect(text.hasPrefix("manual fill · PNB BUY 100 @ 95.50"))
        #expect(text.contains("journalFillSource"))
        #expect(text.contains("stationFillId"))
        #expect(!text.contains("\"tradeId\""))
        #expect(text.contains("0491f631-78d1-496b-b191-4ade1dacb0df"))
    }

    @Test func requestBodyPendingOmitsConsoleTrade() {
        let decl = "0491f631-78d1-496b-b191-4ade1dacb0df"
        let body = ManualFillJournalDraft.requestBody(
            symbol: "pnb",
            sideBuy: false,
            quantity: 1,
            price: 10,
            filledAtMs: 1,
            declarationId: decl,
            idempotencyKey: "k1",
            consoleTradeId: nil
        )
        #expect(body["symbol"] as? String == "PNB")
        #expect(body["side"] as? String == "SELL")
        #expect(body["preTradeDeclarationId"] as? String == decl)
        #expect(body["consoleTradeId"] == nil)
    }

    @Test func requestBodyLinksConsoleTradeUUID() {
        let console = "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee"
        let body = ManualFillJournalDraft.requestBody(
            symbol: "PNB",
            sideBuy: true,
            quantity: 100,
            price: 95.5,
            filledAtMs: 1,
            declarationId: nil,
            idempotencyKey: "k2",
            consoleTradeId: console
        )
        #expect(body["consoleTradeId"] as? String == console)
        #expect(body["preTradeDeclarationId"] == nil)
    }

    @Test func presentAcceptDistinguishesEnqueueAcceptAndErrorCode() {
        let queued = ManualFillJournalDraft.presentAccept(
            httpStatus: 202,
            body: [
                "success": true,
                "data": ["status": "queued", "delivery": "local_enqueue"] as [String: Any],
            ]
        )
        #expect(queued.success == "Queued locally · Console not accepted yet")
        #expect(queued.error == nil)

        let accepted = ManualFillJournalDraft.presentAccept(
            httpStatus: 200,
            body: [
                "success": true,
                "data": [
                    "status": "accepted",
                    "delivery": "console_accept",
                    "pending_capture_id": "11111111-2222-4333-8444-555555555555",
                    "consoleHttpStatus": 200,
                ] as [String: Any],
            ]
        )
        #expect(accepted.success == "Console accept 200 · accepted · 11111111…")
        #expect(accepted.error == nil)

        let rejected = ManualFillJournalDraft.presentAccept(
            httpStatus: 400,
            body: [
                "success": false,
                "error": ["code": "VALIDATION_ERROR"] as [String: Any],
                "consoleHttpStatus": 400,
            ]
        )
        #expect(rejected.success == nil)
        #expect(rejected.error == "Console 400 · VALIDATION_ERROR")
    }

    @Test func consoleTradesKeepTodaysConsoleUUID() {
        let day = Date()
        let stamp = ISO8601DateFormatter()
        stamp.formatOptions = [.withInternetDateTime]
        let today = stamp.string(from: day)
        let json: [String: Any] = [
            "success": true,
            "data": [
                "trades": [
                    [
                        "trade_id": "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee",
                        "symbol": "PNB",
                        "side": "BUY",
                        "timestamp": today,
                    ],
                    [
                        "trade_id": "cccccccc-cccc-4ccc-8ddd-eeeeeeeeeeee",
                        "symbol": "SBIN",
                        "side": "SELL",
                        "timestamp": "2020-01-01T00:00:00Z",
                    ],
                ],
            ],
        ]
        let rows = ManualFillJournalDraft.consoleTrades(from: json, on: day)
        #expect(rows.count == 1)
        #expect(rows.first?.id == "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee")
        #expect(rows.first?.symbol == "PNB")
    }
}
