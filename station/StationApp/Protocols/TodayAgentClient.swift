import Foundation

@MainActor
public protocol TodayAgentClient {
    func fetchToday() async -> TodayAgentPayload?
}
