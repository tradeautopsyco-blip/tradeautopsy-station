import Foundation
import Testing
@testable import Notch

struct PlanHeaderFreshnessTests {
    @Test func connectionLabelMatchesBrokersJustNow() {
        let now = Date(timeIntervalSince1970: 1_000_000)
        let ms = Int64((now.timeIntervalSince1970 - 2) * 1000)
        let label = PlanHeaderFreshness.connectionChipLabel(
            syncClass: "synced",
            lastSyncedAtMs: ms,
            requiresDeviceLogin: false,
            now: now
        )
        #expect(label == "just now")
    }

    @Test func connectionLabelOfflineWhenNotConnected() {
        let label = PlanHeaderFreshness.connectionChipLabel(
            syncClass: "not_connected",
            lastSyncedAtMs: nil,
            requiresDeviceLogin: false,
            now: Date()
        )
        #expect(label == "Offline")
    }

    @Test func quoteLaneOnlyWhenStale() {
        #expect(
            PlanHeaderFreshness.quoteLaneChipLabel(
                quoteCapability: "fresh",
                laneObservedAt: Date(),
                now: Date()
            ) == nil
        )
        let observed = Date(timeIntervalSince1970: 1_000_000)
        let now = observed.addingTimeInterval(120)
        #expect(
            PlanHeaderFreshness.quoteLaneChipLabel(
                quoteCapability: "stale",
                laneObservedAt: observed,
                now: now
            ) == "Quote stale · 2m"
        )
    }

}
