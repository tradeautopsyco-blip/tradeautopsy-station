import Foundation
import Testing
@testable import Notch

@MainActor
struct BarAccountChromeTests {
    @Test func startSlugMapsToShippingBookOnly() {
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "kotak_neo") == "kotak-nse-bse-cash")
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "binance_com") == "binance-com-spot")
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "kotak-nse-nfo") == nil)
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "binance-com-options") == nil)
        #expect(BarAccountChrome.obtainAdapterId(forStartSlug: "kotak") == "kotak_neo")
        #expect(
            BarAccountChrome.obtainPath(
                adapter: "kotak_neo",
                bookId: "kotak-nse-bse-cash",
                operation: "funds"
            ) == "/api/station/obtain?adapter=kotak_neo&book=kotak-nse-bse-cash&operation=funds"
        )
    }

    @Test func kotakFundsPulseUsesObtainAssetNotDaemonPositions() {
        let snap = BarAccountChrome.compose(
            shippingBookId: "kotak-nse-bse-cash",
            quoteCurrency: "INR",
            funds: fundsEnvelope(book: "kotak-nse-bse-cash", asset: "INR", free: 19.41),
            holdings: listEnvelope(
                book: "kotak-nse-bse-cash",
                status: "success",
                countKey: "holding_count",
                rows: [
                    ["symbol": "IDBI", "quantity": 200],
                    ["symbol": "PNB", "quantity": 150],
                ]
            ),
            positions: listEnvelope(
                book: "kotak-nse-bse-cash",
                status: "success",
                countKey: "position_count",
                rows: [["symbol": "RELIANCE", "net_qty": 10]]
            ),
            orders: ["status": "unavailable", "book_id": "kotak-nse-bse-cash", "data": NSNull()]
        )
        #expect(snap.bookLabel == "Cash · INR")
        #expect(snap.freeText == "INR 19.41")
        #expect(snap.holdingsCount == 2)
        #expect(snap.holdingsRows.map(\.symbol) == ["IDBI", "PNB"])
        #expect(snap.positionsCount == 1)
        #expect(snap.positionsRows.first?.qty == "10")
        #expect(BarAccountChrome.ordersCountLabel(snap) == "orders unavailable")
    }

    @Test func foreignBookEnvelopeIsDroppedDualNoBlend() {
        let snap = BarAccountChrome.compose(
            shippingBookId: "kotak-nse-bse-cash",
            quoteCurrency: "INR",
            funds: fundsEnvelope(book: "binance-com-spot", asset: "USDT", free: 88),
            holdings: nil,
            positions: nil,
            orders: nil
        )
        #expect(snap.freeText == "—")
        #expect(snap.fundsStatus == "unavailable")
        #expect(snap.bookId == "kotak-nse-bse-cash")
    }

    @Test func viewModelPlantDoesNotTouchTodayFillInventory() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.deskQuoteCurrency = "INR"
        vm.positions = [
            NotchPosition(symbol: "FILL-INV", qty: 99, unrealizedPnL: 1, direction: "LONG"),
        ]
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "kotak-nse-bse-cash", asset: "INR", free: 19.41),
            holdings: listEnvelope(
                book: "kotak-nse-bse-cash",
                status: "success",
                countKey: "holding_count",
                rows: [["symbol": "IDBI", "quantity": 200]]
            ),
            positions: listEnvelope(
                book: "kotak-nse-bse-cash",
                status: "success",
                countKey: "position_count",
                rows: [["symbol": "RELIANCE", "net_qty": 10]]
            ),
            orders: ["status": "unsupported", "book_id": "kotak-nse-bse-cash"]
        )
        #expect(vm.accountChrome.freeText == "INR 19.41")
        #expect(vm.accountChrome.holdingsRows.first?.symbol == "IDBI")
        #expect(vm.accountChrome.positionsRows.first?.symbol == "RELIANCE")
        #expect(vm.positions.map(\.symbol) == ["FILL-INV"])
        #expect(vm.positions.first?.qty == 99)
        #expect(BarAccountChrome.ordersCountLabel(vm.accountChrome) == "orders unsupported")
    }

    @Test func stopClearsAccountChrome() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.deskQuoteCurrency = "INR"
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "kotak-nse-bse-cash", asset: "INR", free: 1),
            holdings: nil,
            positions: nil,
            orders: nil
        )
        #expect(vm.accountChrome.freeText != "—")
        vm.applyBrokerSyncStatePayload([
            "syncState": "not_connected",
            "brokerSlug": "kotak_neo",
        ])
        #expect(vm.accountChrome == .empty)
    }

    private func fundsEnvelope(book: String, asset: String, free: Double) -> [String: Any] {
        [
            "status": "success",
            "book_id": book,
            "data": [
                "identity": ["family": "account", "capability_id": "funds"],
                "holdings": [["asset": asset, "free": free, "locked": 0]],
            ],
        ]
    }

    private func listEnvelope(
        book: String,
        status: String,
        countKey: String,
        rows: [[String: Any]]
    ) -> [String: Any] {
        [
            "status": status,
            "book_id": book,
            "data": [
                countKey: rows.count,
                "rows": rows,
            ],
        ]
    }
}
