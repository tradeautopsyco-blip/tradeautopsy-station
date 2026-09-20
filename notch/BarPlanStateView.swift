import SwiftUI
import UniformTypeIdentifiers

struct BarPlanStateView: View {
    @ObservedObject var viewModel: NotchViewModel
    /// When `false`, escrow ledger is shown only from sidebar (`BarNotchScreen.escrow`).
    var embedEscrow: Bool = true

    @State private var moveSlFallbackAlert: Bool = false
    @State private var cancelSlWebBarAlert: Bool = false
    @State private var showExitGateSheet: Bool = false
    @State private var showRecalibrateSheet: Bool = false
    @State private var scalperRecalibrateCalm: Int? = nil
    @State private var scalperRecalibrateConfidence: Int? = nil
    /// #4 — native swing check-in (thesis + capitulation chips).
    @State private var swingThesisIntact: Bool?
    @State private var swingCapitulationTag: String?
    @State private var swingCheckInNote: String = ""
    /// Primary kill/composite cards — minimum read window (#3).
    @State private var mandatoryReadStartedAtByInterventionId: [String: Date] = [:]
    @State private var interventionCountdownTick: Date = Date()
    @State private var scalperStepAwayAck: Bool = false

    private var payload: BarLiveStateResponse? { viewModel.barLiveState }

    private var sortedActiveInterventions: [ActiveIntervention] {
        BarInterventionCardSpec.sortedInterventions(payload?.activeInterventions ?? [])
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            BarUndeclaredPositionBanner(position: payload?.undeclaredPosition)

            if let pos = payload?.undeclaredPosition,
               let ccy = viewModel.formatQuoteCurrency
                ?? DeskMoneyFormatting.quoteCurrency(forBrokerSlug: viewModel.resolvedDeskSlug)
            {
                DetectCardView(
                    result: DetectCard.evaluate(
                        DetectCardInput(
                            qty: Double(pos.quantity),
                            entry: nil,
                            planStop: payload?.pendingDeclaration?.stopLoss,
                            liveStop: payload?.slPrice,
                            sideBuy: !pos.side.uppercased().contains("SELL"),
                            accountEquity: nil,
                            tradeCurrency: ccy,
                            accountCurrency: ccy
                        )
                    )
                )
            } else if let pending = payload?.pendingDeclaration,
                      let ccy = viewModel.formatQuoteCurrency
                        ?? DeskMoneyFormatting.quoteCurrency(forBrokerSlug: viewModel.resolvedDeskSlug)
            {
                DetectCardView(
                    result: DetectCard.evaluate(
                        DetectCardInput(
                            qty: pending.filledQty ?? pending.quantity,
                            entry: pending.avgFill,
                            planStop: pending.stopLoss,
                            liveStop: payload?.slPrice,
                            sideBuy: !pending.side.uppercased().contains("SELL"),
                            accountEquity: nil,
                            tradeCurrency: ccy,
                            accountCurrency: ccy
                        )
                    )
                )
            }

            planStateBanner

            if workingPriceInvalidated {
                invalidatedWorkingBanner
            }

            if shouldShowMetricStrip { liveMetricStrip }

            if shouldShowPlanSnapshot { planSnapshotRows }

            if !shouldShowMetricStrip, let composite = payload?.composite {
                compositeRiskLine(composite, showStaleSuffix: syncShowsStaleSuffix)
            }

            if shouldShowDeclareBeforeTradeCTA {
                declareBeforeTradeCTA
            }

            if shouldShowInterference { interferenceQuestionBlock }

            if let echo = viewModel.barInterferenceEcho, !echo.isEmpty {
                Text(echo)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                    .foregroundColor(interferenceEchoColor)
                    .lineSpacing(4)
                    .padding(9)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(interferenceEchoBackground)
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                            .stroke(interferenceEchoBorder, lineWidth: BarDS.borderThin),
                    )
                    .accessibilityLabel(echo)
                    .transition(.opacity)
                    .animation(.easeInOut(duration: 0.15), value: viewModel.barInterferenceEcho)
            }

            BarLiveCaptureCard(viewModel: viewModel)

            if shouldShowScalperSessionPanel { scalperSessionLivePanel }

            if shouldShowSwingCheckInPanel { swingDailyCheckInPanel }

            protectiveSlHintRow

            if shouldShowPlanSnapshot {
                livePlanFooterBar
                    .padding(.top, 4)
            }

            ForEach(Array(sortedActiveInterventions.enumerated()), id: \.element.id) { idx, intervention in
                interventionBanner(intervention, isPrimary: idx == 0)
            }

            if embedEscrow {
                BarEscrowMatchView(
                    report: payload?.escrowMatchReport,
                    pending: payload?.pendingDeclaration,
                    last: viewModel.deskQuoteLast,
                    lastStatus: viewModel.deskLastStatus
                )
            }

            if shouldShowPlanSnapshot {
                emotionNowRow
            }

            if shouldShowExitTradeSection {
                exitTradeSection
                    .padding(.top, 8)
            }

