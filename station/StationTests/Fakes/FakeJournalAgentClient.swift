import Foundation
import Testing
@testable import Station

@MainActor
final class FakeJournalAgentClient: JournalAgentClient {
    var payload: JournalWeekPayload?

    init(payload: JournalWeekPayload? = nil) {
        self.payload = payload
    }

    func fetchWeek() async -> JournalWeekPayload? {
        payload
    }
}
