import Foundation
import Testing
@testable import Station

struct FakeTodayAgentClient: TodayAgentClient {
    var payload: TodayAgentPayload?

    func fetchToday() async -> TodayAgentPayload? {
        payload
    }
}