            if BarDeskTemplate.allowsVenueProtectivePlace(for: viewModel.declareAssetClass) {
                planKillSection
                    .padding(.top, 8)
            }
        }
        .padding(0)
        .onReceive(NotchOneSecondClock.publisher) { date in
            guard needsLiveSecondClock else { return }
            interventionCountdownTick = date
        }
        .alert("Move SL to entry", isPresented: $moveSlFallbackAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(
                "Notch needs broker order sync for modify SL — complete this action in Harness for now.",
            )
        }
        .alert("Cancel stop loss on broker?", isPresented: $cancelSlWebBarAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(
                "Cancel SL requires the live broker order id — finish in Harness until notch exposes the active order handle.",
            )
        }
        .alert("Step away", isPresented: $scalperStepAwayAck) {
            Button("OK", role: .cancel) {}
        } message: {
            Text(
                "Take about five minutes away from the screen. Nothing here enforces it — it is a commitment to yourself.",
            )
        }
        .sheet(isPresented: $showExitGateSheet) {
            BarExitGateView(
                onConfirmExit: {
                    showExitGateSheet = false
                    viewModel.showExitConfirmation()
                },
                onCancel: { showExitGateSheet = false },
            )
        }
        .sheet(isPresented: $showRecalibrateSheet) {
            BarRecalibrateEmotionalSheetView(
                calm: $scalperRecalibrateCalm,
                confidence: $scalperRecalibrateConfidence,
                onDone: { showRecalibrateSheet = false },
            )
        }
    }

    // MARK: - Banner ladder (sync → thesis → plan)

    private var syncShowsStaleSuffix: Bool {
        let s = payload?.syncState ?? ""
        return s == "STALE" || s == "EXPIRED"
    }

    private var workingPriceInvalidated: Bool {
        guard let pending = payload?.pendingDeclaration else { return false }
        let sideBuy = !pending.side.uppercased().contains("SELL")
        return BarWorkingCompare.isPriceInvalidated(
            sideBuy: sideBuy,
            last: viewModel.deskQuoteLast,
            status: viewModel.deskLastStatus,
            kind: pending.planSnapshot?.resolvedInvalidationKind,
            price: pending.planSnapshot?.resolvedInvalidationPrice
        )
    }

    private var invalidatedWorkingBanner: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Invalidated")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .semibold))
                .foregroundColor(BarDS.Accent.red)
            Text("Last crossed the declared invalidation price. Debrief this ticket or Kill the desk — stop does not dismiss Kill.")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 8) {
                Button {
                    viewModel.openWorkingDebrief()
                } label: {
                    Text("Debrief")
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
                        .foregroundColor(Color(hex: "#050505"))
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 10)
                        .background(BarDS.Accent.teal)
                        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Debrief")
                Button {
                    viewModel.presentPlanKillWarning()
                } label: {
                    Text("Kill")
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
                        .foregroundColor(BarDS.Accent.red)
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 10)
                        .background(BarDS.Accent.red.opacity(0.12))
                        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                                .stroke(BarDS.Accent.red.opacity(0.35), lineWidth: BarDS.borderThin)
                        )
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Kill")
            }
        }
        .padding(12)
        .background(BarDS.Accent.red.opacity(0.08))
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Accent.red.opacity(0.28), lineWidth: BarDS.borderThin)
        )
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Invalidated")
    }

    private var emotionNowRow: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("EMOTION NOW")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(Color.white.opacity(0.38))
                .tracking(0.6)
            HStack(spacing: 6) {
                ForEach(1...5, id: \.self) { n in
                    Button {
                        viewModel.declEmotionNow = n
                    } label: {
                        Text("\(n)")
                            .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .semibold))
                            .foregroundColor(
                                viewModel.declEmotionNow == n ? Color(hex: "#050505") : BarDS.Text.secondary
                            )
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                            .background(
                                viewModel.declEmotionNow == n
                                    ? BarDS.Accent.teal
                                    : Color.white.opacity(0.04)
                            )
                            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Emotion now \(n)")
                }
            }
        }
    }

    private var shouldShowPlanSnapshot: Bool {
        viewModel.hasOpenPositions
            || payload?.pendingDeclaration != nil
            || viewModel.barSurfacePhase == .armed
            || viewModel.barOptimisticArmedDisplay != nil
    }

    /// Mockup 5 — show interference whenever PLAN SNAPSHOT / live plan context is active (#113).
    private var shouldShowInterference: Bool {
        shouldShowPlanSnapshot
    }

    private var shouldShowMetricStrip: Bool {
        shouldShowPlanSnapshot
    }

    private var needsLiveSecondClock: Bool {
        shouldShowMetricStrip
            || sortedActiveInterventions.contains { $0.expiresAt != nil }
    }

    private var shouldShowScalperSessionPanel: Bool {
        let arch = payload?.archetype?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        guard arch == "scalper" else { return false }
        let sessionish =
            (payload?.isSessionLevel == true) || (payload?.sessionTradeCount ?? 0) > 0
            || (payload?.sessionMaxTrades ?? 0) > 0
        return sessionish
    }

    private var shouldShowSwingCheckInPanel: Bool {
        payload?.dailyCheckInRequired == true
    }

    /// Armed phase only — cancel pending declaration before fill (#144).
    private var shouldShowExitTradeSection: Bool {
        viewModel.barSurfacePhase == .armed && viewModel.barCancelDeclarationId != nil
    }

    private var planKillChrome: BarPlanKillChrome.Presentation {
        BarPlanKillChrome.presentation(
            phase: viewModel.planKillPhase,
            agentUp: viewModel.planKillAgentUp
        )
    }

    private var shouldShowDeclareBeforeTradeCTA: Bool {
        BarLiveTradeDeclareCTA.shouldShow(
            surfacePhase: viewModel.barSurfacePhase,
            showingDeclarationForm: viewModel.showingDeclarationForm,
            matchedDeclarationId: payload?.matchedDeclarationId,
            hasPendingDeclaration: payload?.pendingDeclaration != nil,
            hasOptimisticArmed: viewModel.barOptimisticArmedDisplay != nil,
        )
    }

    private var declareBeforeTradeCTA: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("No active trade — declare your plan before entering.")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundColor(Color(hex: "#888888"))
                .fixedSize(horizontal: false, vertical: true)
            BarBigButton(label: "Declare before trade →", style: .primary) {
                viewModel.presentBarDeclarationForm()
            }
        }
        .padding(12)
        .background(BarDS.Semantic.tealBg())
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Semantic.tealBorder(), lineWidth: BarDS.borderThin),
        )
    }

    private var planSnapshotFields: BarPlanSnapshotSummary? {
        payload?.pendingDeclaration?.planSnapshot
    }

    private var interferenceEchoColor: Color {
        if viewModel.barInterferenceChoice == "no" { return BarDS.Accent.teal }
        if viewModel.barInterferenceChoice == "maybe" { return BarDS.Accent.amber }
        if viewModel.barInterferenceChoice == "yes" { return BarDS.Accent.red }
        return BarDS.Text.secondary
    }

    private var interferenceEchoBackground: Color {
        switch viewModel.barInterferenceChoice {
        case "no": return BarDS.Accent.teal.opacity(0.06)
        case "maybe": return BarDS.Accent.amber.opacity(0.06)
        case "yes": return BarDS.Accent.red.opacity(0.06)
        default: return Color.white.opacity(0.04)
        }
    }

    private var interferenceEchoBorder: Color {
        switch viewModel.barInterferenceChoice {
        case "no": return BarDS.Accent.teal.opacity(0.15)
        case "maybe": return BarDS.Accent.amber.opacity(0.15)
        case "yes": return BarDS.Accent.red.opacity(0.15)
        default: return BarDS.Border.row
        }
    }

    private var liveMetricStrip: some View {
        let comp = payload?.composite
        let unreal = payload?.unrealizedPnL
        let worst = comp?.worstCase
        let unrealDisplay =
            unreal.map { formatSignedDeskMoney($0) }
            ?? worst.map { formatSignedDeskMoney($0) }
            ?? "—"
        let unrealColor: Color = {
            let raw = unreal ?? worst ?? 0
            if raw < 0 { return BarDS.Accent.red }
            if raw > 0 { return BarDS.Accent.green }
            return BarDS.Text.primary
        }()
        let maxLossDeclared = payload?.declaredMaxLossINR
        let maxLossDisplay = BarLivePlanMetricStripFormatting.maxLossDeclaredDisplayText(
            declaredMaxLossInr: maxLossDeclared,
            compositeWorstCaseFallback: worst,
            formatWholeAbs: { formatDeskWhole($0) },
        )
        let maxLossColor: Color = BarLivePlanMetricStripFormatting.maxLossUsesDeclaredOnly(declaredMaxLossInr: maxLossDeclared)
            ? BarDS.Text.primary
            : BarDS.Text.secondary
        let riskPct = Int((comp?.budgetPct ?? 0) * 100)
        let timeTrade = entryMinutesSuffix ?? "—"
        return VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 6) {
                metricPill(
                    title: "UNREALISED P&L",
                    value: unrealDisplay,
                    valueColor: unrealColor,
                    ax: "Unrealised P L, \(unrealDisplay)",
                )
                metricPill(
                    title: "MAX LOSS DECLARED",
                    value: maxLossDisplay,
                    valueColor: maxLossColor,
                    ax: "Max loss declared, \(maxLossDisplay)",
                )
            }
            HStack(spacing: 6) {
                metricPill(
                    title: "RISK USED",
                    value: "\(riskPct)%",
                    valueColor: riskUsedColor(riskPct: riskPct),
                    ax: "Risk used, \(riskPct) percent",
                )
                metricPill(
                    title: "TIME IN TRADE",
                    value: timeTrade,
                    valueColor: BarDS.Accent.amber,
                    ax: "Time in trade, \(timeTrade)",
                )
            }
        }
        .accessibilityElement(children: .contain)
    }

    private func riskUsedColor(riskPct: Int) -> Color {
        if riskPct >= 90 { return BarDS.Accent.red }
        if riskPct >= 60 { return BarDS.Accent.amber }
        return BarDS.Accent.teal
    }

    private func metricPill(title: String, value: String, valueColor: Color, ax: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(Color.white.opacity(0.45))
                .kerning(0.006 * 11)
            Text(value)
                .font(BarDS.monoFont(BarDS.FontSize.body, weight: .medium))
                .monospacedDigit()
                .foregroundColor(valueColor)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.vertical, 8)
        .padding(.horizontal, 10)
        .background(Color.white.opacity(0.04))
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(Color.white.opacity(0.08), lineWidth: BarDS.borderThin),
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel(ax)
    }

    /// Desk-honest money (R7 / T2.5) — follows active connection currency; never hardcode INR when COM is active.
    private func formatSignedDeskMoney(_ value: Double) -> String {
        viewModel.formatDeskMoney(value)
    }

    private func formatDeskWhole(_ value: Double) -> String {
        guard let ccy = viewModel.formatQuoteCurrency, !ccy.isEmpty else { return "—" }
        return DeskMoneyFormatting.formatWhole(value, quoteCurrency: ccy)
    }

    @ViewBuilder
    private var planSnapshotRows: some View {
        let stopPx = primaryStopNumeric.map { formatQtyPrice($0) } ?? "—"
        let stopMapped = BarLivePlanSnapshotMapping.stopLossValueDisplay(
            slStatus: payload?.slStatus,
            formattedPrice: stopPx,
        )
        VStack(alignment: .leading, spacing: 6) {
            Text("PLAN SNAPSHOT")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(Color.white.opacity(0.38))
                .tracking(0.6)
            planSnapshotRow(
                label: "Setup",
                value: BarLivePlanSnapshotMapping.setupDisplay(plan: planSnapshotFields),
            )
            planSnapshotRow(
                label: "Stop loss",
                value: stopMapped.text,
                valueColor: stopRowColor(stopMapped.tone),
            )
            planSnapshotRow(
                label: "Target",
                value: BarLivePlanSnapshotMapping.targetDisplay(
                    pendingTarget: payload?.pendingDeclaration?.target,
                    format: formatQtyPrice,
                ),
            )
            invalidationPlanSnapshotBlock
            planSnapshotRow(
                label: "Entry state",
                value: BarLivePlanSnapshotMapping.entryStateDisplay(plan: planSnapshotFields),
                valueColor: Color.white.opacity(0.62),
            )
        }
        .padding(10)
        .background(Color.white.opacity(0.04))
        .cornerRadius(8)
    }

    private var invalidationPlanSnapshotBlock: some View {
        let invText = BarLivePlanSnapshotMapping.invalidationDisplay(plan: planSnapshotFields)
        return VStack(alignment: .leading, spacing: 4) {
            HStack(alignment: .firstTextBaseline) {
                Text("Invalidation")
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.42))
                Spacer(minLength: 8)
                Text(invText)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(invalidationRowPrimaryColor(invalidationLine: invText))
            }
            if let greenLine = invalidationTimeGreenSuffix(invalidationLine: invText) {
                HStack {
                    Spacer(minLength: 0)
                    Text(greenLine)
                        .font(.system(size: 10, weight: .semibold, design: .rounded))
                        .foregroundColor(Color.green.opacity(0.88))
                        .multilineTextAlignment(.trailing)
                }
                .accessibilityLabel(greenLine)
            }
        }
    }

    private func invalidationRowPrimaryColor(invalidationLine: String) -> Color {
        let ps = payload?.planState?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        if ps == "GREEN", invalidationLine.lowercased().contains("time") {
            return Color.white.opacity(0.88)
        }
        return Color.orange.opacity(0.88)
    }

    private func invalidationTimeGreenSuffix(invalidationLine: String) -> String? {
        let ps = payload?.planState?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        guard ps == "GREEN", invalidationLine.lowercased().contains("time") else { return nil }
        return "Plan intact — time-based invalidation is the only clock that should close this."
    }

    private var primaryStopNumeric: Double? {
        if let p = payload?.pendingDeclaration?.stopLoss, p > 0 { return p }
        if let sp = payload?.slPrice, sp > 0 { return sp }
        return nil
    }

    private func stopRowColor(_ tone: BarLivePlanSnapshotMapping.StopRowTone) -> Color {
        switch tone {
        case .placedGreen: Color.green.opacity(0.88)
        case .missingRed: Color.red.opacity(0.88)
        case .neutral: Color.white.opacity(0.85)
        }
    }

    private func planSnapshotRow(label: String, value: String, valueColor: Color = Color.white.opacity(0.85))
        -> some View
    {
        HStack(alignment: .firstTextBaseline) {
            Text(label)
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .foregroundColor(Color.white.opacity(0.42))
            Spacer(minLength: 8)
            Text(value)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundColor(valueColor)
        }
    }

    private var livePlanFooterBar: some View {
        let venueActions = BarDeskTemplate.allowsVenueProtectivePlace(for: viewModel.declareAssetClass)
        return HStack(spacing: 8) {
            if venueActions {
                Button {
                    showExitGateSheet = true
                } label: {
                    footerPill(title: "Exit trade", danger: true)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Exit trade")

                Button {
                    moveSlFallbackAlert = true
                } label: {
                    footerPill(title: "Move SL to entry")
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Move stop loss to entry")
            }

            Button {
                viewModel.holdAndWatchBarLive()
            } label: {
                footerPill(title: "Hold and watch")
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Hold and watch")
        }
    }

    private func footerPill(title: String, danger: Bool = false) -> some View {
        Text(title)
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
            .foregroundColor(danger ? BarDS.Accent.red : Color.white.opacity(0.88))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 10)
            .padding(.horizontal, 6)
            .background(Color.white.opacity(0.06))
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                    .stroke(
                        danger ? BarDS.Accent.red.opacity(0.25) : Color.white.opacity(0.12),
                        lineWidth: BarDS.borderThin
                    ),
            )
    }

    private var scalperSessionLivePanel: some View {
        let count = Double(payload?.sessionTradeCount ?? 0)
        let maxT = Double(payload?.sessionMaxTrades ?? 0)
        let loss = payload?.sessionLossAmount ?? 0
        let lossLimit = payload?.sessionLossLimit ?? 0
        let tradeFrac = maxT > 0 ? min(1, count / maxT) : 0
        let lossFrac = lossLimit > 0 ? min(1, abs(loss) / lossLimit) : 0
        let windowFrac: Double = {
            guard let end = payload?.sessionWindowEndsISO,
                let elapsed = BarScalperSessionWindowProgress.elapsedFraction(
                    now: interventionCountdownTick,
                    windowEndsISO: end,
                    entryTimeISO: payload?.entryTimeISO,
                )
            else { return 0 }
            return min(1, max(0, elapsed))
        }()
        let remainingLabel: String = {
            guard let end = payload?.sessionWindowEndsISO else { return "—" }
            return BarScalperSessionWindowProgress.humanReadableRemaining(
                now: interventionCountdownTick,
                windowEndsISO: end,
            ) ?? "—"
        }()
        return VStack(alignment: .leading, spacing: 10) {
            Text("SCALPER SESSION")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(Color(hex: "#2E2E2E"))
                .tracking(0.6)
            HStack(spacing: 6) {
                scalperMetricTile(
                    title: "TRADES",
                    value: maxT > 0 ? "\(Int(count))/\(Int(maxT))" : "—",
                    valueColor: planStressColor(fraction: tradeFrac),
                )
                scalperMetricTile(
                    title: "SESSION LOSS",
                    value: lossLimit > 0
                        ? "\(formatSignedDeskMoney(loss)) · cap \(formatDeskWhole(lossLimit))"
                        : formatSignedDeskMoney(loss),
                    valueColor: lossLimit > 0 ? planStressColor(fraction: lossFrac) : Color.white.opacity(0.85),
                )
                scalperMetricTile(
                    title: "TIME LEFT",
                    value: remainingLabel,
                    valueColor: planStressColor(fraction: windowFrac),
                )
            }
            if maxT > 0 {
                scalperProgressRow(
                    label: "Trade count",
                    leftCaption: "\(Int(count)) of \(Int(maxT))",
                    fraction: tradeFrac,
                )
            }
            if lossLimit > 0 {
                scalperProgressRow(
                    label: "Session loss vs limit",
                    leftCaption: "\(Int(lossFrac * 100))% of cap",
                    fraction: lossFrac,
                )
            }
            if let end = payload?.sessionWindowEndsISO,
               BarScalperSessionWindowProgress.elapsedFraction(
                   now: interventionCountdownTick,
                   windowEndsISO: end,
                   entryTimeISO: payload?.entryTimeISO,
               ) != nil
            {
                scalperProgressRow(
                    label: "Session window",
                    leftCaption: "\(Int(windowFrac * 100))% elapsed",
                    fraction: windowFrac,
                )
            } else if let iso = payload?.sessionWindowEndsISO {
                Text("Window ends \(iso)")
                    .font(.system(size: 10, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.55))
            }
            if payload?.tiltSignal == true {
                VStack(alignment: .leading, spacing: 8) {
                    Text("Tilt risk — recent trades are much faster than your session baseline. Slow the tape.")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                        .foregroundColor(Color(hex: "#EAB308"))
                        .fixedSize(horizontal: false, vertical: true)
                    Button {
                        scalperStepAwayAck = true
                    } label: {
                        Text("Step away 5 min?")
                            .font(.system(size: 11, weight: .semibold, design: .rounded))
                            .foregroundColor(Color(hex: "#CA8A04"))
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                            .background(Color(hex: "#EAB308").opacity(0.08))
                            .cornerRadius(8)
                            .overlay(
                                RoundedRectangle(cornerRadius: 8)
                                    .stroke(Color(hex: "#EAB308").opacity(0.22), lineWidth: 0.5),
                            )
                    }
                    .buttonStyle(.plain)
                }
                .padding(10)
                .background(Color(hex: "#EAB308").opacity(0.06))
                .cornerRadius(8)
                .overlay(
                    RoundedRectangle(cornerRadius: 8)
                        .stroke(Color(hex: "#EAB308").opacity(0.2), lineWidth: 0.5),
                )
            }
            if let scalpSl = payload?.slPrice, scalpSl > 0 {
                Text("Per-trade SL \(formatQtyPrice(scalpSl))")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(Color.white.opacity(0.78))
                    .accessibilityLabel("Per-trade stop loss \(formatQtyPrice(scalpSl))")
            }
            if (payload?.sessionTradeCount ?? 0) >= 10 {
                Button("Recalibrate emotional scale") {
                    scalperRecalibrateCalm = nil
                    scalperRecalibrateConfidence = nil
                    showRecalibrateSheet = true
                }
                .buttonStyle(.plain)
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(BarDS.Accent.teal)
                .accessibilityHint("Re-rate calm and confidence locally after 10th trade in session")
            }
        }
        .padding(10)
        .background(Color(hex: "#111111").opacity(0.6))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.white.opacity(0.07), lineWidth: 0.5),
        )
    }

    /// Shared stress ramp: <60% teal, 60–90% amber, ≥90% red (#4 live-plan polish).
    private func planStressColor(fraction: Double) -> Color {
        let f = min(1, max(0, fraction))
        if f >= 0.9 { return Color(hex: "#EF4444").opacity(0.92) }
        if f >= 0.6 { return Color(hex: "#CA8A04").opacity(0.92) }
        return BarDS.Accent.teal.opacity(0.92)
    }

    private func scalperMetricTile(title: String, value: String, valueColor: Color) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(.system(size: 8, weight: .semibold, design: .rounded))
                .foregroundColor(Color(hex: "#666666"))
                .tracking(0.4)
            Text(value)
                .font(.system(size: 10, weight: .semibold, design: .monospaced))
                .foregroundColor(valueColor)
                .lineLimit(2)
                .minimumScaleFactor(0.85)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(Color.white.opacity(0.04))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color.white.opacity(0.06), lineWidth: 0.5),
        )
    }

    private func scalperProgressRow(label: String, leftCaption: String, fraction: Double) -> some View {
        let f = min(1, max(0, fraction))
        let fill = planStressColor(fraction: f)
        return VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(label)
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(Color(hex: "#666666"))
                Spacer(minLength: 8)
                Text(leftCaption)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(Color.white.opacity(0.72))
            }
            BarPlanThinProgressBar(fraction: f, fill: fill)
        }
    }

    private var swingDailyCheckInPanel: some View {
        let thesisUnset = swingThesisIntact == nil
        let needsCapitulation = swingThesisIntact == false
        let noteTrim = swingCheckInNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let canSubmit =
            !thesisUnset
            && (!needsCapitulation || (swingCapitulationTag != nil && !noteTrim.isEmpty))
            && !viewModel.barSwingCheckInBusy
        return VStack(alignment: .leading, spacing: 8) {
            Text("DAILY CHECK-IN")
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(Color.white.opacity(0.4))
            Text("Is your thesis still valid?")
                .font(.system(size: 13, weight: .semibold, design: .rounded))
                .foregroundColor(Color.white.opacity(0.92))
            if let quote = payload?.swingThesisQuote?.trimmingCharacters(in: .whitespacesAndNewlines), !quote
                .isEmpty
            {
                Text("“\(quote)”")
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.78))
                    .fixedSize(horizontal: false, vertical: true)
            }
            if let pnl = payload?.weeklyPnL {
                Text("Weekly P&L \(formatSignedDeskMoney(pnl))")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(Color.white.opacity(0.72))
            }
            if let d = payload?.daysInTrade {
                Text("Days in trade: \(d)")
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(Color.white.opacity(0.72))
            }
            HStack(spacing: 6) {
                swingThesisChoiceButton(
                    title: "Yes — thesis intact",
                    selected: swingThesisIntact == true,
                    color: BarDS.Accent.teal,
                ) {
                    swingThesisIntact = true
                    swingCapitulationTag = nil
                }
                swingThesisChoiceButton(
                    title: "Something changed",
                    selected: swingThesisIntact == false,
                    color: Color(hex: "#FF3B30"),
                ) {
                    swingThesisIntact = false
                }
            }
            .accessibilityElement(children: .contain)

            Group {
                if swingThesisIntact == true {
                    Text(
                        "Overnight plan stays valid — keep size and invalidation as declared. No new risk without a fresh declaration.",
                    )
                    .font(.system(size: 10, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.78))
                    .fixedSize(horizontal: false, vertical: true)
                    .padding(10)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(BarDS.Accent.teal.opacity(0.08))
                    .cornerRadius(8)
                    .overlay(
                        RoundedRectangle(cornerRadius: 8)
                            .stroke(BarDS.Accent.teal.opacity(0.22), lineWidth: 0.5),
                    )
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .animation(.easeInOut(duration: 0.15), value: swingThesisIntact)

            Group {
                if swingThesisIntact == false {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("What best describes the capitulation?")
                            .font(.system(size: 10, weight: .semibold, design: .rounded))
                            .foregroundColor(Color.white.opacity(0.65))
                        HStack(spacing: 6) {
                            swingCapitulationChip(title: "Time stop", id: "time_stop")
                            swingCapitulationChip(title: "Thesis broken", id: "thesis_break")
                            swingCapitulationChip(title: "Risk / size", id: "risk_cap")
                        }
                        TextField("Required note — what changed?", text: $swingCheckInNote, axis: .vertical)
                            .lineLimit(3...6)
                            .textFieldStyle(.plain)
                            .font(.system(size: 11, weight: .medium, design: .rounded))
                            .foregroundColor(.white.opacity(0.9))
                            .padding(8)
                            .background(Color.white.opacity(0.06))
                            .cornerRadius(8)
                            .accessibilityLabel("Swing check-in note, required when thesis changed")
                    }
                    .transition(.opacity.combined(with: .move(edge: .top)))
                }
            }
            .animation(.easeInOut(duration: 0.15), value: swingThesisIntact)

            if let err = viewModel.barSwingCheckInLastError, !err.isEmpty {
                Text(err)
                    .font(.system(size: 10, weight: .medium, design: .rounded))
                    .foregroundColor(Color.orange.opacity(0.95))
                    .fixedSize(horizontal: false, vertical: true)
            }

            Button {
                let body = swingCheckInJSONObject(
                    thesisIntact: swingThesisIntact == true,
                    capitulation: swingCapitulationTag,
                    note: noteTrim,
                )
                guard let data = try? JSONSerialization.data(withJSONObject: body) else { return }
                Task { await viewModel.submitSwingCheckIn(body: data) }
            } label: {
                HStack(spacing: 6) {
                    if viewModel.barSwingCheckInBusy {
                        ProgressView().scaleEffect(0.65)
                    }
                    Text(viewModel.barSwingCheckInBusy ? "Submitting…" : "Submit check-in")
                        .font(.system(size: 11, weight: .semibold, design: .rounded))
                }
                .frame(maxWidth: .infinity)
                .padding(.vertical, 10)
                .background(canSubmit ? BarDS.Accent.teal : Color.white.opacity(0.18))
                .foregroundColor(canSubmit ? Color(hex: "#050505") : Color.white.opacity(0.45))
                .cornerRadius(10)
            }
            .buttonStyle(.plain)
            .disabled(!canSubmit)
            .accessibilityHint("Sends swing thesis check-in to TradeAutopsy")

        }
        .padding(10)
        .background(Color(hex: "#F5A524").opacity(0.07))
        .cornerRadius(8)
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(Color(hex: "#EAB308").opacity(0.22), lineWidth: 0.5),
        )
        .onAppear {
            swingThesisIntact = nil
            swingCapitulationTag = nil
            swingCheckInNote = ""
        }
    }

    private func swingThesisChoiceButton(
        title: String,
        selected: Bool,
        color: Color,
        action: @escaping () -> Void,
    ) -> some View {
        Button(action: action) {
            Text(title)
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(selected ? color : Color.white.opacity(0.75))
                .padding(.horizontal, 8)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(selected ? color.opacity(0.14) : Color.white.opacity(0.06)),
                )
                .overlay(
                    RoundedRectangle(cornerRadius: 6)
                        .stroke(selected ? color.opacity(0.35) : Color.white.opacity(0.08), lineWidth: 0.5),
                )
        }
        .buttonStyle(.plain)
    }

    private func swingCapitulationChip(title: String, id: String) -> some View {
        let on = swingCapitulationTag == id
        return Button {
            swingCapitulationTag = id
        } label: {
            Text(title)
                .font(.system(size: 9, weight: .medium, design: .rounded))
                .foregroundColor(on ? BarDS.Accent.teal : Color.white.opacity(0.72))
                .padding(.horizontal, 6)
                .padding(.vertical, 5)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(on ? BarDS.Accent.teal.opacity(0.12) : Color.white.opacity(0.06)),
                )
        }
        .buttonStyle(.plain)
    }

    private func swingCheckInJSONObject(
        thesisIntact: Bool,
        capitulation: String?,
        note: String,
    ) -> [String: Any] {
        var o: [String: Any] = [
            "v": 1,
            "thesis_intact": thesisIntact,
            "at_ms": Int(Date().timeIntervalSince1970 * 1000),
        ]
        if let capitulation, !capitulation.isEmpty {
            o["capitulation"] = capitulation
        }
        if !note.isEmpty {
            o["note"] = note
        }
        let planRaw = payload?.planState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if !planRaw.isEmpty {
            o["plan_state"] = planRaw
        }
        let declFromMatch = payload?.matchedDeclarationId?.trimmingCharacters(in: .whitespacesAndNewlines)
        let declPending = payload?.pendingDeclaration?.id.trimmingCharacters(in: .whitespacesAndNewlines)
        let declId: String? = {
            if let m = declFromMatch, !m.isEmpty { return m }
            if let p = declPending, !p.isEmpty { return p }
            return nil
        }()
        if let declId {
            o["declaration_id"] = declId
        }
        return o
    }

    private func formatQtyPrice(_ v: Double) -> String {
        let f = NumberFormatter()
        f.maximumFractionDigits = 2
        f.minimumFractionDigits = 0
        return f.string(from: NSNumber(value: v)) ?? "\(v)"
    }

    @ViewBuilder
    private var interferenceQuestionBlock: some View {
        BarCard {
            Text("Are you thinking about changing anything?")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
            Text("Warning channel — not a toast stacked on the banner.")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .padding(.top, 2)
                .padding(.bottom, 8)
            HStack(spacing: 6) {
                interferenceChip("No — following plan", id: "no")
                interferenceChip("Maybe — unsure", id: "maybe")
                interferenceChip("Yes — feeling it", id: "yes")
            }
        }
    }

    private func interferenceChip(_ title: String, id: String) -> some View {
        let on = viewModel.barInterferenceChoice == id
        return Button {
            viewModel.applyBarInterferenceTap(id)
        } label: {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(on ? BarDS.Text.primary : BarDS.Text.secondary)
                .padding(.horizontal, 6)
                .padding(.vertical, 10)
                .frame(maxWidth: .infinity)
                .background(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .fill(on ? Color.white.opacity(0.08) : Color.white.opacity(0.03)),
                )
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .stroke(
                            on ? Color.white.opacity(0.20) : BarDS.Border.input,
                            lineWidth: BarDS.borderThin
                        ),
                )
        }
        .buttonStyle(.plain)
    }

    @ViewBuilder
    private var protectiveSlHintRow: some View {
        let chrome = BarProtectiveSlPlanChrome.presentation(
            slStatus: payload?.slStatus,
            slPrice: payload?.slPrice,
            slFailureReason: payload?.slFailureReason,
            formattedPrice: payload?.slPrice.map { formatDeskWhole($0) },
        )
        if let rejected = chrome.rejectedText {
            HStack(spacing: 6) {
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(.red)
                    .font(.system(size: 12))
                Text(rejected)
                    .font(.system(size: 12, weight: .medium))
                    .foregroundStyle(.red)
                    .lineLimit(2)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .background(Color.red.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: 6))
        }
        if let status = chrome.statusText {
            VStack(alignment: .leading, spacing: 6) {
                Text(status)
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(
                        chrome.showsCancelSlButton ? Color.green.opacity(0.8) : Color.orange.opacity(0.85)
                    )
                    .fixedSize(horizontal: false, vertical: true)
                if chrome.showsCancelSlButton {
                    Button {
                        cancelSlWebBarAlert = true
                    } label: {
                        Text("Cancel SL (confirm)…")
                            .font(.system(size: 11, weight: .semibold, design: .rounded))
                    }
                    .buttonStyle(.plain)
                    .foregroundColor(BarDS.Accent.teal)
                    .disabled(viewModel.barProtectiveBusy)
                    .accessibilityHint("Opens Harness when a broker cancel is required")
                }
            }
        }
    }

    private var planStateBanner: some View {
        let tier = bannerTier(hasOpenPositions: viewModel.hasOpenPositions)
        return bannerBlock(for: tier)
    }

    private enum PlanBannerTier: Equatable {
        case syncExpired
        case syncStale
        case thesisUnknown
        case serverPlan(state: String, sentence: String, terminal: Bool)
    }

    private func bannerTier(hasOpenPositions: Bool) -> PlanBannerTier {
        let sync = payload?.syncState ?? ""
        if sync == "EXPIRED" { return .syncExpired }
        if sync == "STALE" { return .syncStale }

        let trimmedPlan = payload?.planState?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if hasOpenPositions && trimmedPlan.isEmpty {
            return .thesisUnknown
        }

        let upper = trimmedPlan.isEmpty ? "GREEN" : trimmedPlan.uppercased()
        let sentence = resolvedPlanBannerSentence(planUpper: upper, apiSentence: payload?.primarySentence)
        let terminal = payload?.isRedTerminal ?? false
        return .serverPlan(state: upper, sentence: sentence, terminal: terminal)
    }

    /// Ignore API `primary_sentence` when it matches another plan state's default copy.
    private func resolvedPlanBannerSentence(planUpper: String, apiSentence: String?) -> String {
        let fallback = fallbackSentence(planUpper: planUpper)
        guard let raw = apiSentence?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            return fallback
        }
        if apiSentenceContradictsPlanState(sentence: raw, planUpper: planUpper) {
            return fallback
        }
        return raw
    }

    private func apiSentenceContradictsPlanState(sentence: String, planUpper: String) -> Bool {
        let current = planUpper.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        for state in ["GREEN", "AMBER", "RED"] where state != current {
            if sentence == fallbackSentence(planUpper: state) {
                return true
            }
        }
        return false
    }

    private func fallbackSentence(planUpper: String) -> String {
        switch planUpper {
        case "GREEN": return "Plan intact. Nothing to do."
        case "AMBER": return "Watch conditions — review your plan."
        case "RED": return "Thesis invalid — consider exit."
        default: return "Review your plan in Harness."
        }
    }

    @ViewBuilder
    private func bannerBlock(for tier: PlanBannerTier) -> some View {
        switch tier {
        case .syncExpired:
            syncBanner(headline: "Sync expired",
                       bodyText: "Broker lane trust window ended — reconcile in Harness.",
                       stripe: "RED")
        case .syncStale:
            syncBanner(headline: "Sync stale",
                       bodyText: "Last broker sync is old — proceed with caution.",
                       stripe: "AMBER")
        case .thesisUnknown:
            syncBanner(headline: "Thesis unknown",
                       bodyText: "Open positions, no server plan — reconcile in Harness.",
                       stripe: "AMBER")
        case let .serverPlan(state, sentence, terminal):
            serverPlanBanner(state: state, sentence: sentence, terminal: terminal)
        }
    }

    private func syncBanner(headline: String, bodyText: String, stripe: String)
        -> some View
    {
        let kind: BarPlanBannerKind = stripe == "RED" ? .broken : .watch
        return VStack(alignment: .leading, spacing: 8) {
            BarStateBanner(kind: kind, label: headline, sentence: bodyText)
        }
    }

    private func serverPlanBanner(state: String, sentence: String, terminal: Bool) -> some View {
        let suffix = entryMinutesSuffix
        let bodyText = suffix != nil ? "\(sentence) — \(suffix!)" : sentence
        let upper = state.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let kind = mapPlanBannerKind(upper)
        return VStack(alignment: .leading, spacing: 8) {
            BarStateBanner(kind: kind, label: stateLabel(state), sentence: bodyText)
            if terminal {
                Text("Terminal RED — review exit discipline.")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.red.opacity(0.9))
            }
            if upper == "RED" {
                HStack {
                    Spacer(minLength: 0)
                    Button("Exit trade →") {
                        showExitGateSheet = true
                    }
                    .buttonStyle(.plain)
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .padding(.horizontal, 10)
                    .padding(.vertical, 6)
                    .background(BarDS.Accent.red.opacity(0.85))
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                }
            }
        }
    }

    private func mapPlanBannerKind(_ planUpper: String) -> BarPlanBannerKind {
        switch planUpper {
        case "GREEN": return .intact
        case "AMBER": return .watch
        default: return .broken
        }
    }

    private var entryMinutesSuffix: String? {
        guard let iso = payload?.entryTimeISO else { return nil }
        let frac = ISO8601DateFormatter()
        frac.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let coarse = ISO8601DateFormatter()
        coarse.formatOptions = [.withInternetDateTime]
        let date = frac.date(from: iso) ?? coarse.date(from: iso)
        guard let date else { return nil }
        let m = max(0, Int(Date().timeIntervalSince(date) / 60))
        return "\(m) min"
    }

    private func compositeRiskLine(_ composite: CompositeRisk, showStaleSuffix: Bool) -> some View {
        let pct = composite.budgetPct
        let pctInt = Int(min(1, max(0, pct)) * 100)
        let left = "\(formatDeskWhole(composite.worstCase)) at risk across open positions"
        var right = "\(pctInt)% of daily limit"
        if showStaleSuffix {
            right += " · may be stale"
        }
        let rightColor = compositeBudgetTextColor(budgetPct: pct, stateFallback: composite.state)
        return BarCompositeLine(leftText: left, rightText: right, rightColor: rightColor)
    }

    /// Teal below 60% of daily budget, amber 60–90%, red at or above 90% (non-finite → server `state`).
    private func compositeBudgetTextColor(budgetPct: Double, stateFallback: String) -> Color {
        guard budgetPct.isFinite else { return compositeTextColor(stateFallback) }
        let p = min(1, max(0, budgetPct))
        if p >= 0.9 { return BarDS.Accent.red }
        if p >= 0.6 { return BarDS.Accent.amber }
        return BarDS.Accent.teal
    }

    private func interventionBanner(_ intervention: ActiveIntervention, isPrimary: Bool) -> some View {
        let chrome = BarInterventionCardSpec.chrome(interventionType: intervention.interventionType, isPrimary: isPrimary)
        let accessory = isPrimary ? BarInterventionCardSpec.primaryAccessory(interventionType: intervention.interventionType) : .none
        let iconTint = Color(hex: chrome.borderColorHex).opacity(isPrimary ? 0.92 : 0.88)
        let needsGate =
            isPrimary && BarInterventionCardSpec.requiresMandatoryRead(interventionType: intervention.interventionType)
        let iid = intervention.id
        let anchor = mandatoryReadStartedAtByInterventionId[iid]
        let rem = BarInterventionMandatoryReadGate.secondsRemaining(
            anchorStartedAt: anchor,
            now: interventionCountdownTick,
            minimumSeconds: BarInterventionMandatoryReadGate.defaultSeconds,
        )
        let gateLocksUI = needsGate && rem > 0
        return VStack(alignment: .leading, spacing: 8) {
            HStack(alignment: .top, spacing: 8) {
                HStack(spacing: 5) {
                    if chrome.showNakedPulseDot {
                        NakedWindowInterventionDot()
                    }
                    Image(systemName: "exclamationmark.triangle.fill")
                        .font(.system(size: 11))
                        .foregroundColor(iconTint)
                }
                VStack(alignment: .leading, spacing: 2) {
                    Text(intervention.interventionType.replacingOccurrences(of: "_", with: " ").uppercased())
                        .font(.system(size: 10, weight: .bold, design: .rounded))
                        .foregroundColor(Color.white.opacity(isPrimary ? 0.62 : 0.5))
                    Text(intervention.primaryMessage)
                        .font(.system(size: 12, weight: .medium, design: .rounded))
                        .foregroundColor(.white.opacity(0.9))
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
            }
            if needsGate {
                Text(
                    gateLocksUI
                        ? "Minimum read — \(rem)s before override actions unlock."
                        : "Read window complete — proceed only if you accept the risk.",
                )
                .font(.system(size: 10, weight: .semibold, design: .rounded))
                .foregroundColor(gateLocksUI ? Color.orange.opacity(0.9) : Color.green.opacity(0.82))
                .accessibilityLabel("Intervention read gate")
            }
            interventionPrimaryAccessory(accessory, interactionsEnabled: !gateLocksUI)
        }
        .padding(10)
        .background(Color(hex: chrome.backgroundHex).opacity(chrome.backgroundOpacity))
        .cornerRadius(8)
        .overlay {
            if chrome.borderWidth > 0 {
                RoundedRectangle(cornerRadius: 8)
                    .strokeBorder(Color(hex: chrome.borderColorHex).opacity(0.88), lineWidth: chrome.borderWidth)
            }
        }
        .onAppear {
            if needsGate, mandatoryReadStartedAtByInterventionId[iid] == nil {
                mandatoryReadStartedAtByInterventionId[iid] = Date()
            }
        }
    }

    @ViewBuilder
    private func interventionPrimaryAccessory(_ acc: BarInterventionCardSpec.PrimaryAccessory, interactionsEnabled: Bool)
        -> some View
    {
        switch acc {
        case .none:
            EmptyView()
        case .manageInWebBar:
            if interactionsEnabled {
                Text(
                    "Resolve this intervention through the daemon or broker — the Notch does not open the web dashboard.",
                )
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(Color.white.opacity(0.55))
                .fixedSize(horizontal: false, vertical: true)
            } else {
                EmptyView()
            }
        }
    }

    // MARK: - Exit trade (cancel declaration)

    private var exitTradeSection: some View {
        VStack(alignment: .leading, spacing: 8) {
            if viewModel.cancelDeclStep == 0 {
                Button {
                    viewModel.cancelDeclStep = 1
                } label: {
                    Text("Exit trade")
                        .font(.system(size: 11, weight: .medium, design: .rounded))
                        .foregroundColor(.orange)
                }
                .buttonStyle(.plain)
            } else if viewModel.cancelDeclStep == 1 {
                HStack {
                    Button("Cancel") {
                        viewModel.resetCancelDecl()
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 11, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.45))
                    Spacer()
                    Text("Cancel declaration?")
                        .font(.system(size: 12))
                        .foregroundStyle(.secondary)
                    Spacer()
                    Button("Confirm") {
                        let declId = viewModel.barCancelDeclarationId ?? ""
                        Task {
                            await viewModel.submitCancelDeclaration(
                                declarationId: declId,
                                reasonChip: "manual_exit",
                            )
                        }
                    }
                    .buttonStyle(.plain)
                    .font(.system(size: 11, weight: .semibold, design: .rounded))
                    .foregroundColor(.red)
                }
            } else if viewModel.cancelDeclStep == 2 {
                Text("Cancelling...")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
            }
            if let err = viewModel.cancelDeclError {
                Text(err)
                    .font(.system(size: 11))
                    .foregroundStyle(.red)
            }
        }
    }

    // MARK: - Kill

    private var planKillSection: some View {
        let chrome = planKillChrome
        return VStack(alignment: .leading, spacing: 10) {
            if chrome.showsKillButton {
                Button {
                    viewModel.presentPlanKillWarning()
                } label: {
                    Text(chrome.killButtonTitle)
                        .font(.system(size: 11, weight: .bold, design: .rounded))
                        .foregroundColor(Color(hex: "#FF3B30"))
                }
                .buttonStyle(.plain)
                .accessibilityLabel(chrome.killButtonTitle)
                .accessibilityHint("Shows a warning before locking the desk")
            }
            if chrome.showsWarningCard {
                VStack(alignment: .leading, spacing: 8) {
                    Text(chrome.warningTitle)
                        .font(.system(size: 12, weight: .bold, design: .rounded))
                        .foregroundColor(.white)
                    Text(chrome.warningBody)
                        .font(.system(size: 11, weight: .medium, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.7))
                        .fixedSize(horizontal: false, vertical: true)
                    HStack(spacing: 12) {
                        Button(chrome.confirmTitle) {
                            Task { await viewModel.confirmPlanKill() }
                        }
                        .buttonStyle(.plain)
                        .font(.system(size: 11, weight: .bold, design: .rounded))
                        .foregroundColor(Color(hex: "#FF3B30"))
                        Button(chrome.cancelTitle) {
                            viewModel.cancelPlanKillWarning()
                        }
                        .buttonStyle(.plain)
                        .font(.system(size: 11, weight: .medium, design: .rounded))
                        .foregroundColor(Color.white.opacity(0.45))
                    }
                }
                .padding(12)
                .background(Color(hex: "#FF3B30").opacity(0.08))
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .accessibilityElement(children: .contain)
                .accessibilityLabel(chrome.warningTitle)
            }
        }
        .padding(.top, 4)
    }

    // MARK: - Helpers

    private func stateColor(_ state: String) -> Color {
        switch state.uppercased() {
        case "GREEN": Color.green
        case "AMBER": Color.orange
        case "RED": Color.red
        default: Color.green
        }
    }

    private func stateLabel(_ state: String) -> String {
        switch state.uppercased() {
        case "GREEN": "Plan intact"
        case "AMBER": "Watch condition"
        case "RED": "Plan broken"
        default: "Plan intact"
        }
    }

    private func stateBannerBackground(_ state: String) -> Color {
        switch state.uppercased() {
        case "GREEN": Color.green.opacity(0.12)
        case "AMBER": Color.orange.opacity(0.12)
        case "RED": Color.red.opacity(0.12)
        default: Color.green.opacity(0.08)
        }
    }

    private func compositeTextColor(_ state: String) -> Color {
        switch state.uppercased() {
        case "RED": BarDS.Accent.red
        case "AMBER": BarDS.Accent.amber
        default: BarDS.Text.hint
        }
    }

}

