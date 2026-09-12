import Foundation
import Testing
@testable import Station

struct DemoDeskStoreTests {
    @MainActor
    private func makeStore() -> (store: DemoDeskStore, defaults: UserDefaults, suite: String) {
        let suite = "demo.desk.store.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = DemoDeskStore(defaults: defaults, storageKey: "tradeautopsy.demoDesk.enabled")
        return (store, defaults, suite)
    }

    @Test @MainActor func demoEnabledDefaultsFalse() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }
        #expect(harness.store.demoEnabled == false)
    }

    @Test @MainActor func demoEnabledRoundTripsThroughInjectedUserDefaults() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setEnabled(true)
        #expect(harness.store.demoEnabled == true)

        let reloaded = DemoDeskStore(defaults: harness.defaults, storageKey: "tradeautopsy.demoDesk.enabled")
        #expect(reloaded.demoEnabled == true)

        harness.store.setEnabled(false)
        let off = DemoDeskStore(defaults: harness.defaults, storageKey: "tradeautopsy.demoDesk.enabled")
        #expect(off.demoEnabled == false)
    }

    @Test @MainActor func togglingOnDoesNotEncodeLossLimitsOrWriteDeskRulesMoney() {
        let harness = makeStore()
        defer { harness.defaults.removePersistentDomain(forName: harness.suite) }

        harness.store.setEnabled(true)
        #expect(
            DeskRulesPresentation.lossLimitsRequest(
                dailyFloor: 12_000,
                meanLoss: 5_400,
                maxRoundTrips: 12
            ) == nil
        )
        #expect(harness.defaults.object(forKey: "tradeautopsy.deskRules.dailyFloor") == nil)
        #expect(harness.defaults.object(forKey: "tradeautopsy.deskRules.meanLoss") == nil)
        #expect(harness.defaults.bool(forKey: "tradeautopsy.demoDesk.enabled") == true)
    }
}
