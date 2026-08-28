import Foundation
import Testing
@testable import Notch

struct HonestyChipTests {
    @Test func fourRawValuesAndNoFifth() {
        let raw = HonestyStatus.allCases.map(\.rawValue)
        #expect(raw == ["empty", "unavailable", "unusable", "inherited_dark"])
        #expect(HonestyStatus.allCases.count == 4)
        #expect(HonestyStatus(rawValue: "lit") == nil)
        #expect(HonestyStatus(rawValue: "fresh") == nil)
        #expect(HonestyStatus(rawValue: "stale") == nil)
        #expect(HonestyStatus(rawValue: "success") == nil)
    }

    @Test func chipLabelsMatchCatalog() {
        #expect(HonestyStatus.empty.chipLabel == "empty")
        #expect(HonestyStatus.unavailable.chipLabel == "unavailable")
        #expect(HonestyStatus.unusable.chipLabel == "unusable")
        #expect(HonestyStatus.inheritedDark.chipLabel == "inherited dark")
    }

    @Test func catalogShowsAllFour() {
        #expect(HonestyStatus.allCases.count == 4)
        for status in HonestyStatus.allCases {
            _ = HonestyChip(status: status)
        }
    }

    @Test func chainOiWireParsesAsHonestyNotFreshness() {
        #expect(HonestyStatus.fromWire("unavailable") == .unavailable)
        #expect(HonestyStatus.fromWire("empty") == .empty)
        #expect(HonestyStatus.fromWire("unusable") == .unusable)
        #expect(HonestyStatus.fromWire(" inherited_dark ") == .inheritedDark)
        #expect(HonestyStatus.fromWire("fresh") == nil)
        #expect(HonestyStatus.fromWire("stale") == nil)
    }
}
