import Foundation
import Testing
@testable import Notch

struct DeskRulesStoreTests {
    @MainActor
    private func makeStore() -> (store: DeskRulesStore, defaults: UserDefaults, suite: String) {
        let suite = "desk.rules.store.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = DeskRulesStore(defaults: defaults, storageKeyPrefix: "deskRules")
        return (store, defaults, suite)
    }

    @Test @MainActor func emptyDefaultsAreNilNotInventedRupees() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        #expect(harness.store.dailyFloor == nil)
        #expect(harness.store.meanLoss == nil)
        #expect(harness.store.maxRoundTrips == nil)
        #expect(harness.store.hideNotch == false)
    }

    @Test @MainActor func dailyFloorRoundTripsThroughUserDefaults() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setDailyFloor(12_000)
        #expect(harness.store.dailyFloor == 12_000)

        let reloaded = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(reloaded.dailyFloor == 12_000)
        #expect(reloaded.meanLoss == nil)
        #expect(reloaded.maxRoundTrips == nil)
    }

    @Test @MainActor func clearingDailyFloorDoesNotLeaveZeroRupees() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setDailyFloor(12_000)
        harness.store.setDailyFloor(nil)
        #expect(harness.store.dailyFloor == nil)

        let reloaded = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(reloaded.dailyFloor == nil)
    }

    @Test @MainActor func zeroFloorIsStoredZeroNotEmpty() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setDailyFloor(0)
        #expect(harness.store.dailyFloor == 0)

        let reloaded = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(reloaded.dailyFloor == 0)
    }

    @Test @MainActor func meanLossAndMaxRoundTripsRoundTrip() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setMeanLoss(5_400)
        harness.store.setMaxRoundTrips(12)
        #expect(harness.store.meanLoss == 5_400)
        #expect(harness.store.maxRoundTrips == 12)

        let reloaded = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(reloaded.meanLoss == 5_400)
        #expect(reloaded.maxRoundTrips == 12)
    }

    @Test @MainActor func hideNotchRoundTripsAndDefaultsFalse() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        #expect(harness.store.hideNotch == false)
        harness.store.setHideNotch(true)
        #expect(harness.store.hideNotch == true)

        let reloaded = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(reloaded.hideNotch == true)

        harness.store.setHideNotch(false)
        #expect(harness.store.hideNotch == false)
        let shown = DeskRulesStore(defaults: harness.defaults, storageKeyPrefix: "deskRules")
        #expect(shown.hideNotch == false)
    }
}
