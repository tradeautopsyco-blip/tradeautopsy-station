import Foundation
import Testing
@testable import Notch

struct BarLiveStatePollChromeTests {
    @Test func noLastGoodAndInFlightShowsLoading() {
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: false, isInFlight: true) == true)
    }

    @Test func lastGoodAndInFlightDoesNotShowLoading() {
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: true, isInFlight: true) == false)
    }

    @Test func notInFlightNeverShowsLoading() {
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: false, isInFlight: false) == false)
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: true, isInFlight: false) == false)
    }

    @Test func lastGoodOneTransientFailDoesNotPublishError() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: true,
                consecutiveFailures: 1,
                isDeviceLogin: false
            ) == false
        )
    }

    @Test func lastGoodTwoTransientFailsPublishesError() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: true,
                consecutiveFailures: 2,
                isDeviceLogin: false
            ) == true
        )
    }

    @Test func noLastGoodFirstFailPublishesError() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: false,
                consecutiveFailures: 1,
                isDeviceLogin: false
            ) == true
        )
    }

    @Test func deviceLoginFailPublishesImmediately() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: true,
                consecutiveFailures: 1,
                isDeviceLogin: true
            ) == true
        )
    }

    @Test func successResetsConsecutiveCounter() {
        let afterFail = BarLiveStatePollChrome.nextConsecutiveFailures(previous: 1, succeeded: false)
        #expect(afterFail == 2)
        #expect(BarLiveStatePollChrome.nextConsecutiveFailures(previous: afterFail, succeeded: true) == 0)
    }

    @Test func scoreJitterDoesNotChangeLiveStateIdentity() {
        let a = Self.sampleNotch(score: 0.11, signals: [BarBehaviorSignalRow(signal: "revenge", value: 0.2)])
        let b = Self.sampleNotch(score: 0.19, signals: [BarBehaviorSignalRow(signal: "revenge", value: 0.9)])
        #expect(a == b)
    }

    private static func sampleNotch(
        score: Double?,
        signals: [BarBehaviorSignalRow]
    ) -> BarLiveStateResponse {
        BarLiveStateResponse(
            planState: "GREEN",
            primarySentence: nil,
            triggerType: nil,
            isRedTerminal: false,
            composite: nil,
            activeInterventions: [],
            syncState: "GREEN",
            lastSyncAt: nil,
            declarationSubmitBlocked: nil,
            pendingDeclaration: nil,
            behavioralScore: score,
            behaviorSignals: signals
        )
    }
}
