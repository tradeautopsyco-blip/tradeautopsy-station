import SwiftUI

/// Pre-trade declaration — single scrollable page (#121 / unified reference mockups 3–4).
struct BarDeclarationFlowView: View {
    @ObservedObject var viewModel: NotchViewModel

    /// Scale A (calm, 1 best) — `0` until user taps (mirrors `declEmotionalCalm`).
    /// Scale B (confidence, 5 best) — `0` until user taps (`declEmotionalConfidence`).
    @State private var symbol: String = ""
    @State private var sideBuy: Bool = true
    @State private var quantityText: String = ""
    @State private var stopLossText: String = ""
    @State private var kind: DeclarationKind = .intraday
    @State private var scalperSessionId: String = ""
    @State private var entryPriceText: String = ""
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
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                if viewModel.barLiveState?.blocksDeclarationSubmit == true {
                    Text("Circuit active — finish or clear the web Bar intervention before declaring.")
                        .font(BarDS.bodyFont(11, weight: .semibold))
                        .foregroundColor(BarDS.Accent.amber)
                        .padding(8)
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .background(BarDS.Semantic.amberBg())
                        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                        .overlay(
                            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                                .stroke(BarDS.Semantic.amberBorder(), lineWidth: BarDS.borderThin),
                        )
                }

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

                BarSectionLabel(text: "Step 1 of 4 — State check")
                stateCheckCard

                BarSectionLabel(text: "Step 2 of 4 — Risk numbers")
                riskNumbersCard

                BarSectionLabel(text: "Step 3 of 4 — Setup & invalidation")
                setupAndInvalidationCard

                BarSectionLabel(text: "Step 4 of 4 — Review & consent")
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
                .disabled(viewModel.barDeclarationBusy)
                .opacity(viewModel.barDeclarationBusy ? 0.45 : 1)

                Text("Completed in \(elapsedLiveSeconds)s")
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(Color(hex: "#333333"))
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .onAppear {
            if declarationStartedAt == nil {
                declarationStartedAt = Date()
                viewModel.declInvalidationType = ""
                viewModel.declInvalidationCondition = ""
            }
        }
        .onReceive(Timer.publish(every: 1, on: .main, in: .common).autoconnect()) { recapClock = $0 }
    }

    private var archetypeTabRow: some View {
        HStack(spacing: 5) {
            BarTab(label: "Intraday", active: viewModel.activeArchetype == .intraday) {
                viewModel.setUserDeclarationArchetype(.intraday)
            }
            BarTab(label: "Scalper", active: viewModel.activeArchetype == .scalper) {
                viewModel.setUserDeclarationArchetype(.scalper)
            }
            BarTab(label: "Swing", active: viewModel.activeArchetype == .swing) {
                viewModel.setUserDeclarationArchetype(.swing)
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
            BarInputField(placeholder: "Entry price", text: $entryPriceText)
            BarInputField(placeholder: "Stop loss — exact price", text: $stopLossText)
            BarInputField(placeholder: "Target price", text: $targetPriceText)
            BarInputField(placeholder: "Symbol (e.g. RELIANCE)", text: $symbol)
            HStack(spacing: 5) {
                pretradeSidePill(title: "BUY", selected: sideBuy) { sideBuy = true }
                pretradeSidePill(title: "SELL", selected: !sideBuy) { sideBuy = false }
            }
            .padding(.bottom, 4)
            BarInputField(placeholder: "Quantity", text: $quantityText)

            if let loss = maxPlannedLossINR {
                Text(loss)
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
                .foregroundColor(Color(hex: "#555555"))
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
            return Color(hex: "#ededed").opacity(0.9)
        }
        return r >= 2.0 ? Color(hex: "#22c55e") : Color(hex: "#ededed")
    }

    private var rrComputedRatio: Double? {
        guard let e = Double(entryPriceText.trimmingCharacters(in: .whitespaces)),
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
                        .foregroundColor(Color(hex: "#aaaaaa"))
                        .padding(.top, 10)
                        .padding(.leading, 6)
                }
                TextEditor(text: $viewModel.declInvalidationCondition)
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(Color(hex: "#aaaaaa"))
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
                .foregroundColor(Color(hex: "#444444"))
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
                            .foregroundColor(on ? Color(hex: "#60a5fa") : BarDS.Text.secondary)
                        Text(k.chipTitle)
                            .font(BarDS.bodyFont(11, weight: .regular))
                            .foregroundColor(on ? Color(hex: "#60a5fa") : BarDS.Text.secondary)
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
            viewModel.barDeclarationLastError = "Fix trade fields before submitting."
            return
        }
        await viewModel.submitBarDeclaration(body: data)
    }

    private func buildJsonBody() -> Data? {
        guard viewModel.barLiveState?.blocksDeclarationSubmit != true else { return nil }
        guard viewModel.declProtectiveSLConsent else { return nil }
        guard (1 ... 5).contains(viewModel.declEmotionalCalm),
              (1 ... 5).contains(viewModel.declEmotionalConfidence) else { return nil }
        guard BarIntradayDeclareValidator.stickyPreTradeConfirmEnabled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            stopLossText: stopLossText,
        ) else { return nil }
        let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard !sym.isEmpty else { return nil }
        guard let qty = Double(quantityText.trimmingCharacters(in: .whitespaces)), qty > 0,
              let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)) else { return nil }
        guard !viewModel.declSetupType.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty else { return nil }
        guard selectedInvalidationKind != nil else { return nil }
        let invTrim = viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !invTrim.isEmpty else { return nil }
        if kind == .scalper_session,
           scalperSessionId.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            return nil
        }

        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        let entryTrim = entryPriceText.trimmingCharacters(in: .whitespacesAndNewlines)
        let targetTrim = targetPriceText.trimmingCharacters(in: .whitespacesAndNewlines)
        let entryOpt = Double(entryTrim)
        let targetOpt = targetTrim.isEmpty ? nil : Double(targetTrim)

        let obj = BarIntradayDeclarationPayload.buildJSONObject(
            symbol: sym,
            sideBuy: sideBuy,
            quantity: qty,
            stopLoss: sl,
            declarationKind: kind.rawValue,
            moodStress: Double(calm),
            moodImpulse: Double(conf),
            invalidationNote: invTrim,
            protectiveSlConsent: viewModel.declProtectiveSLConsent,
            entryPrice: entryOpt,
            targetPrice: targetOpt,
            scalperSessionId: kind == .scalper_session ? scalperSessionId : nil,
            isSessionLevel: kind == .scalper_session,
            setupTypeLabel: viewModel.declSetupType.isEmpty ? nil : viewModel.declSetupType,
            invalidationTypeWire: selectedInvalidationKind?.rawValue,
        )
        return try? JSONSerialization.data(withJSONObject: obj, options: [])
    }

    private var maxPlannedLossINR: String? {
        guard let e = Double(entryPriceText.trimmingCharacters(in: .whitespaces)),
              let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)),
              let q = Double(quantityText.trimmingCharacters(in: .whitespaces)), q > 0
        else { return nil }
        let per = abs(e - sl)
        guard per > 0 else { return nil }
        let inr = per * q
        return String(format: "MAX LOSS (plan): ₹%.0f", inr)
    }

    private func pretradeSidePill(title: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(selected ? Color(hex: "#ededed") : Color(hex: "#666666"))
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
            .background(kind == .red ? Color(hex: "#ef4444").opacity(0.08) : Color(hex: "#eab308").opacity(0.08))
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
}