// MARK: - Naked-window pulse (#122)

private struct BarPlanThinProgressBar: View {
    let fraction: Double
    let fill: Color

    var body: some View {
        GeometryReader { geo in
            let w = max(0, min(1, fraction)) * geo.size.width
            ZStack(alignment: .leading) {
                Capsule()
                    .fill(Color.white.opacity(0.05))
                    .frame(height: 3)
                Capsule()
                    .fill(fill)
                    .frame(width: max(2, w), height: 3)
            }
        }
        .frame(height: 3)
    }
}

private struct NakedWindowInterventionDot: View {
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.periodic(from: .now, by: reduceMotion ? 3600 : 0.55)) { timeline in
            let on = Int(timeline.date.timeIntervalSinceReferenceDate / 0.55) % 2 == 0
            Circle()
                .fill(Color(hex: "#F5A524"))
                .opacity(reduceMotion ? 1 : (on ? 1 : 0.48))
                .frame(width: 6, height: 6)
        }
    }
}

// MARK: - Status dot pulse

private struct PlanStatusDot: View {
    let color: Color
    let animate: Bool
    let fastPulse: Bool
    @State private var pulsePhase: CGFloat = 0
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        Circle()
            .fill(color)
            .opacity(dotOpacity)
            .frame(width: 8, height: 8)
            .onAppear { startPulseIfNeeded() }
            .onChange(of: animate) { _, _ in startPulseIfNeeded() }
            .onChange(of: fastPulse) { _, _ in startPulseIfNeeded() }
    }

    private var dotOpacity: Double {
        guard !reduceMotion, animate || fastPulse else { return 1.0 }
        return 0.45 + 0.55 * Double(pulsePhase)
    }

    private func startPulseIfNeeded() {
        guard !reduceMotion else { return }
        if fastPulse {
            withAnimation(.easeInOut(duration: 0.38).repeatForever(autoreverses: true)) {
                pulsePhase = 1
            }
        } else if animate {
            withAnimation(.easeInOut(duration: 0.95).repeatForever(autoreverses: true)) {
                pulsePhase = 1
            }
        } else {
            pulsePhase = 0
        }
    }
}

