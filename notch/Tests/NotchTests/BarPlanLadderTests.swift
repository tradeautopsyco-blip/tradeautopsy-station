import Foundation
import Testing
@testable import Notch

struct BarPlanLadderTests {
    @Test func buyOneLotStopBelowEntryIsLoss() {
        let rung1 = BarPlanLadder.rung1(units: 1, entry: 100, stop: 80, sideBuy: true)
        #expect(rung1 == -20)
        #expect(BarPlanLadder.maxPlannedLossINR(rung1: rung1) == 20)
        #expect(BarPlanLadder.honesty(rung1: rung1) == .declared)
    }

    @Test func sellTwoLotsStopAboveEntryIsLoss() {
        let rung1 = BarPlanLadder.rung1(units: 2, entry: 100, stop: 120, sideBuy: false)
        #expect(rung1 == -40)
        #expect(BarPlanLadder.maxPlannedLossINR(rung1: rung1) == 40)
    }

    @Test func missingStopOrEntryOrLotsIsEmpty() {
        #expect(BarPlanLadder.rung1(units: 1, entry: 100, stop: nil, sideBuy: true) == nil)
        #expect(BarPlanLadder.rung1(units: 1, entry: nil, stop: 80, sideBuy: true) == nil)
        #expect(BarPlanLadder.rung1(units: nil, entry: 100, stop: 80, sideBuy: true) == nil)
        #expect(BarPlanLadder.honesty(rung1: nil) == .empty)
        #expect(BarPlanLadder.maxPlannedLossINR(rung1: nil) == nil)
    }

    @Test func stopNotALossDoesNotInventMaxPlannedLoss() {
        let rung1 = BarPlanLadder.rung1(units: 1, entry: 100, stop: 120, sideBuy: true)
        #expect(rung1 == 20)
        #expect(BarPlanLadder.maxPlannedLossINR(rung1: rung1) == nil)
    }

    @Test func rung1HasNoChainParameter() {
        let rung1 = BarPlanLadder.rung1(units: 1, entry: 100, stop: 80, sideBuy: true)
        #expect(rung1 == -20)
        #expect(BarPlanLadder.maxPlannedLossINR(units: 1, entry: 100, stop: 80, sideBuy: true) == 20)
    }
}
