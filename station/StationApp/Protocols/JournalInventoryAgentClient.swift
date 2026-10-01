import Foundation

public protocol JournalInventoryAgentClient: Sendable {
    func fetchWeekTripCites(weekStart: String?, weekEnd: String?) async -> [JournalClosedTripInventoryRow]
}
