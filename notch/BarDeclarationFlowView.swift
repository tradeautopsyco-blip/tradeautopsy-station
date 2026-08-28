import SwiftUI

/// Pre-trade declaration — single scrollable page (#121 / unified reference mockups 3–4).
struct BarDeclarationFlowView: View {
    @ObservedObject var viewModel: NotchViewModel

    /// Scale A (calm, 1 best) — `0` until user taps (mirrors `declEmotionalCalm`).
    /// Scale B (confidence, 5 best) — `0` until user taps (`declEmotionalConfidence`).
    @State private var sideBuy: Bool = true
    @State private var quantityText: String = ""
    @State private var stopLossText: String = ""
    @State private var kind: DeclarationKind = .intraday
    @State private var scalperSessionId: String = ""
    @State private var targetPriceText: String = ""
    @State private var declarationStartedAt: Date? = nil
    @State private var recapClock: Date = Date()

    /// HTML spec order — six intraday setup chips only.
    private static let intradaySetupChipsSpecOrder: [BarIntradaySetupChip] = [
        .breakout, .pullback, .reversal, .gapFill, .meanReversion, .supportBounce,
    ]

    private enum DeclarationKind: String, CaseIterable, Identifiable {
        case intraday, swing, positional, scalper_session, pre_market
        var id: String { rawValue }
        var label: String {
            switch self {
            case .intraday: return "Intraday"
            case .swing: return "Swing"
            case .positional: return "Positional"
            case .scalper_session: return "Scalper session"
            case .pre_market: return "Pre-market"
            }
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
                BarUndeclaredPositionBanner(position: viewModel.barLiveState?.undeclaredPosition)

                BarDeclarationSubmitBlockedBanner(
                    blocked: viewModel.barLiveState?.blocksDeclarationSubmit == true,
                    activeInterventions: viewModel.barLiveState?.activeInterventions ?? [],
                    clearKillSwitchBusy: viewModel.barStopMeClearBusy,
                    onClearKillSwitch: {
                        Task { await viewModel.clearBarStopMeKillSwitch() }
                    },
                )

                if let e = viewModel.barDeclarationLastError, !e.isEmpty {
                    Text(e)
                        .font(BarDS.bodyFont(10, weight: .medium))
                        .foregroundColor(BarDS.Accent.red)
                }

                if let w = viewModel.barDeclarationConfirmWarning, !w.isEmpty {
                    Text(w)
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundColor(BarDS.Accent.amber)
                        .fixedSize(horizontal: false, vertical: true)
                }

                archetypeTabRow

                if BarOptionsDeclareSurface.usesThreeZone(for: viewModel.declareAssetClass) {
                    BarOptionsDeclareView(
                        viewModel: viewModel,
                        sideBuy: $sideBuy,
                        stopLossText: $stopLossText,
                        targetPriceText: $targetPriceText,
                        submitReady: submitReadiness.ready,
                        submitHint: submitReadiness.hint,
                        onConfirm: { Task { await submit() } },
                    )
                } else {
                    BarSectionLabel(text: "State check")
                    stateCheckCard

                    BarSectionLabel(text: "Numbers")
                    riskNumbersCard

                    BarSectionLabel(text: "Setup & invalidation")
                    setupAndInvalidationCard

                    BarSectionLabel(text: "Review")
                    BarToggleRow(
                        label: "Auto-place stop loss on fill",
                        sub: "Pre-authorized — placed within 500ms of broker fill",
                        isOn: $viewModel.declProtectiveSLConsent,
                    )
                    .disabled(viewModel.barLiveState?.blocksDeclarationSubmit == true)

                    BarBigButton(
                        label: viewModel.barDeclarationBusy ? "Submitting…" : "Confirm — enter trade →",
                        style: .primary,
                    ) {
                        Task { await submit() }
                    }
                    .disabled(!submitReadiness.ready || viewModel.barDeclarationBusy)
                    .opacity(submitReadiness.ready && !viewModel.barDeclarationBusy ? 1 : 0.3)

                    if !submitReadiness.ready,
                       viewModel.barDeclarationLastError == nil,
                       let hint = submitReadiness.hint,
                       !viewModel.barDeclarationBusy {
                        Text(hint)
                            .font(BarDS.bodyFont(10, weight: .medium))
                            .foregroundColor(BarDS.Text.hint)
                            .fixedSize(horizontal: false, vertical: true)
                    }

                    Text("Completed in \(elapsedLiveSeconds)s")
                        .font(BarDS.bodyFont(10, weight: .regular))
                        .foregroundColor(BarDS.Text.labels)
                        .frame(maxWidth: .infinity, alignment: .trailing)
                }
            }
        .onAppear {
            if declarationStartedAt == nil {
                declarationStartedAt = Date()
                viewModel.setIfChanged(\.declInvalidationType, "")
                viewModel.setIfChanged(\.declInvalidationCondition, "")
            }
            applyBrokerDeclarationPrefillIfNeeded()
        }
        .onChange(of: viewModel.showingDeclarationForm) { _, showing in
            if !showing {
                viewModel.dismissSymbolSuggestions()
            }
        }
        .onReceive(NotchOneSecondClock.publisher) { recapClock = $0 }
        .onChange(of: viewModel.declEmotionalCalm) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declEmotionalConfidence) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: stopLossText) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: quantityText) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declLots) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.barDeclarationSymbol) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declSetupType) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declInvalidationType) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declInvalidationCondition) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
        .onChange(of: viewModel.declProtectiveSLConsent) { _, _ in viewModel.clearDeclarationErrorIfNeeded() }
    }

    private var submitReadiness: (ready: Bool, hint: String?) {
        BarIntradayDeclareValidator.submitReadiness(
            BarIntradayDeclarationSubmitInput(
                blocksDeclarationSubmit: viewModel.barLiveState?.blocksDeclarationSubmit == true,
                protectiveSlConsent: viewModel.declProtectiveSLConsent,
                calm: viewModel.declEmotionalCalm,
                confidence: viewModel.declEmotionalConfidence,
                stopLossText: stopLossText,
                symbolRaw: viewModel.barDeclarationSymbol,
                quantityText: quantityText,
                setupType: viewModel.declSetupType,
                invalidationTypeRaw: viewModel.declInvalidationType,
                invalidationCondition: viewModel.declInvalidationCondition,
                declarationKindWire: declarationKindWire(for: viewModel.activeArchetype),
                scalperSessionId: scalperSessionId,
                lotsText: viewModel.declLots,
                isOptions: viewModel.declareAssetClass == .options,
                optionLegCount: viewModel.optionLegs.count,
                maxPlannedLossText: viewModel.declMaxPlannedLossText,
            ),
        )
    }

    private var archetypeTabRow: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 5) {
                ForEach(BarDeclareAssetClass.allCases) { asset in
                    BarTab(label: asset.label, active: viewModel.declareAssetClass == asset) {
                        viewModel.declareAssetClass = asset
                    }
                }
            }
            HStack(spacing: 5) {
                BarTab(label: "Intraday", active: viewModel.activeArchetype == .intraday) {
                    selectArchetype(.intraday)
                }
                BarTab(label: "Scalper", active: viewModel.activeArchetype == .scalper) {
                    selectArchetype(.scalper)
                }
                BarTab(label: "Swing", active: viewModel.activeArchetype == .swing) {
                    selectArchetype(.swing)
                }
            }
        }
        .padding(.bottom, 4)
    }

    private var stateCheckCard: some View {
        BarCard {
            Text("How are you feeling?")
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .padding(.bottom, 3)
            Text("Two readings. Answer honestly — it changes what the system flags.")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 10)

            BarSectionLabel(text: "Psychological calm — 1 is best")
            feelRow(
                value: Binding(
                    get: { viewModel.declEmotionalCalm },
                    set: { viewModel.declEmotionalCalm = $0 },
                ),
                labels: [1: "Calm", 2: "Focused", 3: "Tense", 4: "Anxious", 5: "Angry"],
                feel: calmFeelState,
            )

            BarSectionLabel(text: "Confidence — 5 is best")
            feelRow(
                value: Binding(
                    get: { viewModel.declEmotionalConfidence },
                    set: { viewModel.declEmotionalConfidence = $0 },
                ),
                labels: [1: "Low", 2: "Flat", 3: "Neutral", 4: "Good", 5: "Sharp"],
                feel: confidenceFeelState,
            )

            Group {
                if viewModel.declEmotionalCalm >= 4 {
                    stateWarningBox(
                        text: "State \(viewModel.declEmotionalCalm) — position size halved. Trade will be flagged.",
                        kind: .red,
                    )
                    .transition(.opacity)
                } else if viewModel.declEmotionalCalm == 3 {
                    stateWarningBox(
                        text: "State 3 — trade with heightened awareness.",
                        kind: .amber,
                    )
                    .transition(.opacity)
                }
            }
            .animation(.easeInOut(duration: 0.15), value: viewModel.declEmotionalCalm)
        }
    }

    private var riskNumbersCard: some View {
        BarCard {
            BarInputField(placeholder: "Entry price", text: $viewModel.declEntryPrice)
            if let ltpErr = viewModel.barLtpFetchError, !ltpErr.isEmpty {
                Text(ltpErr)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
            }
            deskGlanceStrip
            BarInputField(placeholder: "Stop loss — exact price", text: $stopLossText)
            BarInputField(placeholder: "Target price", text: $targetPriceText)
            symbolAutocompleteField
            if viewModel.declareAssetClass == .options {
                optionsShellFields
            } else {
                horizonRow
            }
            HStack(spacing: 5) {
                pretradeSidePill(title: "BUY", selected: sideBuy) { sideBuy = true }
                pretradeSidePill(title: "SELL", selected: !sideBuy) { sideBuy = false }
            }
            .padding(.bottom, 4)
            if viewModel.declareAssetClass != .options {
                BarInputField(placeholder: "Quantity", text: $quantityText)
            }

            if viewModel.declareAssetClass == .options {
                BarPlanLadderView(rung1: planRung1, declaredMaxLossINR: planMaxLossINR)
            } else if let loss = planMaxLossINR {
                Text(String(format: "MAX LOSS (plan): ₹%.0f", loss))
                    .font(BarDS.monoFont(10, weight: .semibold))
                    .foregroundColor(BarDS.Text.hint)
            }

            riskRewardSpecRow
        }
    }

    private var setupAndInvalidationCard: some View {
        BarCard {
            BarSectionLabel(text: "Setup type")
            BarFlowLayout(spacing: 5, rowSpacing: 5) {
                ForEach(Self.intradaySetupChipsSpecOrder, id: \.rawValue) { chip in
                    BarChip(label: chip.rawValue, selected: viewModel.declSetupType == chip.rawValue) {
                        viewModel.declSetupType = chip.rawValue
                    }
                }
            }

            BarSectionLabel(text: "Invalidation type")
            invalidationKindSpecRow

            Group {
                if selectedInvalidationKind != nil {
                    invalidationTextAreaBlock
                }
            }
            .animation(.easeInOut(duration: 0.15), value: selectedInvalidationKind)
        }
    }

    private var elapsedLiveSeconds: Int {
        let end = recapClock
        let start = declarationStartedAt ?? end
        return max(0, Int(end.timeIntervalSince(start).rounded(.down)))
    }

    private var selectedInvalidationKind: BarInvalidationKind? {
        let t = viewModel.declInvalidationType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return BarInvalidationKind(rawValue: t)
    }

    private var riskRewardSpecRow: some View {
        HStack {
            Text("Risk : Reward")
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.hint)
            Spacer(minLength: 8)
            Text(rrSpecValueText)
                .font(BarDS.monoFont(12, weight: .medium))
                .foregroundColor(rrSpecValueColor)
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 10)
        .background(Color.white.opacity(0.03))
        .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 6, style: .continuous)
                .stroke(Color.white.opacity(0.08), lineWidth: 0.5),
        )
        .padding(.bottom, 10)
    }

    private var rrSpecValueText: String {
        guard let r = rrComputedRatio else { return "—" }
        return String(format: "1 : %.1f", r)
    }

    private var rrSpecValueColor: Color {
        guard let r = rrComputedRatio else {
            return BarDS.Text.primary.opacity(0.9)
        }
        return r >= 2.0 ? BarDS.Accent.green : BarDS.Text.primary
    }

    private var rrComputedRatio: Double? {
        guard let e = Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespaces)),
              let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces))
        else { return nil }
        let tStr = targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !tStr.isEmpty, let t = Double(tStr) else { return nil }
        let risk = abs(e - sl)
        let reward = abs(t - e)
        guard risk > 0 else { return nil }
        return reward / risk
    }

    private var invalidationTextAreaBlock: some View {
        let kind = selectedInvalidationKind
        return VStack(alignment: .leading, spacing: 6) {
            ZStack(alignment: .topLeading) {
                if viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text(invalidationPlaceholder(kind))
                        .font(BarDS.bodyFont(12, weight: .regular))
                        .foregroundColor(BarDS.Text.secondary)
                        .padding(.top, 10)
                        .padding(.leading, 6)
                }
                TextEditor(text: $viewModel.declInvalidationCondition)
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .scrollContentBackground(.hidden)
                    .frame(minHeight: 60)
                    .padding(.vertical, 4)
                    .padding(.horizontal, 4)
            }
            .padding(.vertical, 9)
            .padding(.horizontal, 10)
            .background(Color.white.opacity(0.02))
            .clipShape(RoundedRectangle(cornerRadius: 6, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .stroke(Color.white.opacity(0.08), lineWidth: 0.5),
            )

            Text(invalidationExampleHint(kind))
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 10)
        }
        .transition(.opacity.combined(with: .move(edge: .top)))
    }

    private func invalidationPlaceholder(_ kind: BarInvalidationKind?) -> String {
        switch kind {
        case .time:
            return "If no move by [time], momentum thesis expires..."
        case .behaviour:
            return "If price does this on the chart..."
        case .context:
            return "If this external condition changes..."
        case .none:
            return ""
        }
    }

    private func invalidationExampleHint(_ kind: BarInvalidationKind?) -> String {
        switch kind {
        case .time:
            return "Example: \"By 10:30 AM — morning momentum window closes.\""
        case .behaviour:
            return "Example: \"If price closes back inside the range on a 5-min candle.\""
        case .context:
            return "Example: \"If NIFTY breaks 21,800, my thesis is gone.\""
        case .none:
            return ""
        }
    }

    private var invalidationKindSpecRow: some View {
        HStack(spacing: 6) {
            ForEach(BarInvalidationKind.allCases) { k in
                let on = selectedInvalidationKind == k
                Button {
                    viewModel.declInvalidationType = k.rawValue
                } label: {
                    VStack(spacing: 4) {
                        invalidationIcon(for: k)
                            .font(.system(size: 16))
                            .foregroundColor(on ? BarDS.Accent.blue : BarDS.Text.secondary)
                        Text(k.chipTitle)
                            .font(BarDS.bodyFont(11, weight: .regular))
                            .foregroundColor(on ? BarDS.Accent.blue : BarDS.Text.secondary)
                            .multilineTextAlignment(.center)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 9)
                    .padding(.horizontal, 6)
                    .background(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .fill(on ? Color(red: 59 / 255, green: 130 / 255, blue: 246 / 255).opacity(0.08) : Color.white.opacity(0.03)),
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .stroke(
                                on ? Color(red: 59 / 255, green: 130 / 255, blue: 246 / 255).opacity(0.25) : Color.white.opacity(0.10),
                                lineWidth: 0.5,
                            ),
                    )
                }
                .buttonStyle(.plain)
            }
        }
    }

    @ViewBuilder
    private func invalidationIcon(for k: BarInvalidationKind) -> some View {
        switch k {
        case .time:
            Image(systemName: "clock")
        case .behaviour:
            Image(systemName: "chart.xyaxis.line")
        case .context:
            Image(systemName: "globe")
        }
    }

    private func submit() async {
        viewModel.barDeclarationLastError = nil
        guard let data = buildJsonBody() else {
            viewModel.barDeclarationLastError = submitReadiness.hint ?? "Fix trade fields before submitting."
            return
        }
        await viewModel.submitBarDeclaration(body: data)
    }

    private func buildJsonBody() -> Data? {
        let isOptions = viewModel.declareAssetClass == .options
        guard viewModel.barLiveState?.blocksDeclarationSubmit != true else { return nil }
        if !isOptions {
            guard viewModel.declProtectiveSLConsent else { return nil }
        }
        guard (1 ... 5).contains(viewModel.declEmotionalCalm),
              (1 ... 5).contains(viewModel.declEmotionalConfidence) else { return nil }
        guard BarIntradayDeclareValidator.stickyPreTradeConfirmEnabled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            stopLossText: stopLossText,
        ) else { return nil }
        guard let sym = BarBrokerTicker.normalize(raw: viewModel.barDeclarationSymbol) else { return nil }
        let lotsParsed = Int(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines))
        let qty: Double
        if isOptions {
            guard !viewModel.optionLegs.isEmpty else { return nil }
            qty = Double(viewModel.optionLegs[0].lots)
        } else if let parsedQty = Double(quantityText.trimmingCharacters(in: .whitespaces)), parsedQty > 0 {
            qty = parsedQty
        } else {
            return nil
        }
        guard let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)) else { return nil }
        if !isOptions {
            guard !viewModel.declSetupType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
            guard selectedInvalidationKind != nil else { return nil }
        }
        let invTrim = viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !invTrim.isEmpty else { return nil }
        let wireKind = declarationKindWire(for: viewModel.activeArchetype)
        if wireKind == "scalper_session",
           scalperSessionId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return nil
        }

        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        let entryTrim = viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)
        let targetTrim = targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines)
        let includeEntryTarget = isOptions || wireKind == "swing" || wireKind == "positional"
        let entryOpt = includeEntryTarget && !entryTrim.isEmpty ? Double(entryTrim) : nil
        let targetOpt = !targetTrim.isEmpty ? Double(targetTrim) : nil
        let typedMax = Double(viewModel.declMaxPlannedLossText.trimmingCharacters(in: .whitespacesAndNewlines))
        let maxLoss = isOptions ? typedMax : planMaxLossINR
        if isOptions, typedMax == nil { return nil }

        let first = viewModel.optionLegs.first
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: first?.underlying ?? sym,
            sideBuy: first?.sideBuy ?? sideBuy,
            quantity: qty,
            stopLoss: sl,
            declarationKind: wireKind,
            moodStress: Double(calm),
            moodImpulse: Double(conf),
            invalidationNote: invTrim,
            protectiveSlConsent: isOptions ? true : viewModel.declProtectiveSLConsent,
            entryPrice: entryOpt,
            targetPrice: targetOpt,
            scalperSessionId: wireKind == "scalper_session" ? scalperSessionId : nil,
            isSessionLevel: wireKind == "scalper_session",
            setupTypeLabel: isOptions ? nil : (viewModel.declSetupType.isEmpty ? nil : viewModel.declSetupType),
            invalidationTypeWire: isOptions ? nil : selectedInvalidationKind?.rawValue,
            optionLeg: isOptions ? nil : optionLeg(symbol: sym, lots: lotsParsed),
            optionLegs: isOptions ? viewModel.optionLegs : [],
            horizonDays: viewModel.declHorizonDays,
            maxPlannedLossINR: maxLoss,
        )
        return try? JSONSerialization.data(withJSONObject: obj, options: [])
    }

    private func optionLeg(symbol: String, lots: Int?) -> BarIntradayDeclarationPayload.OptionLeg? {
        guard viewModel.declareAssetClass == .options, let lots, lots > 0 else { return nil }
        let right = viewModel.declOptionRight == "PE" ? "PE" : "CE"
        return BarIntradayDeclarationPayload.OptionLeg(
            underlying: symbol,
            expiry: viewModel.declOptionExpiry.trimmingCharacters(in: .whitespacesAndNewlines),
            strike: viewModel.declOptionStrike.trimmingCharacters(in: .whitespacesAndNewlines),
            right: right,
            sideBuy: sideBuy,
            lots: lots,
        )
    }

    private var planUnits: Double? {
        if viewModel.declareAssetClass == .options {
            let t = viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)
            guard let lots = Int(t), lots > 0 else { return nil }
            return Double(lots)
        }
        guard let q = Double(quantityText.trimmingCharacters(in: .whitespacesAndNewlines)), q > 0 else {
            return nil
        }
        return q
    }

    private var planRung1: Double? {
        BarPlanLadder.rung1(
            units: planUnits,
            entry: Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)),
            stop: Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)),
            sideBuy: sideBuy,
        )
    }

    private var planMaxLossINR: Double? {
        BarPlanLadder.maxPlannedLossINR(rung1: planRung1)
    }

    private func pretradeSidePill(title: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(selected ? BarDS.Text.primary : BarDS.Text.secondary)
                .padding(.vertical, 6)
                .padding(.horizontal, 16)
                .background(selected ? Color.white.opacity(0.10) : Color.white.opacity(0.03))
                .clipShape(Capsule())
                .overlay(
                    Capsule()
                        .stroke(
                            selected ? Color.white.opacity(0.20) : Color.white.opacity(0.10),
                            lineWidth: 0.5,
                        ),
                )
        }
        .buttonStyle(.plain)
    }

    private enum CalmWarnKind { case amber, red }

    private func stateWarningBox(text: String, kind: CalmWarnKind) -> some View {
        Text(text)
            .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
            .foregroundColor(kind == .red ? BarDS.Accent.red : BarDS.Accent.amber)
            .lineSpacing(4)
            .fixedSize(horizontal: false, vertical: true)
            .padding(8)
            .frame(maxWidth: .infinity, alignment: .leading)
            .background(kind == .red ? BarDS.Accent.red.opacity(0.08) : BarDS.Accent.amber.opacity(0.08))
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(kind == .red ? BarDS.Semantic.redBorder() : BarDS.Semantic.amberBorder(), lineWidth: BarDS.borderThin),
            )
    }

    private func feelRow(
        value: Binding<Int>,
        labels: [Int: String],
        feel: @escaping (Int, Int) -> BarFeelButton.FeelState,
    ) -> some View {
        HStack(spacing: 5) {
            ForEach(1 ... 5, id: \.self) { i in
                Button {
                    value.wrappedValue = i
                } label: {
                    BarFeelButton(
                        number: i,
                        label: labels[i] ?? "",
                        state: feel(value.wrappedValue, i),
                    )
                    .frame(maxWidth: .infinity, minHeight: 56)
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func calmFeelState(selected: Int, index: Int) -> BarFeelButton.FeelState {
        guard selected == index, (1 ... 5).contains(selected) else { return .unselected }
        switch index {
        case 1, 2: return .teal
        case 3: return .amber
        case 4, 5: return .red
        default: return .unselected
        }
    }

    private func confidenceFeelState(selected: Int, index: Int) -> BarFeelButton.FeelState {
        guard selected == index, (1 ... 5).contains(selected) else { return .unselected }
        switch index {
        case 1, 2: return .red
        case 3: return .amber
        case 4, 5: return .teal
        default: return .unselected
        }
    }

    private var deskGlanceStrip: some View {
        VStack(alignment: .leading, spacing: 4) {
            BarSectionLabel(text: "Desk extracts")
            ForEach(visibleGlanceKinds, id: \.rawValue) { kind in
                HStack {
                    Text(kind.title)
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundColor(BarDS.Text.hint)
                    Spacer()
                    if let honesty = glanceHonesty(kind) {
                        HonestyChip(status: honesty)
                    } else {
                        Text(glanceDetail(kind))
                            .font(BarDS.monoFont(10, weight: .medium))
                            .foregroundColor(BarDS.Text.muted)
                            .lineLimit(2)
                            .multilineTextAlignment(.trailing)
                    }
                }
            }
        }
        .padding(.bottom, 6)
    }

    private var visibleGlanceKinds: [BarDeskInstrumentKind] {
        BarDeskTemplate.glanceKinds(for: viewModel.declareAssetClass).filter { kind in
            switch kind {
            case .chain: return viewModel.wantOptionsChain
            case .openInterest: return viewModel.wantOptionsOI
            default: return true
            }
        }
    }

    private func glanceHonesty(_ kind: BarDeskInstrumentKind) -> HonestyStatus? {
        switch kind {
        case .chain:
            return HonestyStatus.fromWire(viewModel.deskChainStatus)
        case .openInterest:
            return HonestyStatus.fromWire(viewModel.deskOiStatus)
        case .last, .history:
            return nil
        }
    }

    private func glanceDetail(_ kind: BarDeskInstrumentKind) -> String {
        switch kind {
        case .last:
            return viewModel.deskLastStatus
        case .history:
            return BarDeskTemplate.historyGlanceLine(
                licensedStatus: viewModel.deskHistoryStatus,
                licensedIneligible: viewModel.deskHistoryIneligible,
                yahooStatus: viewModel.deskYahooHistoryStatus,
                yahooIneligible: viewModel.deskYahooHistoryIneligible,
                stitchYahoo: false
            )
        case .chain:
            return viewModel.deskChainStatus
        case .openInterest:
            return viewModel.deskOiStatus
        }
    }

    private var optionsShellFields: some View {
        VStack(alignment: .leading, spacing: 6) {
            BarSectionLabel(text: "Options template")
            BarInputField(placeholder: "Expiry (YYYY-MM-DD)", text: $viewModel.declOptionExpiry)
            BarInputField(placeholder: "Strike", text: $viewModel.declOptionStrike)
            HStack(spacing: 5) {
                BarChip(label: "CE", selected: viewModel.declOptionRight == "CE") {
                    viewModel.declOptionRight = "CE"
                }
                BarChip(label: "PE", selected: viewModel.declOptionRight == "PE") {
                    viewModel.declOptionRight = "PE"
                }
            }
            BarInputField(placeholder: "Lots", text: $viewModel.declLots)
            horizonRow
            HStack(spacing: 5) {
                BarChip(label: "Chain", selected: viewModel.wantOptionsChain) {
                    viewModel.wantOptionsChain.toggle()
                }
                BarChip(label: "OI", selected: viewModel.wantOptionsOI) {
                    viewModel.wantOptionsOI.toggle()
                }
            }
        }
        .padding(.bottom, 4)
    }

    private var horizonRow: some View {
        HStack(spacing: 8) {
            Text("Horizon: \(viewModel.declHorizonDays) days")
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(BarDS.Text.hint)
            Spacer(minLength: 8)
            Stepper(
                "",
                value: $viewModel.declHorizonDays,
                in: BarPlanHorizon.dayRange,
            )
            .labelsHidden()
            .fixedSize()
        }
        .padding(.bottom, 4)
    }

    private func selectArchetype(_ archetype: TraderArchetype) {
        viewModel.setUserDeclarationArchetype(archetype)
        viewModel.declHorizonDays = BarPlanHorizon.defaultFor(declarationKindWire(for: archetype))
        viewModel.declHorizonMode = BarPlanHorizon.mode(forDays: viewModel.declHorizonDays, dte: nil)
    }

    private var symbolAutocompleteField: some View {
        VStack(alignment: .leading, spacing: 0) {
            BarInputField(
                placeholder: "Symbol (e.g. BTC or RELIANCE)",
                text: $viewModel.barDeclarationSymbol,
                marginBottom: (viewModel.showSymbolSuggestions
                    || !(viewModel.symbolSearchHint ?? "").isEmpty) ? 0 : 7,
            )
            .onChange(of: viewModel.barDeclarationSymbol) { _, newValue in
                viewModel.searchSymbols(newValue)
            }

            if viewModel.showSymbolSuggestions {
                // No nested ScrollView — parent page already scrolls, and a nested
                // one plus a focused TextField swallows the first click on macOS.
                VStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(viewModel.symbolSuggestions.prefix(5))) { result in
                        HStack {
                            VStack(alignment: .leading, spacing: 2) {
                                Text(result.trading_symbol)
                                    .font(.system(size: 13, weight: .medium))
                                Text(result.name)
                                    .font(.system(size: 11))
                                    .foregroundStyle(.secondary)
                                    .lineLimit(1)
                            }
                            Spacer()
                            Text(result.venueLabel)
                                .font(.system(size: 11))
                                .foregroundStyle(.tertiary)
                        }
                        .padding(.horizontal, 8)
                        .padding(.vertical, 6)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .contentShape(Rectangle())
                        .highPriorityGesture(
                            TapGesture().onEnded { viewModel.selectSymbol(result) }
                        )
                        .accessibilityAddTraits(.isButton)
                        .accessibilityLabel("\(result.trading_symbol) \(result.venueLabel)")
                        Divider()
                    }
                }
                .background(.regularMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 6))
                .overlay(RoundedRectangle(cornerRadius: 6).stroke(Color.primary.opacity(0.1)))
                .padding(.bottom, 7)
                .zIndex(1)
            } else if let hint = viewModel.symbolSearchHint, !hint.isEmpty {
                HStack(alignment: .center, spacing: 8) {
                    Text(hint)
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundStyle(.secondary)
                    if hint == "Catalog failed"
                        || (DeskCapabilityChrome.showsRetryInstruments(
                            status: viewModel.deskInstrumentsCapability
                        ) && viewModel.symbolSuggestions.isEmpty)
                    {
                        Button("Retry instruments") {
                            Task { await viewModel.retryInstruments() }
                        }
                        .buttonStyle(.plain)
                        .font(BarDS.bodyFont(11, weight: .medium))
                        .foregroundStyle(BarDS.Accent.teal)
                    }
                }
                .padding(.top, 4)
                .padding(.bottom, 7)
            }
        }
    }

    /// Maps Notch archetype tab → API `declaration_kind` (scalper → `scalper_session`).
    /// Prefill empty fields from daemon positions / recent fills (trdSym-backed).
    private func applyBrokerDeclarationPrefillIfNeeded() {
        guard viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              quantityText.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty,
              let pre = viewModel.barDeclarationPrefillDefaults()
        else { return }
        viewModel.barDeclarationSymbol = pre.symbol
        sideBuy = pre.sideBuy
        if !pre.quantity.isEmpty {
            quantityText = pre.quantity
        }
    }

    private func declarationKindWire(for archetype: TraderArchetype) -> String {
        switch archetype {
        case .intraday:
            return "intraday"
        case .scalper:
            return "scalper_session"
        case .swing:
            return "swing"
        }
    }
}
