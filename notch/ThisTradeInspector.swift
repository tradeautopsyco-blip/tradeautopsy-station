import Foundation

/// Station-local this-trade depth. Console journal images wait for T4 N2.
public struct ThisTradeInspectorModel: Equatable, Sendable, Identifiable {
    public var id: String { symbol }
    public var symbol: String
    public var sideText: String
    public var qtyText: String
    public var planStopText: String
    public var liveStopText: String
    public var pulseMTMText: String
    public var equityIfSLCaption: String
    public var journalStub: String

    public static let journalWaitForT4 = "Journal images wait for T4 N2 — this inspector does not link Console captures."

    public init(
        symbol: String,
        sideText: String,
        qtyText: String,
        planStopText: String,
        liveStopText: String,
        pulseMTMText: String,
        equityIfSLCaption: String,
        journalStub: String = Self.journalWaitForT4
    ) {
        self.symbol = symbol
        self.sideText = sideText
        self.qtyText = qtyText
        self.planStopText = planStopText
        self.liveStopText = liveStopText
        self.pulseMTMText = pulseMTMText
        self.equityIfSLCaption = equityIfSLCaption
        self.journalStub = journalStub
    }

    /// Pulse MTM only when the row already has it. Missing stays em dash — not zero.
    public static func fromOpenRow(
        symbol: String,
        sideText: String,
        qtyText: String,
        mtmText: String,
        detect: DetectCardResult
    ) -> ThisTradeInspectorModel {
        let plan = detect.planLoss.map { String(format: "%.0f", $0) } ?? "—"
        let live = detect.liveLoss.map { String(format: "%.0f", $0) } ?? "—"
        let slCaption: String
        if detect.kind == .noForm {
            slCaption = "No declared stop — inspector does not invent one."
        } else {
            slCaption = detect.body
        }
        return ThisTradeInspectorModel(
            symbol: symbol,
            sideText: sideText,
            qtyText: qtyText,
            planStopText: plan,
            liveStopText: live,
            pulseMTMText: mtmText,
            equityIfSLCaption: slCaption
        )
    }
}
