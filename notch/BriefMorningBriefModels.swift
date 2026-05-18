import Foundation

// MARK: - #127 — daemon `/api/daemon/morning-brief` behavioral contract (unified reference §BriefTab)

struct BriefBehavioralPattern: Decodable, Equatable, Identifiable {
    let condition: String
    let confidence: String
    let costInr: Double?
    let recoveredInr: Double?

    var id: String { condition + "|" + confidence }

    enum CodingKeys: String, CodingKey {
        case condition, confidence
        case costInrSnake = "cost_inr"
        case costInrCamel = "costINR"
        case recoveredInrSnake = "recovered_inr"
        case recoveredInrCamel = "recoveredINR"
    }

    init(condition: String, confidence: String, costInr: Double?, recoveredInr: Double?) {
        self.condition = condition
        self.confidence = confidence
        self.costInr = costInr
        self.recoveredInr = recoveredInr
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        condition = try c.decodeIfPresent(String.self, forKey: .condition) ?? ""
        confidence = try c.decodeIfPresent(String.self, forKey: .confidence) ?? ""
        costInr =
            try c.decodeIfPresent(Double.self, forKey: .costInrSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .costInrCamel)
        recoveredInr =
            try c.decodeIfPresent(Double.self, forKey: .recoveredInrSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .recoveredInrCamel)
    }
}

struct BriefOwnMetrics: Decodable, Equatable {
    let stopRespected: Int
    let stopTotal: Int
    let exitPlanDriven: Int
    let exitTotal: Int

    enum CodingKeys: String, CodingKey {
        case stopRespectedSnake = "stop_respected"
        case stopRespectedCamel = "stopRespected"
        case stopTotalSnake = "stop_total"
        case stopTotalCamel = "stopTotal"
        case exitPlanDrivenSnake = "exit_plan_driven"
        case exitPlanDrivenCamel = "exitPlanDriven"
        case exitTotalSnake = "exit_total"
        case exitTotalCamel = "exitTotal"
    }

    init(stopRespected: Int, stopTotal: Int, exitPlanDriven: Int, exitTotal: Int) {
        self.stopRespected = stopRespected
        self.stopTotal = stopTotal
        self.exitPlanDriven = exitPlanDriven
        self.exitTotal = exitTotal
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        stopRespected =
            try c.decodeIfPresent(Int.self, forKey: .stopRespectedSnake)
            ?? c.decodeIfPresent(Int.self, forKey: .stopRespectedCamel) ?? 0
        stopTotal =
            try c.decodeIfPresent(Int.self, forKey: .stopTotalSnake)
            ?? c.decodeIfPresent(Int.self, forKey: .stopTotalCamel) ?? 0
        exitPlanDriven =
            try c.decodeIfPresent(Int.self, forKey: .exitPlanDrivenSnake)
            ?? c.decodeIfPresent(Int.self, forKey: .exitPlanDrivenCamel) ?? 0
        exitTotal =
            try c.decodeIfPresent(Int.self, forKey: .exitTotalSnake)
            ?? c.decodeIfPresent(Int.self, forKey: .exitTotalCamel) ?? 0
    }
}

private struct BriefCautionWireRow: Decodable {
    let symbol: String?
    let reason: String?
    let note: String?
}

/// Inner `briefing` object for `GET /api/daemon/morning-brief` — pre-market fields + behavioral (#127).
struct BriefMorningBriefPayload: Decodable {
    let summary: String?
    let briefing: String?
    let niftyFutures: Double
    let bankniftyFutures: Double
    let niftyChangePct: Double
    let bankniftyChangePct: Double
    let vix: Double
    let edgeSymbols: [String]
    let cautionSymbols: [CautionSymbolRow]
    let recommendation: String?

    let tradeCount: Int
    let isNewUser: Bool
    let patterns: [BriefBehavioralPattern]
    let ownMetrics: BriefOwnMetrics?

