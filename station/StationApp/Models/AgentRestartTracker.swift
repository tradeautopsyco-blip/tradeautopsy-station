import Foundation

/// Tracks crash-loop window and exponential backoff for agent auto-restart.
struct AgentRestartTracker {
    static let maxRestartsPerWindow = 3
    static let windowDuration: TimeInterval = 60
    static let baseBackoff: TimeInterval = 1

    private(set) var crashTimestamps: [Date] = []
    private(set) var restartAttemptCount = 0

    mutating func recordCrash(at date: Date) {
        pruneOldCrashes(before: date.addingTimeInterval(-Self.windowDuration))
        crashTimestamps.append(date)
    }

    mutating func resetOnSuccessfulAttach() {
        crashTimestamps.removeAll()
        restartAttemptCount = 0
    }

    func crashLoopExceeded(at date: Date) -> Bool {
        let windowStart = date.addingTimeInterval(-Self.windowDuration)
        return crashTimestamps.filter { $0 >= windowStart }.count >= Self.maxRestartsPerWindow
    }

    mutating func nextBackoffDelay() -> TimeInterval {
        let delay = Self.baseBackoff * pow(2.0, Double(restartAttemptCount))
        restartAttemptCount += 1
        return delay
    }

    private mutating func pruneOldCrashes(before cutoff: Date) {
        crashTimestamps.removeAll { $0 < cutoff }
    }
}
