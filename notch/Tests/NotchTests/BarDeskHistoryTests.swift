import Testing
@testable import Notch

@MainActor
struct BarDeskHistoryTests {
    @Test func applyStationHistoryEnvelopeSuccessSetsStatusNoYahooGlance() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "instrument_id": "btcusdt",
            "ineligible": [] as [String],
            "data": [
                "last_close": "0.01590000",
                "interval": "1m",
                "source": "binance_klines",
            ],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "success")
        #expect(vm.deskHistoryIneligible.isEmpty)
        let line = glanceLine(vm)
        #expect(line == "success")
        #expect(!line.contains("yahoo"))
        #expect(!line.contains("yahoo-shaped"))
        #expect(vm.deskYahooHistoryStatus == "unavailable")
    }

    @Test func applyStationHistoryEnvelopeSpotKlinesPaintsCandles() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "instrument_id": "btcusdt",
            "book_id": "binance-com-spot",
            "ineligible": [] as [String],
            "data": [
                "last_close": "0.01590000",
                "interval": "1m",
                "source": "binance_klines",
                "candles": [[
                    "open_time_ms": 1_499_040_000_000,
                    "open": "0.01577100",
                    "high": "0.01577100",
                    "low": "0.01577100",
                    "close": "0.01590000",
                    "volume": "148976.11",
                ]],
            ],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "success")
        #expect(vm.deskHistoryCandles.count == 1)
        #expect(vm.deskHistoryCandles[0].close == "0.01590000")
        #expect(glanceLine(vm) == "success")
        #expect(!glanceLine(vm).contains("yahoo"))
    }

    @Test func applyStationHistoryEnvelopeSpotEmptyCandlesIsUnavailable() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "instrument_id": "btcusdt",
            "data": [
                "source": "binance_klines",
                "candles": [] as [[String: Any]],
            ],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "unavailable")
        #expect(vm.deskHistoryCandles.isEmpty)
    }

    @Test func applyStationHistoryEnvelopeUnavailableIsNoLicensedSeries() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.applyStationHistoryEnvelope([
            "status": "unavailable",
            "instrument_id": "btcusdt",
            "ineligible": [] as [String],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "unavailable")
        let line = glanceLine(vm)
        #expect(line == "no licensed series")
        #expect(!line.contains("yahoo"))
    }

    @Test func applyStationHistoryEnvelopeIneligibleUnsupportedInterval() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.applyStationHistoryEnvelope([
            "status": "unavailable",
            "instrument_id": "btcusdt",
            "ineligible": ["unsupported_interval"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "unavailable")
        #expect(vm.deskHistoryIneligible.contains("unsupported_interval"))
        #expect(!vm.deskHistoryIneligible.contains("rights_forbid_canonical"))
        let line = glanceLine(vm)
        #expect(line == "no licensed series")
        #expect(!line.contains("yahoo"))
    }

    @Test func applyStationHistoryEnvelopeKotakStaysUnsupportedOnBinanceShapedEnvelope() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "instrument_id": "btcusdt",
            "ineligible": [] as [String],
            "data": [
                "last_close": "0.01590000",
                "interval": "1m",
                "source": "binance_klines",
            ],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskHistoryStatus == "unsupported")
        #expect(vm.deskHistoryIneligible.isEmpty)
        let line = glanceLine(vm)
        #expect(line == "unsupported")
        #expect(!line.contains("yahoo"))
        #expect(!line.contains("yahoo-shaped"))
    }

    @Test func applyStationHistoryEnvelopeKotakVendorSuccessPaintsLabsSeries() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "adapter_id": "licensed_history",
            "book_id": "licensed-history",
            "product_use": "labs",
            "provenance_adapter_id": "licensed_history",
            "ineligible": [] as [String],
            "data": [
                "last_close": "1401.00",
                "interval": "1m",
                "source": "licensed_history",
                "candles": [
                    [
                        "open_time_ms": 1_700_000_000_000,
                        "open": "1400.00",
                        "high": "1402.00",
                        "low": "1398.00",
                        "close": "1401.00",
                        "volume": "10",
                    ],
                ],
            ],
        ])
        #expect(vm.deskHistoryStatus == "success")
        #expect(vm.deskHistoryCandles.count == 1)
        #expect(vm.deskHistoryCandles[0].close == "1401.00")
        let line = glanceLine(vm)
        #expect(line == "success · labs · licensed-history")
        #expect(!line.contains("licensed_history"))
        #expect(!line.contains("yahoo"))
        #expect(!line.contains("binance"))
    }

    @Test func applyStationHistoryEnvelopeKotakNativePaintsUtcMsAndNotLicensedHistory() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "adapter_id": "kotak_neo",
            "book_id": "kotak-nse-bse-cash",
            "provenance_adapter_id": "kotak_neo",
            "ineligible": [] as [String],
            "data": [
                "source": "kotak_neo_historical",
                "interval": "15min",
                "candles": [[
                    "open_time": "2026-08-20T09:15:00+0530",
                    "open_time_ms": 1_787_197_500_000,
                    "open": "1400.00",
                    "high": "1402.00",
                    "low": "1398.00",
                    "close": "1401.00",
                    "volume": "10",
                ]],
            ],
        ])
        #expect(vm.deskHistoryStatus == "success")
        #expect(vm.deskHistoryCandles.count == 1)
        #expect(vm.deskHistoryCandles[0].openTimeMs == 1_787_197_500_000)
        #expect(vm.deskHistoryCandles[0].close == "1401.00")
        let line = glanceLine(vm)
        #expect(line == "success")
        #expect(!line.contains("licensed_history"))
        #expect(!line.contains("yahoo"))
        #expect(!line.contains("binance"))
    }

    @Test func applyStationHistoryEnvelopeKotakNativeEmptySuccessIsUnavailable() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([
            "status": "success",
            "adapter_id": "kotak_neo",
            "book_id": "kotak-nse-bse-cash",
            "provenance_adapter_id": "kotak_neo",
            "data": [
                "source": "kotak_neo_historical",
                "candles": [] as [[String: Any]],
            ],
        ])
        #expect(vm.deskHistoryStatus == "unavailable")
        #expect(vm.deskHistoryCandles.isEmpty)
        #expect(glanceLine(vm) == "no licensed series")
    }

    @Test func applyStationHistoryEnvelopeKotakVendorUnavailableIsNoLicensedSeries() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([
            "status": "unavailable",
            "adapter_id": "licensed_history",
            "book_id": "licensed-history",
            "product_use": "labs",
            "provenance_adapter_id": "licensed_history",
        ])
        #expect(vm.deskHistoryStatus == "unavailable")
        #expect(vm.deskHistoryCandles.isEmpty)
        #expect(glanceLine(vm) == "no licensed series")
    }

    @Test func applyStationHistoryEnvelopeKotakEmptyStaysUnsupported() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationHistoryEnvelope([:])
        #expect(vm.deskHistoryStatus == "unsupported")
        #expect(vm.deskHistoryCandles.isEmpty)
        #expect(glanceLine(vm) == "unsupported")
    }

    @Test func applyStationHistoryEnvelopeIgnoresYahooSource() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.deskHistoryStatus = "success"
        vm.applyStationHistoryEnvelope([
            "status": "research_segment",
            "instrument_id": "btcusdt",
            "ineligible": ["rights_forbid_canonical"],
            "data": ["source": "yahoo"],
            "provenance": ["adapter_id": "yahoo_chart"],
        ])
        #expect(vm.deskHistoryStatus == "success")
        #expect(!vm.deskHistoryIneligible.contains("rights_forbid_canonical"))
        #expect(vm.deskYahooHistoryStatus == "unavailable")
        let line = glanceLine(vm)
        #expect(line == "success")
        #expect(!line.contains("yahoo"))
        #expect(!line.contains("yahoo-shaped"))
    }

    @Test func kotakCashBindAsksNativeHistoryWithInstrument() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .equity
        let path = vm.deskHistoryExtractPath(instrument: "nse_cm|2885", consumeVendorArm: true)
        #expect(path == VendorHistoryObtain.kotakNativeHistoryPath(instrument: "nse_cm|2885"))
        #expect(path?.contains("licensed_history") == false)
    }

    @Test func kotakNfoHistoryStaysVendorGatedHoleWhenOff() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .options
        #expect(vm.deskHistoryExtractPath(instrument: "nse_fo|61466", consumeVendorArm: true) == nil)
    }

    @Test func usdmHistoryPathStaysNil() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.declareAssetClass = .usdm
        #expect(vm.deskHistoryExtractPath(instrument: "BTCUSDT", consumeVendorArm: true) == nil)
    }

    private func glanceLine(_ vm: NotchViewModel) -> String {
        BarDeskTemplate.historyGlanceLine(
            licensedStatus: vm.deskHistoryStatus,
            licensedIneligible: vm.deskHistoryIneligible,
            yahooStatus: vm.deskYahooHistoryStatus,
            yahooIneligible: vm.deskYahooHistoryIneligible,
            stitchYahoo: false,
            productUse: vm.deskHistoryProductUse,
            bookId: vm.deskHistoryBookId
        )
    }
}