    let behavioralDateLine: String?
    let behavioralHeadline: String?
    let sessionPnLKpi: Double?
    let planAdherencePct: Double?
    let winRateKpi: Double?
    let leftOnTableInr: Double?
    let nonNegotiableRule: String?

    enum CodingKeys: String, CodingKey {
        case summary, briefing, recommendation, vix, patterns
        case niftyFuturesSnake = "nifty_futures"
        case niftyFuturesCamel = "niftyFutures"
        case bankniftyFuturesSnake = "banknifty_futures"
        case bankniftyFuturesCamel = "bankniftyFutures"
        case niftyChangePctSnake = "nifty_change_pct"
        case niftyChangePctCamel = "niftyChangePct"
        case bankniftyChangePctSnake = "banknifty_change_pct"
        case bankniftyChangePctCamel = "bankniftyChangePct"
        case edgeSymbolsSnake = "edge_symbols"
        case edgeSymbolsCamel = "edgeSymbols"
        case cautionSymbolsSnake = "caution_symbols"
        case cautionSymbolsCamel = "cautionSymbols"
        case tradeCountSnake = "trade_count"
        case tradeCountCamel = "tradeCount"
        case isNewUserSnake = "is_new_user"
        case isNewUserCamel = "isNewUser"
        case ownMetricsSnake = "own_metrics"
        case ownMetricsCamel = "ownMetrics"
        case behavioralDateLineSnake = "behavioral_date_line"
        case behavioralDateLineCamel = "behavioralDateLine"
        case behavioralHeadlineSnake = "headline_sentence"
        case behavioralHeadlineCamel = "headlineSentence"
        case sessionPnLKpiSnake = "session_pnl_kpi"
        case sessionPnLKpiCamel = "sessionPnLKpi"
        case planAdherencePctSnake = "plan_adherence_pct"
        case planAdherencePctCamel = "planAdherencePct"
        case winRateKpiSnake = "win_rate_kpi"
        case winRateKpiCamel = "winRateKpi"
        case leftOnTableInrSnake = "left_on_table_inr"
        case leftOnTableInrCamel = "leftOnTableInr"
        case nonNegotiableRuleSnake = "non_negotiable_rule"
        case nonNegotiableRuleCamel = "nonNegotiableRule"
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        summary = try c.decodeIfPresent(String.self, forKey: .summary)
        briefing = try c.decodeIfPresent(String.self, forKey: .briefing)
        recommendation = try c.decodeIfPresent(String.self, forKey: .recommendation)

        niftyFutures =
            try c.decodeIfPresent(Double.self, forKey: .niftyFuturesSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .niftyFuturesCamel) ?? 0
        bankniftyFutures =
            try c.decodeIfPresent(Double.self, forKey: .bankniftyFuturesSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .bankniftyFuturesCamel) ?? 0
        niftyChangePct =
            try c.decodeIfPresent(Double.self, forKey: .niftyChangePctSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .niftyChangePctCamel) ?? 0
        bankniftyChangePct =
            try c.decodeIfPresent(Double.self, forKey: .bankniftyChangePctSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .bankniftyChangePctCamel) ?? 0
        vix = try c.decodeIfPresent(Double.self, forKey: .vix) ?? 0

        edgeSymbols =
            try c.decodeIfPresent([String].self, forKey: .edgeSymbolsSnake)
            ?? c.decodeIfPresent([String].self, forKey: .edgeSymbolsCamel) ?? []

        let cautionCamel = try c.decodeIfPresent([BriefCautionWireRow].self, forKey: .cautionSymbolsCamel) ?? []
        let cautionSnake = try c.decodeIfPresent([BriefCautionWireRow].self, forKey: .cautionSymbolsSnake) ?? []
        let cautionWire = cautionCamel.isEmpty ? cautionSnake : cautionCamel
        cautionSymbols = cautionWire.compactMap { row in
            guard let sym = row.symbol, !sym.isEmpty else { return nil }
            let reason = row.reason ?? row.note ?? ""
            return CautionSymbolRow(symbol: sym, reason: reason)
        }

