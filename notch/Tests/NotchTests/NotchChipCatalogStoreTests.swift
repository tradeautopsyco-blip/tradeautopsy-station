import Foundation
import Testing
@testable import Notch

struct NotchChipCatalogStoreTests {
    @Test func impactCannotBeRemoved() {
        let start = NotchChipCatalogState()
        let next = NotchChipCatalogMutations.toggleExtra(NotchChipCatalogState.impactId, in: start)
        #expect(next == start)
        #expect(start.isOn(NotchChipCatalogState.impactId))
    }

    @Test func capFourExtras() {
        var state = NotchChipCatalogState()
        state = NotchChipCatalogMutations.toggleExtra("RELIANCE", in: state)
        state = NotchChipCatalogMutations.toggleExtra("NIFTY", in: state)
        state = NotchChipCatalogMutations.toggleExtra("limit_left", in: state)
        state = NotchChipCatalogMutations.toggleExtra("trades", in: state)
        #expect(state.extras.count == 4)
        let blocked = NotchChipCatalogMutations.toggleExtra("clock", in: state)
        #expect(blocked.extras == state.extras)
        #expect(!blocked.isOn("clock"))
    }

    @Test func emptyPinsMeanAllOpen() {
        let state = NotchChipCatalogState(extras: ["limit_left"])
        #expect(state.pinnedNameSymbols.isEmpty)
    }

    @Test func namePinsIgnoreSystemSlots() {
        let state = NotchChipCatalogState(extras: ["RELIANCE", "trades", "NIFTY"])
        #expect(state.pinnedNameSymbols == ["RELIANCE", "NIFTY"])
    }

    @Test func persistRoundTrip() {
        let suite = "notch.chip.catalog.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = UserDefaultsNotchChipCatalogStore(defaults: defaults, storageKey: "catalog")
        let saved = NotchChipCatalogState(extras: ["RELIANCE", "sync"], seenSymbols: ["RELIANCE", "NIFTY"])
        store.save(saved)
        #expect(store.load() == saved)
        defaults.removePersistentDomain(forName: suite)
    }

    @Test func rememberSymbolsSkipsDuplicatesAndSlots() {
        let start = NotchChipCatalogState(seenSymbols: ["RELIANCE"])
        let next = NotchChipCatalogMutations.rememberSymbols(
            ["reliance", "NIFTY", "trades"],
            in: start,
        )
        #expect(next.seenSymbols == ["RELIANCE", "NIFTY"])
    }
}
