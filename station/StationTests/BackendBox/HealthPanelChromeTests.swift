import Foundation
import Notch
import Testing
@testable import Station

struct HealthPanelChromeTests {
    @Test func fourParentsAndVendorsHangUnderBoxWithoutSlugsOrKeys() {
        let chrome = HealthPanelChrome.compose(
            agentHealthy: true,
            agentMessage: nil,
            vendorRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "up",
                    what: "licensed_history India ohlcv (Kotak has none)",
                    why: "up",
                    obtainNoun: "history"
                ),
                VendorFenceRow(
                    adapterId: "amfi",
                    status: "unsupported",
                    what: "AMFI NAV (labs)",
                    why: "unsupported",
                    obtainNoun: "amfi_nav"
                ),
            ],
            killActive: false
        )
        #expect(chrome.rows.map(\.id) == ["station", "backend_box", "kill", "harness"])
        #expect(chrome.rows[0].status == "up")
        #expect(chrome.rows[1].status == "unsupported")
        #expect(chrome.rows[1].children.map(\.title) == ["History", "NAV"])
        #expect(chrome.rows[1].children.map(\.status) == ["up", "unsupported"])
        #expect(chrome.rows[2].status == "idle")
        #expect(chrome.rows[3].status == "up")
        let text = chrome.visibleText
        #expect(text.contains("licensed_history") == false)
        #expect(text.contains("lh-fixture") == false)
        #expect(text.contains("History"))
        #expect(text.contains("NAV"))
    }

    @Test func agentDownPaintsStationAndHarnessDown() {
        let chrome = HealthPanelChrome.compose(
            agentHealthy: false,
            agentMessage: "Agent unavailable",
            vendorRows: [],
            killActive: false
        )
        #expect(chrome.rows[0].status == "down")
        #expect(chrome.rows[0].why == "Agent unavailable")
        #expect(chrome.rows[3].status == "down")
    }

    @Test func killFiringIsNotIdle() {
        let chrome = HealthPanelChrome.compose(
            agentHealthy: true,
            agentMessage: nil,
            vendorRows: [],
            killActive: true
        )
        #expect(chrome.rows[2].status == "firing")
        #expect(chrome.rows[2].title == "Kill")
    }

    @Test func exhaustedVendorMakesBoxExhausted() {
        let chrome = HealthPanelChrome.compose(
            agentHealthy: true,
            agentMessage: nil,
            vendorRows: [
                VendorFenceRow(
                    adapterId: "licensed_history",
                    status: "exhausted",
                    what: "history",
                    why: "exhausted",
                    obtainNoun: "history"
                ),
            ],
            killActive: false
        )
        #expect(chrome.rows[1].status == "exhausted")
        #expect(chrome.rows[1].children[0].error == nil)
    }

    @Test @MainActor func viewModelStartsWithFourParents() {
        let viewModel = HealthPanelViewModel(
            agentHealthy: { true },
            agentMessage: { nil },
            killActive: { false }
        )
        #expect(viewModel.chrome.rows.map(\.id) == ["station", "backend_box", "kill", "harness"])
        #expect(viewModel.chrome.rows[2].status == "idle")
    }
}
