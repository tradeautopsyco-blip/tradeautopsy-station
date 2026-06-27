import Foundation

/// Merges streaming partial results with finalized utterances for a single dictation session.
/// `userPrefix` is the field text before dictation started (user-typed + any prior finals).
struct DictationTranscriptComposer: Equatable {
    private(set) var userPrefix: String
    private(set) var finalizedSegment: String

    init(userPrefix: String = "", finalizedSegment: String = "") {
        self.userPrefix = userPrefix
        self.finalizedSegment = finalizedSegment
    }

    /// Current phrase still being recognized.
    private(set) var partialUtterance: String = ""

    private static func join(_ a: String, _ b: String) -> String {
        if a.isEmpty { return b }
        if b.isEmpty { return a }
        return a + " " + b
    }

    /// Full text for the bound field while dictation runs.
    var displayText: String {
        Self.join(Self.join(userPrefix, finalizedSegment), partialUtterance)
    }

    mutating func setUserPrefix(_ text: String) {
        userPrefix = text
    }

    mutating func applyPartial(_ text: String) {
        partialUtterance = text
    }

    /// Appends a finalized utterance and clears the live partial buffer.
    mutating func applyFinal(_ text: String) {
        let t = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !t.isEmpty else {
            partialUtterance = ""
            return
        }
        finalizedSegment = Self.join(finalizedSegment, t)
        partialUtterance = ""
    }

    mutating func resetForNewSession(keepingUserPrefix: String) {
        userPrefix = keepingUserPrefix
        finalizedSegment = ""
        partialUtterance = ""
    }
}
