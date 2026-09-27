import SwiftUI

/// Pre-trade declaration. Spot / equity / USDM use the prototype cockpit (strip + mosaic + Plan rail).
/// NFO options keep mosaic+rail+6-cell strip; NFO futures keep three-zone;
/// dated crypto Options is mosaic + 6-cell strip + Plan rail (ticket-C on the mosaic).
struct BarDeclarationFlowView: View {
    @ObservedObject var viewModel: NotchViewModel

    /// Scale A (calm, 1 best) — `0` until user taps (mirrors `declEmotionalCalm`).
    /// Scale B (confidence, 5 best) — `0` until user taps (`declEmotionalConfidence`).
    @State private var sideBuy: Bool = true
    @State private var quantityText: String = ""
    @State private var stopLossText: String = ""
    @State private var scalperSessionId: String = ""
    @State private var targetPriceText: String = ""

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

                switch BarOptionsDeclareSurface.surface(
                    for: viewModel.declareAssetClass,
                    slug: viewModel.resolvedDeskSlug,
                    instrumentId: viewModel.deskSelectedInstrumentId,
                    instrumentType: viewModel.deskSelectedInstrumentType,
                ) {
                case .nfoCockpit:
                    BarNfoOptionsCockpitView(
                        viewModel: viewModel,
                        sideBuy: $sideBuy,
                        stopLossText: $stopLossText,
                        targetPriceText: $targetPriceText,
                        submitReady: submitReadiness.ready,
                        submitHint: submitReadiness.hint,
                        onConfirm: { Task { await submit() } },
                    )
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                case .nfoThreeZone:
                    ScrollView {
                        BarOptionsDeclareView(
                            viewModel: viewModel,
                            sideBuy: $sideBuy,
                            stopLossText: $stopLossText,
                            targetPriceText: $targetPriceText,
                            submitReady: submitReadiness.ready,
                            submitHint: submitReadiness.hint,
                            onConfirm: { Task { await submit() } },
                        )
                    }
                    .scrollIndicators(.hidden)
                case .cryptoOptions:
                    BarCryptoOptionsDeclareView(
                        viewModel: viewModel,
                        sideBuy: $sideBuy,
                        quantityText: $quantityText,
                        stopLossText: $stopLossText,
                        targetPriceText: $targetPriceText,
                        submitReady: submitReadiness.ready,
                        submitHint: submitReadiness.hint,
                        onConfirm: { Task { await submit() } },
                    )
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                case .standardForm:
                    BarCashCockpitView(
                        viewModel: viewModel,
                        sideBuy: $sideBuy,
                        quantityText: $quantityText,
                        stopLossText: $stopLossText,
                        targetPriceText: $targetPriceText,
                        submitReady: submitReadiness.ready,
                        submitHint: submitReadiness.hint,
                        onConfirm: { Task { await submit() } },
                    )
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
                }
            }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onAppear {
            viewModel.setIfChanged(\.declInvalidationType, "")
            viewModel.setIfChanged(\.declInvalidationCondition, "")
            viewModel.adoptDeskTicket()
            applyBrokerDeclarationPrefillIfNeeded()
        }
        .onChange(of: viewModel.showingDeclarationForm) { _, showing in
            if !showing {
                viewModel.dismissSymbolSuggestions()
            }
        }
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
                isUsdm: viewModel.declareAssetClass.isNamedComFutures,
                optionLegCount: viewModel.optionLegs.count,
                maxPlannedLossText: viewModel.declMaxPlannedLossText,
                frustration: viewModel.declEmotionalFrustration,
                excitement: viewModel.declEmotionalExcitement,
                stanceRaw: viewModel.declStance,
                intent: viewModel.declIntent,
                targetPriceText: targetPriceText,
                invalidationPriceText: viewModel.declInvalidationPrice,
                requiresCashProduct: viewModel.requiresCashProduct,
                cashProduct: viewModel.declCashProduct,
                gate: viewModel.declGateStripState,
                usesVenueTicket: showsVenueTicket,
                quoteOrderQtyText: viewModel.quoteOrderQtyText,
                deskSizeModeRaw: viewModel.deskTicket.sizeMode.rawValue,
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
            brokerChipRow
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

    @ViewBuilder
    private var brokerChipRow: some View {
        if let slug = viewModel.activeExecutionBrokerSlug?.trimmingCharacters(in: .whitespacesAndNewlines),
           !slug.isEmpty
        {
            HStack(spacing: 6) {
                Text("BROKER")
                    .font(BarDS.monoFont(9, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(0.6)
                Text(NotchViewModel.brokerDisplayName(forSlug: slug))
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .padding(.vertical, 4)
                    .padding(.horizontal, 10)
                    .background(BarDS.Fill.elevated)
                    .clipShape(Capsule())
                    .overlay(
                        Capsule().stroke(BarDS.Border.chipUnselected, lineWidth: BarDS.borderThin),
                    )
            }
        }
    }

    private func submit() async {
        viewModel.barDeclarationLastError = nil
        guard viewModel.canSubmitBarDeclaration else { return }
        guard let data = buildJsonBody() else {
            viewModel.barDeclarationLastError = submitReadiness.hint ?? "Fix trade fields before submitting."
            return
        }
        await viewModel.submitBarDeclaration(body: data)
    }

    private func buildJsonBody() -> Data? {
        let isOptions = viewModel.declareAssetClass == .options
        let isNamedFutures = viewModel.declareAssetClass == .usdm
            || viewModel.declareAssetClass == .coinm
        guard viewModel.barLiveState?.blocksDeclarationSubmit != true else { return nil }
        if !isOptions, !isNamedFutures {
            guard viewModel.declProtectiveSLConsent else { return nil }
        }
        guard (1 ... 5).contains(viewModel.declEmotionalCalm),
              (1 ... 5).contains(viewModel.declEmotionalConfidence),
              (1 ... 5).contains(viewModel.declEmotionalFrustration),
              (1 ... 5).contains(viewModel.declEmotionalExcitement) else { return nil }
        guard BarPlanStance(rawValue: viewModel.declStance) != nil else { return nil }
        let intentTrim = viewModel.declIntent.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !intentTrim.isEmpty else { return nil }
        guard BarIntradayDeclareValidator.stickyPreTradeConfirmEnabled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            stopLossText: stopLossText,
        ) else { return nil }
        guard let sym = BarBrokerTicker.normalize(raw: viewModel.barDeclarationSymbol) else { return nil }
        let lotsParsed = Int(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines))
        viewModel.persistDeskTicket()
        let ticketForBody = showsVenueTicket ? viewModel.deskTicket.coerced() : nil
        let qty: Double
        if isOptions {
            guard !viewModel.optionLegs.isEmpty else { return nil }
            qty = Double(viewModel.optionLegs[0].lots)
        } else if ticketForBody?.sizeMode == .quote, let q = ticketForBody?.quoteOrderQty, q > 0 {
            qty = q
        } else if let parsedQty = Double(quantityText.trimmingCharacters(in: .whitespaces)), parsedQty > 0 {
            qty = VenueLotTick.round(parsedQty, stepSize: viewModel.deskStepSize)
        } else {
            return nil
        }
        guard let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)) else { return nil }
        if !isOptions {
            guard !viewModel.declSetupType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
            guard selectedInvalidationKind != nil else { return nil }
        }
        let invTrim = viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines)
        let invPrice = Double(viewModel.declInvalidationPrice.trimmingCharacters(in: .whitespacesAndNewlines))
        if selectedInvalidationKind == .price {
            guard let invPrice, invPrice > 0 else { return nil }
        } else {
            guard !invTrim.isEmpty else { return nil }
        }
        let targetTrimForGate = targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let targetParsed = Double(targetTrimForGate), targetParsed > 0 else { return nil }
        if viewModel.requiresCashProduct {
            let p = viewModel.declCashProduct.uppercased()
            guard p == "CNC" || p == "MIS" else { return nil }
        }
        let emotionOk = BarPlanGateStrip.emotionFilled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            frustration: viewModel.declEmotionalFrustration,
            excitement: viewModel.declEmotionalExcitement
        )
        guard BarPlanGateStrip.isComplete(
            state: viewModel.declGateStripState,
            emotionFilled: emotionOk,
            exitFilled: true
        ) else { return nil }
        let wireKind = declarationKindWire(for: viewModel.activeArchetype)
        if wireKind == "scalper_session",
           scalperSessionId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return nil
        }

        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        let entryTrim = viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)
        let entryOpt = !entryTrim.isEmpty ? Double(entryTrim) : nil
        let typedMax = Double(viewModel.declMaxPlannedLossText.trimmingCharacters(in: .whitespacesAndNewlines))
        let maxLoss = isOptions ? typedMax : planMaxLossINR
        if isOptions, typedMax == nil { return nil }

        let first = viewModel.optionLegs.first
        let invNoteOut: String? = {
            if selectedInvalidationKind == .price {
                return invTrim.isEmpty ? nil : invTrim
            }
            return invTrim
        }()
        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: first?.underlying ?? sym,
            sideBuy: first?.sideBuy ?? sideBuy,
            quantity: qty,
            stopLoss: sl,
            declarationKind: wireKind,
            moodStress: Double(calm),
            moodImpulse: Double(conf),
            invalidationNote: invNoteOut,
            protectiveSlConsent: isNamedFutures ? false : (isOptions ? true : viewModel.declProtectiveSLConsent),
            entryPrice: entryOpt,
            targetPrice: targetParsed,
            scalperSessionId: wireKind == "scalper_session" ? scalperSessionId : nil,
            isSessionLevel: wireKind == "scalper_session",
            setupTypeLabel: isOptions ? nil : (viewModel.declSetupType.isEmpty ? nil : viewModel.declSetupType),
            invalidationTypeWire: isOptions ? nil : selectedInvalidationKind?.rawValue,
            optionLeg: isOptions ? nil : optionLeg(symbol: sym, lots: lotsParsed),
            optionLegs: isOptions ? viewModel.optionLegs : [],
            horizonDays: viewModel.declHorizonDays,
            maxPlannedLossINR: maxLoss,
            bookId: viewModel.declareBookId,
            ticket: ticketForBody,
            moodFrustration: Double(viewModel.declEmotionalFrustration),
            moodExcitement: Double(viewModel.declEmotionalExcitement),
            stance: viewModel.declStance,
            intent: intentTrim,
            invalidationPrice: selectedInvalidationKind == .price ? invPrice : nil,
            cashProduct: viewModel.requiresCashProduct ? viewModel.declCashProduct : nil,
            gateStrip: viewModel.declGateStripState,
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

    private var showsVenueTicket: Bool {
        BarDeskTicketSurface.usesVenueTicket(
            for: viewModel.declareAssetClass,
            slug: viewModel.resolvedDeskSlug,
            instrumentId: viewModel.deskSelectedInstrumentId,
        )
    }

    private var selectedInvalidationKind: BarInvalidationKind? {
        let t = viewModel.declInvalidationType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return BarInvalidationKind(rawValue: t)
    }

    private func selectArchetype(_ archetype: TraderArchetype) {
        viewModel.setUserDeclarationArchetype(archetype)
        viewModel.declHorizonDays = BarPlanHorizon.defaultFor(declarationKindWire(for: archetype))
        viewModel.declHorizonMode = BarPlanHorizon.mode(forDays: viewModel.declHorizonDays, dte: nil)
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