// MARK: - Preview

private extension BarLiveStateResponse {
    static func preview(plan: String?, sentence: String?) -> BarLiveStateResponse {
        BarLiveStateResponse(
            planState: plan,
            primarySentence: sentence,
            triggerType: nil,
            isRedTerminal: plan == "RED",
            composite: CompositeRisk(worstCase: 4200, budgetPct: 0.62, state: "amber", breakdown: []),
            activeInterventions: [],
            syncState: "GREEN",
            lastSyncAt: nil,
            declarationSubmitBlocked: nil,
            pendingDeclaration: nil,
        )
    }
}

@MainActor
struct BarPlanStateView_Previews: PreviewProvider {
    static var previews: some View {
        Group {
            previewHost(
                vm: sampleVM(
                    BarLiveStateResponse.preview(plan: "GREEN", sentence: nil),
                ),
                title: "Plan GREEN",
            )
            previewHost(
                vm: sampleVM(
                    BarLiveStateResponse(
                        planState: "AMBER",
                        primarySentence: "Size extended beyond declared risk budget.",
                        triggerType: "sizing",
                        isRedTerminal: false,
                        composite: CompositeRisk(
                            worstCase: 9123,
                            budgetPct: 0.71,
                            state: "AMBER",
                            breakdown: [],
                        ),
                        activeInterventions: [
                            ActiveIntervention(
                                interventionType: "soft_block",
                                primaryMessage: "Walk away from the ladder for 5 minutes.",
                                expiresAt: nil,
                            ),
                        ],
                        syncState: "AMBER",
                        lastSyncAt: nil,
                        declarationSubmitBlocked: nil,
                        pendingDeclaration: nil,
                    ),
                ),
                title: "Plan AMBER + intervention",
            )
        }
    }

    private static func sampleVM(_ state: BarLiveStateResponse?) -> NotchViewModel {
        let vm = NotchViewModel()
        vm.barLiveState = state
        return vm
    }

    private static func previewHost(vm: NotchViewModel, title: String) -> some View {
        BarPlanStateView(viewModel: vm)
            .previewDisplayName(title)
            .frame(width: 360, height: 320)
            .background(Color(hex: "#050505"))
    }
}
