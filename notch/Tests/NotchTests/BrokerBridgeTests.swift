import Testing
@testable import Notch

@MainActor
struct BrokerBridgeTests {
    @Test func brokerDisplayNameTable() {
        #expect(NotchViewModel.brokerDisplayName(forSlug: "kotak_neo") == "Kotak Neo")
        #expect(NotchViewModel.brokerDisplayName(forSlug: "binance_com") == "Binance.com")
        #expect(NotchViewModel.brokerDisplayName(forSlug: nil) == "No broker")
        #expect(NotchViewModel.brokerDisplayName(forSlug: "") == "No broker")
    }

    @Test func switchingExecutionDeskSnapsUnsupportedDeclareClass() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeExecutionBrokerSlug = "binance_com"
        vm.declareAssetClass = .usdm
        #expect(vm.planDeclareAssetClassTabs == [.spot, .options, .usdm, .coinm])
        vm.activeExecutionBrokerSlug = "kotak_neo"
        #expect(vm.declareAssetClass == .equity)
        #expect(vm.planDeclareAssetClassTabs == [.equity, .options])
        vm.declareAssetClass = .options
        vm.activeExecutionBrokerSlug = "binance_com"
        #expect(vm.declareAssetClass == .options)
        // Desk must actually change for didSet reconcile (re-assigning binance is a no-op).
        vm.activeExecutionBrokerSlug = "kotak_neo"
        vm.declareAssetClass = .equity
        vm.activeExecutionBrokerSlug = "binance_com"
        #expect(vm.declareAssetClass == .spot)
    }

    @Test func brokerPillChromeMapping() {
        let live = NotchViewModel.brokerPillChrome(brokerSyncClass: "synced", slug: "kotak_neo")
        #expect(live.dotName == "teal")
        #expect(live.label == "Kotak Neo · live")

        let connecting = NotchViewModel.brokerPillChrome(brokerSyncClass: "syncing", slug: "binance_com")
        #expect(connecting.dotName == "amber")
        #expect(connecting.label == "Binance.com · connecting")

        let degraded = NotchViewModel.brokerPillChrome(brokerSyncClass: "stale", slug: "kotak_neo")
        #expect(degraded.dotName == "amber")
        #expect(degraded.label == "Kotak Neo · degraded")

        let offline = NotchViewModel.brokerPillChrome(brokerSyncClass: "not_connected", slug: "kotak_neo")
        #expect(offline.dotName == "red")
        #expect(offline.label == "No broker · offline")
    }

    @Test func requestOpenBrokerConnectCollapsesAndInvokesCallbackWithResolvedSlug() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var received: String?
        vm.onRequestOpenBrokerConnect = { received = $0 }
        vm.barProtectiveBrokerSlug = ""
        vm.expandFromCollapsedChromeTap()
        #expect(vm.isExpanded == true)

        vm.requestOpenBrokerConnect()
        #expect(vm.isExpanded == false)
        #expect(received == "kotak_neo")
    }

    @Test func requestOpenBrokerReauthUsesActiveSlug() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var received: String?
        vm.onRequestOpenBrokerReauth = { received = $0 }
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSyncClass = "synced"

        vm.requestOpenBrokerReauth()
        #expect(received == "binance_com")
    }

    @Test func resolvedBrokerSlugFallsBackToKotakNeoWhenEmpty() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var received: String?
        vm.onRequestOpenBrokerConnect = { received = $0 }
        vm.activeBrokerSlug = nil
        vm.barProtectiveBrokerSlug = "   "
        vm.requestOpenBrokerConnect()
        #expect(received == "kotak_neo")
    }

    @Test func applyBrokerSyncStatePayloadSetsSyncedClassSlugAndDeskCurrency() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "not_connected"
        vm.activeBrokerSlug = nil
        vm.deskQuoteCurrency = nil

        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "quoteCurrency": "INR",
            "calcProfileId": "in_eq",
            "lastPollAtMs": 1_700_000_000_000,
        ])

        #expect(vm.brokerSyncClass == "synced")
        #expect(vm.brokerSessionActive == true)
        #expect(vm.activeBrokerSlug == "kotak_neo")
        #expect(vm.deskQuoteCurrency == "INR")
        #expect(vm.deskCalcProfileId == "in_eq")
        #expect(vm.brokerSyncLastPollAtMs == 1_700_000_000_000)

        let pill = NotchViewModel.brokerPillChrome(
            brokerSyncClass: vm.brokerSyncClass,
            slug: vm.activeBrokerSlug
        )
        #expect(pill.label == "Kotak Neo · live")
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: vm.brokerSyncClass) == false)
        #expect(NotchViewModel.brokerSettingsConnectionBadge(brokerSyncClass: vm.brokerSyncClass) == "Connected")
    }

    @Test func applyBrokerSyncStatePayloadComSyncedHidesOpenStationCta() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "binance_com",
            "quoteCurrency": "USDT",
        ])
        #expect(vm.activeBrokerSlug == "binance_com")
        #expect(vm.deskQuoteCurrency == "USDT")
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: "synced") == false)
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: "syncing") == false)
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: "stale") == false)
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: "not_connected") == true)
        #expect(NotchViewModel.brokerSettingsConnectionBadge(brokerSyncClass: "syncing") == "Connecting")
        #expect(NotchViewModel.brokerSettingsConnectionBadge(brokerSyncClass: "stale") == "Degraded")
        #expect(NotchViewModel.brokerSettingsConnectionBadge(brokerSyncClass: "not_connected") == "Not connected")
    }

    @Test func applyBrokerSyncStatePayloadOfflineClearsSlugKeepsLastKnownCurrency() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "quoteCurrency": "INR",
            "lastPollAtMs": 42,
        ])
        vm.applyBrokerSyncStatePayload([
            "syncState": "not_connected",
            "brokerSlug": "kotak_neo",
        ])
        #expect(vm.brokerSyncClass == "not_connected")
        #expect(vm.brokerSessionActive == false)
        #expect(vm.activeBrokerSlug == nil)
        #expect(vm.deskQuoteCurrency == "INR")
        #expect(NotchViewModel.showsOpenStationBrokersCta(brokerSyncClass: vm.brokerSyncClass) == true)
        let pill = NotchViewModel.brokerPillChrome(
            brokerSyncClass: vm.brokerSyncClass,
            slug: vm.activeBrokerSlug
        )
        #expect(pill.label == "No broker · offline")
    }

    @Test func applyBrokerSyncStatePayloadQuoteIndependentOfFundsFills() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "quoteCurrency": "INR",
            "capabilities": [
                "quote": "fresh",
                "funds": "unavailable",
                "fills": "stale",
                "instruments": "fresh",
            ],
        ])
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.deskFundsCapability == "unavailable")
        #expect(vm.deskFillsCapability == "stale")
        #expect(vm.deskQuoteCapability != vm.deskFundsCapability)
        let account = DeskCapabilityChrome.accountStatus(
            funds: vm.deskFundsCapability,
            fills: vm.deskFillsCapability
        )
        #expect(account == "stale")
        #expect(DeskCapabilityChrome.dotName(forStatus: vm.deskQuoteCapability) == "teal")
        #expect(DeskCapabilityChrome.dotName(forStatus: account) != "teal")
        #expect(vm.deskInstrumentsCapability == "fresh")
        #expect(vm.deskQuoteCurrency == "INR")
    }

    @Test func applyBrokerSyncStatePayloadLoadsInstrumentsCapability() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        #expect(vm.deskInstrumentsCapability == "unavailable")
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "binance_com",
            "capabilities": [
                "quote": "fresh",
                "funds": "fresh",
                "fills": "fresh",
                "instruments": "loading",
            ],
        ])
        #expect(vm.deskInstrumentsCapability == "loading")
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(DeskCapabilityChrome.dotName(forStatus: vm.deskInstrumentsCapability) == "amber")
        #expect(DeskCapabilityChrome.dotName(forStatus: vm.deskQuoteCapability) == "teal")
    }

    @Test func applyStationQuoteEnvelopeBindsKotakLastNotBinancePair() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "nse_cm|2885",
            "data": ["last": "2910.50"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.declEntryPrice == "2910.50")
    }

    @Test func applyStationQuoteEnvelopeRejectsBinanceAdapterOnKotakDesk() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.declEntryPrice = ""
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "btcusdt",
            "data": ["last": "65000"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
    }

    @Test func applyStationQuoteEnvelopeUnavailableQuotesHttpSetsLastStatus() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationQuoteEnvelope([
            "status": "unavailable",
            "ineligible": ["quotes_http"],
            "instrument_id": "nse_cm|2885",
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "quotes_http")
        #expect(vm.deskQuoteCapability == "quotes_http")
    }

    @Test func applyStationQuoteEnvelopeUnavailableQuotesUnusableSetsLastStatus() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.applyStationQuoteEnvelope([
            "status": "unavailable",
            "ineligible": ["quotes_unusable"],
            "instrument_id": "nse_cm|2885",
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "quotes_unusable")
        #expect(vm.deskQuoteCapability == "quotes_unusable")
    }

    @Test func selectSymbolKotakCashSetsTickBookId() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.showSymbolSuggestions = true
        let cash = InstrumentResult(
            trading_symbol: "NMDC",
            name: "NMDC Limited",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 11532,
            last_price: 0
        )
        vm.symbolSuggestions = [cash]
        vm.selectSymbol(cash)
        #expect(vm.barDeclarationLastError == nil)
        #expect(vm.deskSelectedInstrumentId == "nse_cm|11532")
        #expect(vm.barDeclarationSymbol == "NMDC")
        #expect(vm.symbolSuggestions.isEmpty)
        #expect(vm.showSymbolSuggestions == false)
    }

    @Test func selectSymbolRefusesNseOnKotakDesk() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.deskSelectedInstrumentId = ""
        vm.barDeclarationSymbol = "RE"
        let nse = InstrumentResult(
            trading_symbol: "RELIANCE",
            name: "Reliance Industries",
            exchange: "NSE",
            segment: "NSE",
            instrument_token: 2885,
            last_price: 0
        )
        vm.selectSymbol(nse)
        #expect(vm.barDeclarationLastError == "Select a Kotak cash instrument")
        #expect(vm.deskSelectedInstrumentId.isEmpty)
        #expect(vm.barDeclarationSymbol == "RE")
    }

    @Test func selectSymbolKotakNfoOnOptionsSetsTickBookId() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        vm.showSymbolSuggestions = true
        let nfo = InstrumentResult(
            trading_symbol: "BANKNIFTY25SEP57500CE",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: 12345,
            last_price: 0
        )
        vm.symbolSuggestions = [nfo]
        vm.selectSymbol(nfo)
        #expect(vm.barDeclarationLastError == nil)
        #expect(vm.deskSelectedInstrumentId == "nse_fo|12345")
        #expect(vm.barDeclarationSymbol == "BANKNIFTY25SEP57500CE")
        #expect(vm.symbolSuggestions.isEmpty)
        #expect(vm.showSymbolSuggestions == false)
    }

    @Test func selectSymbolRefusesNfoOnKotakCashDeclare() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .equity
        vm.deskSelectedInstrumentId = ""
        let nfo = InstrumentResult(
            trading_symbol: "BANKNIFTY25SEP57500CE",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: 12345,
            last_price: 0
        )
        vm.selectSymbol(nfo)
        #expect(vm.barDeclarationLastError == "Select a Kotak cash instrument")
        #expect(vm.deskSelectedInstrumentId.isEmpty)
    }

    @Test func selectSymbolRefusesCashOnKotakOptionsDeclare() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = ""
        let cash = InstrumentResult(
            trading_symbol: "NMDC",
            name: "NMDC Limited",
            exchange: "kotak_neo",
            segment: "nse_cm",
            instrument_token: 11532,
            last_price: 0
        )
        vm.selectSymbol(cash)
        #expect(vm.barDeclarationLastError == "Select a Kotak NFO instrument")
        #expect(vm.deskSelectedInstrumentId.isEmpty)
    }

    @Test func selectSymbolKotakNfoKeepsChainOnTheNamedNfoBook() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        let nfo = InstrumentResult(
            trading_symbol: "BANKNIFTY25SEP57500CE",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: 12345,
            last_price: 0
        )
        vm.selectSymbol(nfo)
        #expect(vm.deskSelectedInstrumentId == "nse_fo|12345")
        let chain = vm.deskChainExtractPath(symbol: "BANKNIFTY")
        #expect(chain.contains("book=kotak-nse-nfo"))
        #expect(chain.contains("instrument=BANKNIFTY"))
        #expect(!chain.contains("57500"))
    }

    @Test func selectSymbolBinanceSpotPairUnderOptionsFallsBackToSpot() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        let pair = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "Bitcoin",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 0
        )
        vm.selectSymbol(pair)
        // The instrument is spot, so the Options tab does not survive the selection.
        #expect(vm.declareAssetClass == .spot)
        #expect(vm.deskSelectedInstrumentId == "BTCUSDT")
        #expect(vm.barDeclarationLastError == nil)
        let chain = vm.deskChainExtractPath(symbol: "BTCUSDT")
        #expect(!chain.contains("kotak-nse-nfo"))
        #expect(!chain.contains("book="))
    }

    @Test func selectSymbolBinancePairOnUsdmStaysUsdmAndDoesNotSnapToSpot() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .usdm
        let pair = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "Bitcoin",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 65000
        )
        vm.selectSymbol(pair)
        #expect(vm.declareAssetClass == .usdm)
        #expect(vm.deskSelectedInstrumentId == "BTCUSDT")
        #expect(vm.declEntryPrice.isEmpty)
        #expect(vm.deskQuoteExtractPath(instrument: "BTCUSDT").contains("book=binance-com-usdm"))
        #expect(vm.canSubmitBarDeclaration)
        #expect(!vm.canExecuteSelectedInstrument())
    }

    @Test func selectSymbolCoinmPairStaysCoinmAndDoesNotShareUsdmBook() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .coinm
        let pair = InstrumentResult(
            trading_symbol: "BTCUSD_PERP",
            name: "BTCUSD_PERP",
            exchange: "binance_com",
            segment: "COINM",
            instrument_token: 0,
            last_price: 65000
        )
        vm.selectSymbol(pair)
        #expect(vm.declareAssetClass == .coinm)
        #expect(vm.deskSelectedInstrumentId == "BTCUSD_PERP")
        #expect(vm.declEntryPrice.isEmpty)
        #expect(vm.deskQuoteExtractPath(instrument: "BTCUSD_PERP").contains("book=binance-com-coinm"))
        #expect(!vm.deskQuoteExtractPath(instrument: "BTCUSD_PERP").contains("book=binance-com-usdm"))
        #expect(!vm.canExecuteSelectedInstrument())
    }

    @Test func selectSymbolDatedContractOnUsdmDoesNotPaintOptionsLast() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .usdm
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.declEntryPrice = ""
        let contract = InstrumentResult(
            trading_symbol: "BTC-200730-9000-C",
            name: "BTC option",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 64000
        )
        vm.selectSymbol(contract)
        #expect(vm.declareAssetClass == .usdm)
        #expect(vm.deskSelectedInstrumentId == "BTC-200730-9000-C")
        #expect(!vm.shouldBindQuoteLast(adapter: "binance_com", instrumentId: "BTC-200730-9000-C"))
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTC-200730-9000-C",
            "book_id": "binance-com-options",
            "data": ["last": "0.001"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
    }

    @Test func selectSymbolBinanceContractKeepsOptionsWithDarkLastAndNoNfoBook() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        vm.declEntryPrice = ""
        let contract = InstrumentResult(
            trading_symbol: "BTC-200730-9000-C",
            name: "BTC option",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 64000
        )
        vm.selectSymbol(contract)
        #expect(vm.declareAssetClass == .options)
        // Verbatim casing — the spot lowercase normalize is agent-side and spot-only.
        #expect(vm.deskSelectedInstrumentId == "BTC-200730-9000-C")
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
        #expect(vm.deskChainStatus == "unavailable")
        #expect(!vm.deskChainExtractPath(symbol: "BTC-200730-9000-C").contains("kotak-nse-nfo"))
        #expect(!vm.deskOiExtractPath(symbol: "BTC-200730-9000-C").contains("kotak-nse-nfo"))
    }

    @Test func selectSymbolRebindingToAnotherPairReseedsEntryFromTheNewLast() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .spot
        let btc = InstrumentResult(
            trading_symbol: "BTCUSDT",
            name: "Bitcoin",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 65000
        )
        vm.selectSymbol(btc)
        #expect(vm.declEntryPrice == "65000.00")

        let eth = InstrumentResult(
            trading_symbol: "ETHUSDT",
            name: "Ethereum",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 3000
        )
        vm.selectSymbol(eth)

        // Rebinding is a new instrument, so the previous pair's Last must not stand in the
        // Entry field the user is about to declare against.
        #expect(vm.deskSelectedInstrumentId == "ETHUSDT")
        #expect(vm.declEntryPrice == "3000.00")
    }

    @Test func selectSymbolSamePairKeepsATypedEntry() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .spot
        let eth = InstrumentResult(
            trading_symbol: "ETHUSDT",
            name: "Ethereum",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 3000
        )
        vm.selectSymbol(eth)
        vm.declEntryPrice = "1.23"

        // Re-picking the row already bound is not a rebind — the typed Entry is the user's.
        vm.selectSymbol(eth)

        #expect(vm.deskSelectedInstrumentId == "ETHUSDT")
        #expect(vm.declEntryPrice == "1.23")
    }

    @Test func liveTickDoesNotClobberATypedEntryAfterRebinding() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSyncClass = "synced"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .spot
        let eth = InstrumentResult(
            trading_symbol: "ETHUSDT",
            name: "Ethereum",
            exchange: "binance_com",
            segment: nil,
            instrument_token: nil,
            last_price: 3000
        )
        vm.selectSymbol(eth)
        vm.declEntryPrice = "1.23"

        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "ETHUSDT",
            "data": ["last": "3500"],
            "provenance": ["adapter_id": "binance_com"],
        ])

        // Seeding is a bind-time act, not a tick-time one.
        #expect(vm.declEntryPrice == "1.23")
    }

    @Test func selectSymbolRefusesKotakIdentityOnBinanceDeskAndDropsNfoExtracts() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "binance_com"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = "BTCUSDT"
        vm.barDeclarationSymbol = "BTCUSDT"
        vm.deskChainStatus = "success"
        let nfo = InstrumentResult(
            trading_symbol: "BANKNIFTY25SEP57500CE",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: 12345,
            last_price: 0
        )
        vm.selectSymbol(nfo)
        #expect(vm.barDeclarationLastError == "Select a Binance instrument")
        // Refused: neither the symbol field nor the bound instrument moves.
        #expect(vm.deskSelectedInstrumentId == "BTCUSDT")
        #expect(vm.barDeclarationSymbol == "BTCUSDT")
        // …and the NFO chain that was standing on screen goes dark.
        #expect(vm.deskChainStatus == "unavailable")
    }

    @Test func selectSymbolKotakOptionsWithoutTokenDoesNotPaintUnderlyingTicker() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = "kotak_neo"
        vm.brokerSessionActive = true
        vm.declareAssetClass = .options
        vm.deskSelectedInstrumentId = ""
        vm.declEntryPrice = ""
        vm.deskLastStatus = "unavailable"
        let ghost = InstrumentResult(
            trading_symbol: "BANKNIFTY",
            name: "BANKNIFTY",
            exchange: "kotak_neo",
            segment: "nse_fo",
            instrument_token: nil,
            last_price: 0
        )
        vm.selectSymbol(ghost)
        #expect(vm.deskSelectedInstrumentId != "BANKNIFTY")
        #expect(vm.deskSelectedInstrumentId.isEmpty)
        #expect(!InstrumentTickBookId.isNfoIdentity(vm.deskSelectedInstrumentId))
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(!vm.shouldBindQuoteLast(adapter: "kotak_neo", instrumentId: "BANKNIFTY"))
    }

    @Test func applyStationQuoteEnvelopeOptionsRejectsBinanceSpotLast() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "binance_com"
        vm.declEntryPrice = ""
        vm.declareAssetClass = .options
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "BTCUSDT",
            "data": ["last": "65000"],
            "provenance": ["adapter_id": "binance_com"],
        ])
        #expect(vm.deskLastStatus == "unavailable")
        #expect(vm.deskQuoteCapability == "unavailable")
        #expect(vm.declEntryPrice.isEmpty)
    }

    @Test func applyStationQuoteEnvelopeOptionsBindsNfoLast() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.brokerSyncClass = "synced"
        vm.activeBrokerSlug = "kotak_neo"
        vm.declEntryPrice = ""
        vm.declareAssetClass = .options
        vm.applyStationQuoteEnvelope([
            "status": "fresh",
            "instrument_id": "nse_fo|12345",
            "data": ["last": "245.50"],
            "provenance": ["adapter_id": "kotak_neo"],
        ])
        #expect(vm.deskLastStatus == "fresh")
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.declEntryPrice == "245.50")
    }

    @Test func emptySlugWhileSessionActiveIsCatalogDesk() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.activeBrokerSlug = nil
        vm.barProtectiveBrokerSlug = ""
        vm.brokerSessionActive = true
        #expect(vm.connectedInstrumentCatalogDesk)
        vm.brokerSessionActive = false
        vm.brokerSyncClass = "not_connected"
        #expect(!vm.connectedInstrumentCatalogDesk)
    }

    @Test func applyBrokerSyncStatePayloadOfflineKeepsLastKnownCapabilitiesWithoutBlendingFx() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "quoteCurrency": "INR",
            "capabilities": [
                "quote": "fresh",
                "funds": "fresh",
                "fills": "fresh",
                "instruments": "loading",
            ],
        ])
        vm.applyBrokerSyncStatePayload([
            "syncState": "not_connected",
            "brokerSlug": "kotak_neo",
        ])
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.deskFundsCapability == "fresh")
        #expect(vm.deskInstrumentsCapability == "loading")
        #expect(vm.deskQuoteCurrency == "INR")
    }

    @Test func applyBrokerSyncStatePayloadOmittedCapabilitiesDoNotPaintQuoteAccountRed() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "capabilities": [
                "quote": "fresh",
                "funds": "fresh",
                "fills": "fresh",
                "instruments": "fresh",
            ],
        ])
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
        ])
        #expect(vm.deskQuoteCapability == "fresh")
        #expect(vm.deskFundsCapability == "fresh")
        #expect(vm.deskFillsCapability == "fresh")
        #expect(vm.deskInstrumentsCapability == "fresh")
        #expect(DeskCapabilityChrome.dotName(forStatus: vm.deskQuoteCapability) == "teal")
    }

    @Test func venueEgressStateBannedDoesNotClearComSlug() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "binance_com",
            "quoteCurrency": "USDT",
        ])
        #expect(vm.activeBrokerSlug == "binance_com")
        #expect(vm.brokerSyncClass == "synced")

        let untilMs: Int64 = 4_102_444_800_000
        let applied = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "binance_com",
                "posture": "banned",
                "until_ms": untilMs,
            ],
            immediateToolbarShow: false
        )
        #expect(applied)
        #expect(vm.activeBrokerSlug == "binance_com")
        #expect(vm.brokerSyncClass == "synced")
        #expect(vm.venuePostureBySlug["binance_com"]?.venue == "binance_com")
        #expect(vm.venuePostureBySlug["binance_com"]?.posture == "banned")
        #expect(vm.venuePostureBySlug["binance_com"]?.untilMs == untilMs)
    }

    @Test func venueEgressStateComBannedLeavesKotakLiveInMap() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let kotakApplied = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "kotak_neo",
                "posture": "live",
            ],
            immediateToolbarShow: false
        )
        #expect(kotakApplied)
        #expect(vm.venuePostureBySlug["kotak_neo"]?.posture == "live")

        let untilMs: Int64 = 4_102_444_800_000
        let comApplied = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "binance_com",
                "posture": "banned",
                "until_ms": untilMs,
            ],
            immediateToolbarShow: false
        )
        #expect(comApplied)
        #expect(vm.venuePostureBySlug["binance_com"]?.posture == "banned")
        #expect(vm.venuePostureBySlug["kotak_neo"]?.posture == "live")
        #expect(vm.venuePostureBySlug.count == 2)
    }

    @Test func venueEgressStateDoesNotChangeKillOverlay() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        #expect(vm.killSwitchOverlayVisible == false)
        #expect(vm.killSwitchActive == false)

        let untilMs: Int64 = 4_102_444_800_000
        let venueApplied = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "binance_com",
                "posture": "banned",
                "until_ms": untilMs,
            ],
            immediateToolbarShow: false
        )
        #expect(venueApplied)
        #expect(vm.killSwitchOverlayVisible == false)
        #expect(vm.killSwitchActive == false)

        let killApplied = vm.applyDaemonEventPayload(
            type: "kill_switch_state",
            payload: [
                "active": true,
                "level": "L2",
                "countdown_secs": 90,
                "requires_ack": true,
            ],
            immediateToolbarShow: false
        )
        #expect(killApplied)
        #expect(vm.killSwitchActive == true)
        #expect(vm.killSwitchLevel == "L2")
        #expect(vm.killSwitchCountdownSecs == 90)
        #expect(vm.killSwitchRequiresAck == true)
        #expect(vm.killSwitchOverlayVisible == true)
    }

    @Test func applyDeskHonestyDisconnectedKeepsComSlug() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "binance_com",
            "quoteCurrency": "USDT",
        ])
        #expect(vm.activeBrokerSlug == "binance_com")

        let applied = vm.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: [
                "class": "disconnected",
                "brokerSlug": "binance_com",
            ],
            immediateToolbarShow: false
        )
        #expect(applied)
        #expect(vm.brokerSyncClass == "disconnected")
        #expect(vm.activeBrokerSlug == "binance_com")
    }

    @Test func brokerPillChromeBannedComIsNotNoBrokerOffline() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "binance_com",
            "quoteCurrency": "USDT",
        ])
        let untilMs: Int64 = 4_102_444_800_000
        _ = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "binance_com",
                "posture": "banned",
                "until_ms": untilMs,
            ],
            immediateToolbarShow: false
        )
        let chrome = NotchViewModel.brokerPillChrome(
            brokerSyncClass: vm.brokerSyncClass,
            slug: vm.activeBrokerSlug,
            venuePosture: vm.venuePostureBySlug["binance_com"]
        )
        #expect(chrome.label != "No broker · offline")
        #expect(chrome.label.hasPrefix("Binance.com · banned"))
        #expect(chrome.dotName == "amber" || chrome.dotName == "red")
    }

    @Test func brokerPillChromeKotakLiveIndependentOfComBanned() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyBrokerSyncStatePayload([
            "syncState": "synced",
            "brokerSlug": "kotak_neo",
            "quoteCurrency": "INR",
        ])
        _ = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "kotak_neo",
                "posture": "live",
            ],
            immediateToolbarShow: false
        )
        let untilMs: Int64 = 4_102_444_800_000
        _ = vm.applyDaemonEventPayload(
            type: "venue_egress_state",
            payload: [
                "venue": "binance_com",
                "posture": "banned",
                "until_ms": untilMs,
            ],
            immediateToolbarShow: false
        )
        #expect(vm.activeBrokerSlug == "kotak_neo")
        #expect(vm.venuePostureBySlug["binance_com"]?.posture == "banned")
        #expect(vm.venuePostureBySlug["kotak_neo"]?.posture == "live")

        let chrome = NotchViewModel.brokerPillChrome(
            brokerSyncClass: vm.brokerSyncClass,
            slug: vm.activeBrokerSlug,
            venuePosture: vm.venuePostureBySlug["kotak_neo"]
        )
        #expect(chrome.label == "Kotak Neo · live")
        #expect(chrome.dotName == "teal")
        #expect(chrome.label != "No broker · offline")
    }
}
