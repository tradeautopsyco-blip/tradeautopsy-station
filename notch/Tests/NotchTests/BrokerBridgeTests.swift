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
}
