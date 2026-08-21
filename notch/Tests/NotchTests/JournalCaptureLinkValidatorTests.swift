import Foundation
import Testing
@testable import Notch

struct JournalCaptureLinkValidatorTests {
    @Test func imageOnlyAllowsEmptyCaptionWhenTradeIsLinked() {
        let trade = "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee"
        #expect(
            JournalCaptureLinkValidator.validationMessage(
                draftTrimmed: "",
                explicitPending: false,
                tradeRawTrimmed: trade,
                imageOnly: true
            ) == nil
        )
    }

    @Test func imageOnlyRejectsUnlinkedPending() {
        let msg = JournalCaptureLinkValidator.validationMessage(
            draftTrimmed: "",
            explicitPending: true,
            tradeRawTrimmed: "",
            imageOnly: true
        )
        #expect(msg == "Pick a trade to send to the journal.")
    }

    @Test func noteStillRequiresTextWhenNotImageOnly() {
        let msg = JournalCaptureLinkValidator.validationMessage(
            draftTrimmed: "",
            explicitPending: false,
            tradeRawTrimmed: "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee",
            imageOnly: false
        )
        #expect(msg == "Note cannot be empty.")
    }
}
