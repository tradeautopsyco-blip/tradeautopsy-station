import Foundation

/// Mirrors server rules in `toolbar-pending-capture.ts` (`INVALID_LINK_STRATEGY`, `AMBIGUOUS_LINK`, `EMPTY_CONTENT`).
enum JournalCaptureLinkValidator {
    static func validationMessage(
        draftTrimmed: String,
        explicitPending: Bool,
        tradeRawTrimmed: String,
        imageOnly: Bool = false
    ) -> String? {
        if imageOnly {
            if explicitPending || tradeRawTrimmed.isEmpty {
                return "Pick a trade to send to the journal."
            }
            if UUID(uuidString: tradeRawTrimmed) == nil {
                return "Trade ID must be a valid UUID."
            }
            return nil
        }
        if draftTrimmed.isEmpty {
            return "Note cannot be empty."
        }
        if explicitPending, !tradeRawTrimmed.isEmpty {
            return "Remove the trade ID or uncheck “pending”."
        }
        if !explicitPending, tradeRawTrimmed.isEmpty {
            return "Enter a trade UUID or save as pending."
        }
        if !explicitPending, UUID(uuidString: tradeRawTrimmed) == nil {
            return "Trade ID must be a valid UUID."
        }
        return nil
    }
}
