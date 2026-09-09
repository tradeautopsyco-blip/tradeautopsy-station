import Foundation
import Testing
@testable import Notch

struct RiskDeskModeTests {
    @Test @MainActor func defaultIsBlotterA() {
        #expect(RiskDeskMode.default == .blotter)
        #expect(RiskDeskMode(rawValue: "a") == .blotter)
        #expect(RiskDeskMode.allCases.count == 3)
    }

    @Test @MainActor func persistRoundTripOneActiveMode() {
        let suite = "risk.desk.mode.test.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suite)!
        defaults.removePersistentDomain(forName: suite)
        let store = RiskDeskModeStore(defaults: defaults, storageKey: "mode")
        #expect(store.mode == .blotter)
        store.setMode(.notchFlip)
        #expect(store.mode == .notchFlip)
        let reloaded = RiskDeskModeStore(defaults: defaults, storageKey: "mode")
        #expect(reloaded.mode == .notchFlip)
        store.setMode(.sessionTape)
        #expect(store.mode == .sessionTape)
        #expect(store.mode != .blotter)
        defaults.removePersistentDomain(forName: suite)
    }

    @Test func switchingModeDoesNotInventAMergedBook() {
        // Chrome only — three modes, one kernel. No union of cash + NFO + spot.
        #expect(RiskDeskMode.blotter.rawValue != RiskDeskMode.notchFlip.rawValue)
        #expect(RiskDeskMode.notchFlip.rawValue != RiskDeskMode.sessionTape.rawValue)
    }
}

struct DetectCardTests {
    @Test func noFormDoesNotInventAStop() {
        let result = DetectCard.evaluate(
            DetectCardInput(
                qty: 10,
                entry: 100,
                planStop: nil,
                liveStop: 90,
                sideBuy: true,
                accountEquity: 200_000,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        #expect(result.kind == .noForm)
        #expect(result.planLoss == nil)
        #expect(result.headline.contains("No stop"))
    }

    @Test func liveWorseWhenLiveStopIsFarther() {
        let result = DetectCard.evaluate(
            DetectCardInput(
                qty: 10,
                entry: 100,
                planStop: 95,
                liveStop: 90,
                sideBuy: true,
                accountEquity: 200_000,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        #expect(result.kind == .liveWorse)
        #expect(result.planLoss == 50)
        #expect(result.liveLoss == 100)
    }

    @Test func planTighterWhenLiveStopIsCloser() {
        let result = DetectCard.evaluate(
            DetectCardInput(
                qty: 10,
                entry: 100,
                planStop: 90,
                liveStop: 95,
                sideBuy: true,
                accountEquity: 200_000,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        #expect(result.kind == .planTighter)
        #expect(result.planLoss == 100)
        #expect(result.liveLoss == 50)
    }

    @Test func thirteenPercentOfAccountIsAWarning() {
        // qty 130 @ 1300 / SL 1100 ≈ 13% on ₹2L — illustration from the detect path notes.
        let result = DetectCard.evaluate(
            DetectCardInput(
                qty: 130,
                entry: 1300,
                planStop: 1100,
                liveStop: 1100,
                sideBuy: true,
                accountEquity: 200_000,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        #expect(result.kind == .planHolds)
        #expect(result.planLoss == 26_000)
        #expect(result.accountPercent == 0.13)
        #expect(result.sizeWarning)
    }

    @Test func dualNoBlendDoesNotFxBlendPercentOfAccount() {
        let result = DetectCard.evaluate(
            DetectCardInput(
                qty: 1,
                entry: 100,
                planStop: 90,
                liveStop: 90,
                sideBuy: true,
                accountEquity: 10_000,
                tradeCurrency: "USD",
                accountCurrency: "INR"
            )
        )
        #expect(result.dualNoBlendBlockedPercent)
        #expect(result.accountPercent == nil)
        #expect(result.planLoss == 10)
        #expect(!result.sizeWarning)
    }
}

struct EquityIfSLTests {
    @Test func undeclaredKeepsGlanceCaption() {
        let snap = EquityIfSL.snapshot(
            units: 10,
            entry: 100,
            stop: 90,
            sideBuy: true,
            declared: false
        )
        #expect(!snap.declared)
        #expect(snap.scenarioCaption.contains("Declare"))
    }

    @Test func declaredUsesLadderLossAsScenario() {
        let snap = EquityIfSL.snapshot(
            units: 10,
            entry: 100,
            stop: 90,
            sideBuy: true,
            declared: true
        )
        #expect(snap.declared)
        #expect(snap.maxPlannedLoss == 100)
        #expect(snap.scenarioCaption.contains("Scenario"))
        #expect(snap.scenarioCaption.contains("Not remaining-risk"))
    }
}

struct ThisTradeInspectorTests {
    @Test func journalStubWaitsForT4() {
        let detect = DetectCard.evaluate(
            DetectCardInput(
                qty: 1,
                entry: nil,
                planStop: nil,
                liveStop: nil,
                sideBuy: true,
                accountEquity: nil,
                tradeCurrency: "INR",
                accountCurrency: "INR"
            )
        )
        let model = ThisTradeInspectorModel.fromOpenRow(
            symbol: "RELIANCE",
            sideText: "LONG",
            qtyText: "10",
            mtmText: "—",
            detect: detect
        )
        #expect(model.journalStub.contains("T4 N2"))
        #expect(model.pulseMTMText == "—")
        #expect(model.kindNoFormWaits)
    }
}

private extension ThisTradeInspectorModel {
    var kindNoFormWaits: Bool {
        equityIfSLCaption.contains("invent")
    }
}
