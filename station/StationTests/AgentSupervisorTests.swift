import Foundation
import Testing
@testable import Station

struct AgentSupervisorTests {
    // Backoff intervals increase exponentially between restart attempts
    @Test func backoffIntervalsIncreaseExponentially() {
        var tracker = AgentRestartTracker()

        #expect(tracker.nextBackoffDelay() == 1)
        #expect(tracker.nextBackoffDelay() == 2)
        #expect(tracker.nextBackoffDelay() == 4)
        #expect(tracker.nextBackoffDelay() == 8)
    }

    // Restart count resets after successful attach
    @Test func restartCountResetsAfterSuccessfulAttach() {
        var tracker = AgentRestartTracker()
        _ = tracker.nextBackoffDelay()
        _ = tracker.nextBackoffDelay()
        tracker.recordCrash(at: Date())

        tracker.resetOnSuccessfulAttach()

        #expect(tracker.restartAttemptCount == 0)
        #expect(tracker.crashTimestamps.isEmpty)
        #expect(tracker.nextBackoffDelay() == 1)
    }

    // Crash loop window is rolling 60s (not cumulative)
    @Test func crashLoopWindowIsRollingSixtySeconds() {
        var tracker = AgentRestartTracker()
        let base = Date(timeIntervalSinceReferenceDate: 100_000)

        tracker.recordCrash(at: base)
        tracker.recordCrash(at: base.addingTimeInterval(20))
        tracker.recordCrash(at: base.addingTimeInterval(40))
        #expect(tracker.crashLoopExceeded(at: base.addingTimeInterval(40)))

        var agedTracker = AgentRestartTracker()
        agedTracker.recordCrash(at: base)
        agedTracker.recordCrash(at: base.addingTimeInterval(20))
        agedTracker.recordCrash(at: base.addingTimeInterval(61))
        #expect(agedTracker.crashLoopExceeded(at: base.addingTimeInterval(61)) == false)
    }
}
