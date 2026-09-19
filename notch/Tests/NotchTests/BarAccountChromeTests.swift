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
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "binance_com") != "binance-com-usdm")
        #expect(BarAccountChrome.shippingBookId(forStartSlug: "binance_com") == "binance-com-spot")
        #expect(BarAccountChrome.namedBookId(forAssetClass: .spot, startSlug: "binance_com") == nil)
        #expect(BarAccountChrome.namedBookId(forAssetClass: .options, startSlug: "binance_com") == nil)
        #expect(
            BarAccountChrome.namedBookId(forAssetClass: .usdm, startSlug: "binance_com")
                == "binance-com-usdm"
        )
        #expect(BarAccountChrome.namedBookId(forAssetClass: .usdm, startSlug: "kotak_neo") == nil)
        #expect(
            BarAccountChrome.pulseBookId(forAssetClass: .usdm, startSlug: "binance_com")
                == "binance-com-usdm"
        )
        #expect(
            BarAccountChrome.pulseBookId(forAssetClass: .spot, startSlug: "binance_com")
                == "binance-com-spot"
        )
        let usdmFunds = BarAccountChrome.obtainPath(
            adapter: "binance_com",
            bookId: BarAccountChrome.namedBookId(forAssetClass: .usdm, startSlug: "binance_com")
                ?? "",
            operation: "funds"
        )
        #expect(usdmFunds.contains("adapter=binance_com"))
        #expect(usdmFunds.contains("book=binance-com-usdm"))
        #expect(usdmFunds.contains("operation=funds"))
        let usdmPositions = BarAccountChrome.obtainPath(
            adapter: "binance_com",
            bookId: "binance-com-usdm",
            operation: "positionbook"
        )
        #expect(usdmPositions.contains("book=binance-com-usdm"))
        #expect(usdmPositions.contains("operation=positionbook"))
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

    @Test func spotComposeDropsForeignUsdmBookIdDualNoBlend() {
        let snap = BarAccountChrome.compose(
            shippingBookId: "binance-com-spot",
            quoteCurrency: "USDT",
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: [["symbol": "ETHUSDT", "net_qty": 2]]
            ),
            orders: nil
        )
        #expect(snap.bookId == "binance-com-spot")
        #expect(snap.freeText == "—")
        #expect(snap.fundsStatus == "unavailable")
        #expect(snap.positionsCount == 0)
        #expect(snap.positionsRows.isEmpty)
    }

    @Test func usdmEmptyPositionbookComposesToCountZero() {
        let snap = BarAccountChrome.compose(
            shippingBookId: "binance-com-usdm",
            quoteCurrency: "USDT",
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: []
            ),
            orders: nil
        )
        #expect(snap.bookLabel == "USDM · USDT")
        #expect(snap.freeText == "USDT 1.04")
        #expect(snap.positionsCount == 0)
        #expect(snap.positionsRows.isEmpty)
        #expect(snap.holdingsCount == 0)
        #expect(snap.holdingsRows.isEmpty)
        #expect(snap.fundsStatus == "success")
        #expect(snap.fundsStatus != "synced")
    }

    @Test func usdmPulsePlantUsesNamedBookNotStartSpot() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USDT"
        vm.declareAssetClass = .usdm
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: []
            ),
            orders: nil
        )
        #expect(vm.accountChrome.bookId == "binance-com-usdm")
        #expect(vm.accountChrome.freeText == "USDT 1.04")
        #expect(vm.accountChrome.positionsCount == 0)
        #expect(vm.accountChrome.fundsStatus == "success")
        #expect(vm.accountChrome.fundsStatus != "synced")
    }

    @Test func spotPulseStillDropsUsdmWhenClassIsSpot() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USDT"
        vm.declareAssetClass = .spot
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: nil,
            orders: nil
        )
        #expect(vm.accountChrome.bookId == "binance-com-spot")
        #expect(vm.accountChrome.freeText == "—")
        #expect(vm.accountChrome.fundsStatus == "unavailable")
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

    @Test func usdmSpotDaemonPositionsDoNotBecomeLivePlan() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USDT"
        vm.declareAssetClass = .usdm
        vm.positions = [
            NotchPosition(symbol: "BTCUSDT", qty: 1, unrealizedPnL: 0, direction: "LONG"),
        ]
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: []
            ),
            orders: nil
        )
        #expect(vm.barSurfacePhase != .livePlan)
        #expect(!vm.barDebriefPending)
        #expect(!vm.hasOpenPositions)
    }

    @Test func usdmPositionbookOpenIsLivePlanAndCloseArmsDebrief() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.deskQuoteCurrency = "USDT"
        vm.declareAssetClass = .usdm
        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: [["symbol": "ETHUSDT", "net_qty": 2]]
            ),
            orders: nil
        )
        #expect(vm.accountChrome.positionsCount == 1)
        #expect(vm.accountChrome.positionsRows.first?.qty == "2")
        #expect(vm.barSurfacePhase == .livePlan)
        #expect(!vm.barDebriefPending)
        #expect(vm.hasOpenPositions)

        vm.applyAccountObtainEnvelopes(
            funds: fundsEnvelope(book: "binance-com-usdm", asset: "USDT", free: 1.04),
            holdings: nil,
            positions: listEnvelope(
                book: "binance-com-usdm",
                status: "success",
                countKey: "position_count",
                rows: []
            ),
            orders: nil
        )
        #expect(vm.accountChrome.positionsCount == 0)
        #expect(vm.barDebriefPending)
        #expect(vm.barSurfacePhase == .debrief)
    }

    @Test func escrowDeclaredColumnUsesPendingWhenConsoleNodesEmpty() {
        let pending = BarPendingDeclaration(
            id: "00000000-0000-4000-8000-000000000099",
            status: "PENDING",
            createdAt: nil,
            symbol: "BTCUSDT",
            side: "BUY",
            quantity: 0.002,
            declarationKind: "intraday",
            protectiveSlConsent: false,
            stopLoss: 64000,
            target: 0.75,
            planSnapshot: nil
        )
        let rows = BarEscrowMatchPresentation.sevenSlotRows(from: nil, pending: pending)
        #expect(rows[1].declared == "BTCUSDT")
        #expect(rows[2].declared == "BUY")
        #expect(rows[5].declared != "—")
        #expect(rows[6].declared != "—")
        #expect(rows.allSatisfy { $0.actual == "—" })
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
