import Foundation
import Testing
@testable import Notch

struct BarLiveStatePollChromeTests {
    @Test func loadingIsAlwaysFalse() {
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: false, isInFlight: true) == false)
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: true, isInFlight: true) == false)
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: false, isInFlight: false) == false)
        #expect(BarLiveStatePollChrome.showsLoading(hasLiveState: true, isInFlight: false) == false)
    }

    @Test func lastGoodTransientFailsNeverPublishError() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: true,
                consecutiveFailures: 1,
                isDeviceLogin: false
            ) == false
        )
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: true,
                consecutiveFailures: 8,
                isDeviceLogin: false
            ) == false
        )
    }

    @Test func noLastGoodTransientFailNeverPublishesError() {
        #expect(
            BarLiveStatePollChrome.shouldPublishError(
                hasLiveState: false,
                consecutiveFailures: 1,
                isDeviceLogin: false
            ) == false
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

    @Test func freshnessLabelWaitingNineSecondsAndMinutes() {
        let now = Date(timeIntervalSince1970: 1_700_000_000)
        #expect(BarLiveStatePollChrome.freshnessLabel(lastFetched: nil, now: now) == "Waiting")
        #expect(
            BarLiveStatePollChrome.freshnessLabel(
                lastFetched: now.addingTimeInterval(-9),
                now: now
            ) == "9s ago"
        )
        #expect(
            BarLiveStatePollChrome.freshnessLabel(
                lastFetched: now.addingTimeInterval(-120),
                now: now
            ) == "2m ago"
        )
    }

    @Test func staleAfterTenSecondsWithoutSuccess() {
        let now = Date(timeIntervalSince1970: 1_700_000_000)
        #expect(BarLiveStatePollChrome.isStale(lastFetched: nil, now: now) == false)
        #expect(
            BarLiveStatePollChrome.isStale(lastFetched: now.addingTimeInterval(-9), now: now) == false
        )
        #expect(
            BarLiveStatePollChrome.isStale(lastFetched: now.addingTimeInterval(-10), now: now) == true
        )
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
