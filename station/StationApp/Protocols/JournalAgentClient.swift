import Foundation

@MainActor
public protocol JournalAgentClient {
    /// Consume-only. Notch remains the only writer — this client has no declare POST.
    func fetchWeek() async -> JournalWeekPayload?
}
