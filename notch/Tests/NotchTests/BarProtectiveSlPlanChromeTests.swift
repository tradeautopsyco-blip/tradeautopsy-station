import Foundation
import Testing
@testable import Notch

/// T5 K3 / PRD Q2 B — Set SL chrome and `place_sl` send are cut; Cancel SL stays.
struct BarProtectiveSlPlanChromeTests {
    @Test func setSlButtonAbsentWhenSlStatusMissing() {
        let chrome = BarProtectiveSlPlanChrome.presentation(
            slStatus: "missing",
            slPrice: 1_425,
            slFailureReason: nil,
            formattedPrice: "1,425"
        )
        #expect(chrome.showsSetSlButton == false)
        #expect(chrome.statusText?.localizedCaseInsensitiveContains("not placed") == true)
        #expect(chrome.showsCancelSlButton == false)
    }

    @Test func planDoesNotBuildPlaceSlJSON() {
        let data = BarProtectiveSlPlanChrome.placeSlJSONFromPlan()
        #expect(data == nil)
        if let data {
            let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any]
            #expect(obj?["action"] as? String != "place_sl")
        }
        #expect(BarInterventionCardSpec.primaryAccessory(interventionType: "bar_protective_sl") == .none)
        #expect(BarInterventionCardSpec.primaryAccessory(interventionType: "naked_window") == .none)
    }

    @Test func cancelSlStillOfferedWhenSlStatusPlaced() {
        let chrome = BarProtectiveSlPlanChrome.presentation(
            slStatus: "placed",
            slPrice: 1_425,
            slFailureReason: nil,
            formattedPrice: "1,425"
        )
        #expect(chrome.showsCancelSlButton == true)
        #expect(chrome.showsSetSlButton == false)
        #expect(chrome.statusText?.localizedCaseInsensitiveContains("live") == true)
    }
}
