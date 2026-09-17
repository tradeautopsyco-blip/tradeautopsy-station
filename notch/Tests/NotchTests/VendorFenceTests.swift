import Foundation
import Testing
@testable import Notch

struct VendorFenceTests {
    @Test func healthVendorsBecomeFenceLinesWithoutLastOrKeys() {
        let json: [String: Any] = [
            "vendors": [
                [
                    "adapter_id": "licensed_history",
                    "status": "up",
                    "what": "licensed_history India ohlcv (Kotak has none)",
                    "why": "up",
                    "error": NSNull(),
                    "obtain_noun": "history",
                    "key": "lh-fixture-key-must-not-appear",
                    "last": "1234.50",
                ],
                [
                    "adapter_id": "amfi",
                    "status": "unsupported",
                    "what": "AMFI NAV (labs)",
                    "why": "unsupported",
                    "obtain_noun": "amfi_nav",
                ],
                [
                    "adapter_id": "kotak_neo",
                    "status": "up",
                    "what": "broker last",
                ],
            ],
        ]
        let rows = VendorFence.rows(fromHealthJSON: json)
        #expect(rows.map(\.adapterId) == ["licensed_history", "amfi"])
        #expect(rows[0].fenceLine.contains("licensed_history · up"))
        #expect(rows[0].fenceLine.contains("Kotak has none"))
        #expect(rows[0].fenceLine.contains("lh-fixture-key-must-not-appear") == false)
        #expect(rows[0].fenceLine.contains("1234.50") == false)
        #expect(rows[1].status == "unsupported")
    }

    @Test @MainActor func applyVendorFenceOnViewModel() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        vm.applyVendorFence(from: [
            "vendors": [
                [
                    "adapter_id": "amfi",
                    "status": "up",
                    "what": "AMFI NAV (labs)",
                    "why": "up",
                ],
            ],
        ])
        #expect(vm.vendorFenceRows.count == 1)
        #expect(vm.vendorFenceRows[0].adapterId == "amfi")
        #expect(vm.vendorFenceRows[0].fenceLine.contains("last") == false)
    }
}

struct VendorHistoryObtainTests {
    @Test func offNeverFetches() {
        #expect(VendorHistoryObtain.kotakHistoryPath(mode: .off, armed: true) == nil)
        #expect(VendorHistoryObtain.kotakHistoryPath(mode: .off, armed: false) == nil)
    }

    @Test func onObtainOnlyWhenArmed() {
        #expect(VendorHistoryObtain.kotakHistoryPath(mode: .onObtain, armed: false) == nil)
        #expect(
            VendorHistoryObtain.kotakHistoryPath(mode: .onObtain, armed: true)
                == VendorHistoryObtain.kotakHistoryPath
        )
    }

    @Test func paneAutoAlwaysFetches() {
        #expect(
            VendorHistoryObtain.kotakHistoryPath(mode: .paneAuto, armed: false)
                == "/api/station/obtain?adapter=kotak_neo&operation=history"
        )
    }

    @Test func obtainNowPromotesOffToOnObtainAndArms() {
        let suite = "vendor.fetchMode.obtainNow.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        #expect(VendorFetchModeStore.mode(for: "licensed_history", defaults: defaults) == .off)
        VendorFetchModeStore.prepareObtainNow(adapterId: "licensed_history", defaults: defaults)
        #expect(VendorFetchModeStore.mode(for: "licensed_history", defaults: defaults) == .onObtain)
        #expect(VendorFetchModeStore.consumeArmed("licensed_history", defaults: defaults) == true)
        #expect(VendorFetchModeStore.consumeArmed("licensed_history", defaults: defaults) == false)
    }

    @Test func obtainNowLeavesPaneAutoAndStillArms() {
        let suite = "vendor.fetchMode.paneAuto.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        VendorFetchModeStore.set(.paneAuto, for: "amfi", defaults: defaults)
        VendorFetchModeStore.prepareObtainNow(adapterId: "amfi", defaults: defaults)
        #expect(VendorFetchModeStore.mode(for: "amfi", defaults: defaults) == .paneAuto)
        #expect(VendorFetchModeStore.consumeArmed("amfi", defaults: defaults) == true)
    }
}
