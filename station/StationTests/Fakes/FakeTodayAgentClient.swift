import Foundation
import Testing
@testable import Station

final class FakeTodayAgentClient: TodayAgentClient {
    var payload: TodayAgentPayload?

    init(payload: TodayAgentPayload? = nil) {
        self.payload = payload
    }

    func fetchToday() async -> TodayAgentPayload? {
        payload
    }
}
