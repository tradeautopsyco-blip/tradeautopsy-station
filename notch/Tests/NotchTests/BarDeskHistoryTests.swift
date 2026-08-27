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

    private func glanceLine(_ vm: NotchViewModel) -> String {
        BarDeskTemplate.historyGlanceLine(
            licensedStatus: vm.deskHistoryStatus,
            licensedIneligible: vm.deskHistoryIneligible,
            yahooStatus: vm.deskYahooHistoryStatus,
            yahooIneligible: vm.deskYahooHistoryIneligible,
            stitchYahoo: false
        )
    }
}