        tradeCount =
            try c.decodeIfPresent(Int.self, forKey: .tradeCountSnake)
            ?? c.decodeIfPresent(Int.self, forKey: .tradeCountCamel) ?? 0
        isNewUser =
            try c.decodeIfPresent(Bool.self, forKey: .isNewUserSnake)
            ?? c.decodeIfPresent(Bool.self, forKey: .isNewUserCamel) ?? false
        patterns = try c.decodeIfPresent([BriefBehavioralPattern].self, forKey: .patterns) ?? []
        ownMetrics =
            try c.decodeIfPresent(BriefOwnMetrics.self, forKey: .ownMetricsSnake)
            ?? c.decodeIfPresent(BriefOwnMetrics.self, forKey: .ownMetricsCamel)

        behavioralDateLine =
            try c.decodeIfPresent(String.self, forKey: .behavioralDateLineSnake)
            ?? c.decodeIfPresent(String.self, forKey: .behavioralDateLineCamel)
        behavioralHeadline =
            try c.decodeIfPresent(String.self, forKey: .behavioralHeadlineSnake)
            ?? c.decodeIfPresent(String.self, forKey: .behavioralHeadlineCamel)

        sessionPnLKpi =
            try c.decodeIfPresent(Double.self, forKey: .sessionPnLKpiSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .sessionPnLKpiCamel)
        planAdherencePct =
            try c.decodeIfPresent(Double.self, forKey: .planAdherencePctSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .planAdherencePctCamel)
        winRateKpi =
            try c.decodeIfPresent(Double.self, forKey: .winRateKpiSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .winRateKpiCamel)
        leftOnTableInr =
            try c.decodeIfPresent(Double.self, forKey: .leftOnTableInrSnake)
            ?? c.decodeIfPresent(Double.self, forKey: .leftOnTableInrCamel)
        nonNegotiableRule =
            try c.decodeIfPresent(String.self, forKey: .nonNegotiableRuleSnake)
            ?? c.decodeIfPresent(String.self, forKey: .nonNegotiableRuleCamel)
    }

    func makeMorningBrief(fetchedAt: Date) -> MorningBrief {
        let sum = (summary ?? briefing ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        let rec = (recommendation ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        return MorningBrief(
            summary: sum,
            niftyFutures: niftyFutures,
            bankniftyFutures: bankniftyFutures,
            niftyChangePct: niftyChangePct,
            bankniftyChangePct: bankniftyChangePct,
            vix: vix,
            edgeSymbols: edgeSymbols,
            cautionSymbols: cautionSymbols,
            recommendation: rec,
            fetchedAt: fetchedAt,
            tradeCount: tradeCount,
            isNewUser: isNewUser,
            patterns: patterns,
            ownMetrics: ownMetrics,
            behavioralDateLine: behavioralDateLine,
            behavioralHeadline: behavioralHeadline,
            sessionPnLKpi: sessionPnLKpi,
            planAdherencePct: planAdherencePct,
            winRateKpi: winRateKpi,
            leftOnTableInr: leftOnTableInr,
            nonNegotiableRule: nonNegotiableRule,
        )
    }
}

/// Normalized daemon envelope: `{ "briefing_contract_version", "briefing" }` only — Swift ignores legacy flat shapes (#7).
struct BriefMorningBriefResponse: Decodable {
    let briefingContractVersion: Int
    let briefing: BriefMorningBriefPayload

    enum CodingKeys: String, CodingKey {
        case briefingContractVersion = "briefing_contract_version"
        case briefing
    }

    init(from decoder: Decoder) throws {
        let c = try decoder.container(keyedBy: CodingKeys.self)
        briefingContractVersion = try c.decodeIfPresent(Int.self, forKey: .briefingContractVersion) ?? 1
        briefing = try c.decode(BriefMorningBriefPayload.self, forKey: .briefing)
    }

    func makeMorningBrief(fetchedAt: Date) -> MorningBrief {
        briefing.makeMorningBrief(fetchedAt: fetchedAt)
    }
}
