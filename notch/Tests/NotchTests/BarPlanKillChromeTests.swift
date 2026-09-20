import Foundation
import Testing
@testable import Notch

/// T5 PLAN Kill — warning card, no Stop me, T5 fire body (PRD Q6–Q11).
struct BarPlanKillChromeTests {
    @Test func killVisibleWhenAgentUpEvenWithoutPosition() {
        let chrome = BarPlanKillChrome.presentation(phase: .idle, agentUp: true)
        #expect(chrome.showsKillButton == true)
        #expect(chrome.showsStopMe == false)
        #expect(chrome.showsWarningCard == false)
        #expect(chrome.killButtonTitle == "Kill")
    }

    @Test func killHiddenWhenAgentDown() {
        let chrome = BarPlanKillChrome.presentation(phase: .idle, agentUp: false)
        #expect(chrome.showsKillButton == false)
        #expect(chrome.showsWarningCard == false)
        #expect(chrome.showsStopMe == false)
    }

    @Test func warningCardUsesLockedCopy() {
        let chrome = BarPlanKillChrome.presentation(phase: .warning, agentUp: true)
        #expect(chrome.showsWarningCard == true)
        #expect(chrome.showsKillButton == false)
        #expect(chrome.showsStopMe == false)
        #expect(chrome.warningTitle == "Kill")
        #expect(chrome.warningBody.contains("Stop is not Kill"))
        #expect(chrome.warningBody.contains("not a max-loss flatten"))
        #expect(!chrome.warningBody.lowercased().contains("max-loss trip"))
        #expect(chrome.confirmTitle == "Confirm")
        #expect(chrome.cancelTitle == "Cancel")
    }

    @Test func confirmBodyIsT5KillNotStopMe() {
        let body = BarPlanKillChrome.fireBody()
        #expect(body["reason"] as? String == "notch_manual")
        #expect(body["level"] == nil)
        #expect(body["triggered_at_ms"] == nil)
        #expect(body["action"] as? String != "stop-me")
    }
}
