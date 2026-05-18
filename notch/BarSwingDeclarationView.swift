import SwiftUI

/// Swing pre-trade declaration — same emotional + risk gates as intraday, plus holding-plan fields and overnight / daily check-in flags in `declaration_payload.s1` (#118).
struct BarSwingDeclarationView: View {
    @ObservedObject var viewModel: NotchViewModel

    @State private var step: Int = 0

    @State private var calmSelection: Int?
    @State private var confidenceSelection: Int?
    @State private var symbol: String = ""
    @State private var sideBuy: Bool = true
    @State private var quantityText: String = ""
    @State private var stopLossText: String = ""
    @State private var holdingDaysText: String = ""
    @State private var maxDrawdownPctText: String = ""
    @State private var swingInvalidationKind: BarInvalidationKind? = nil
    @State private var invalidationNote: String = ""
    @State private var acceptsOvernightRisk: Bool = false
    @State private var dailyCheckInRequired: Bool = false
    @State private var protectiveConsent: Bool = false

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

                Text(stepTitle)
                    .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal)
                    .kerning(0.08 * 10)
                    .textCase(.uppercase)

                stepContent

                HStack(spacing: 12) {
                    if step > 0 {
                        Button("Back") { step -= 1 }
                            .buttonStyle(.plain)
                            .font(BarDS.bodyFont(11, weight: .semibold))
                            .foregroundColor(BarDS.Text.secondary)
                    }
                    Spacer(minLength: 0)
                    if step < 5 {
                        Button("Next") {
                            if stepValid(step) { step += 1 }
                        }
                        .buttonStyle(.plain)
                        .font(BarDS.bodyFont(11, weight: .semibold))
                        .foregroundColor(stepValid(step) ? BarDS.Accent.teal : BarDS.Text.hint)
                        .disabled(!stepValid(step))
                    }
                }
                .padding(.top, 4)

                if step == 5 {
                    consentAndSubmit
                }
            }
        }
    }

    private var stepTitle: String {
        switch step {
        case 0: return "Step 1 / 6 — Emotional check-in"
        case 1: return "Step 2 / 6 — Trade risk"
        case 2: return "Step 3 / 6 — Holding plan"
        case 3: return "Step 4 / 6 — Invalidation"
        case 4: return "Step 5 / 6 — Review"
        case 5: return "Step 6 / 6 — Overnight risk & consent"
        default: return ""
        }
    }

    @ViewBuilder
    private var stepContent: some View {
        switch step {
        case 0:
            emotionalCheckInSection
        case 1:
            tradeRiskSection
        case 2:
            Text("How long are you willing to hold this thesis (trading days), and what max drawdown still keeps the idea valid?")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            BarInputField(placeholder: "Holding horizon (days)", text: $holdingDaysText)
            BarInputField(placeholder: "Max drawdown (% of position)", text: $maxDrawdownPctText)
        case 3:
            swingInvalidationKindRow
            Group {
                if swingInvalidationKind != nil {
                    swingInvalidationNoteBlock
                }
            }
            .animation(.easeInOut(duration: 0.15), value: swingInvalidationKind)
        case 4:
            Text(reviewSummary)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .fixedSize(horizontal: false, vertical: true)
        case 5:
            Text(
                "Swing trades can gap overnight. Confirm whether you accept overnight risk and whether TradeAutopsy should require a daily thesis check-in while this declaration is active."
            )
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
            .foregroundColor(BarDS.Text.secondary)
            .fixedSize(horizontal: false, vertical: true)
            BarToggleRow(
                label: "Accept overnight / gap risk",
                sub: "Gaps can print through stops — confirm you accept this for this swing.",
                isOn: $acceptsOvernightRisk,
            )
            BarToggleRow(
                label: "Daily check-in prompt",
                sub: "While the position is open, surface a short thesis check-in.",
                isOn: $dailyCheckInRequired,
            )
            Text(
                "Protective stop consent matches the web Bar — submit stays disabled until you toggle consent on the block below."
            )
            .font(BarDS.bodyFont(10, weight: .medium))
            .foregroundColor(BarDS.Text.hint)
            .fixedSize(horizontal: false, vertical: true)
            .padding(.top, 4)
        default:
            EmptyView()
        }
    }

    private var swingInvalidationKindRow: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Invalidation type")
                .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                .foregroundColor(BarDS.Accent.teal)
                .kerning(0.08 * 10)
                .textCase(.uppercase)
            HStack(spacing: 6) {
                ForEach(BarInvalidationKind.allCases) { k in
                    let on = swingInvalidationKind == k
                    Button {
                        swingInvalidationKind = k
                    } label: {
                        VStack(spacing: 4) {
                            swingInvalidationIcon(for: k)
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
    }

    private var swingInvalidationNoteBlock: some View {
        let kind = swingInvalidationKind
        return VStack(alignment: .leading, spacing: 6) {
            TextField(
                "",
                text: $invalidationNote,
                prompt:
                    Text(BarInvalidationKind.textareaPlaceholderHint(for: kind))
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.labels),
                axis: .vertical,
            )
            .lineLimit(3...6)
            .textFieldStyle(.plain)
            .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
            .foregroundColor(BarDS.Text.primary)
            .padding(.vertical, 8)
            .padding(.horizontal, 10)
            .background(BarDS.Fill.input)
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(BarDS.Border.input, lineWidth: BarDS.borderThin)
            )

            Text(swingInvalidationExampleHint(kind))
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(Color(hex: "#444444"))
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 4)
        }
        .transition(.opacity.combined(with: .move(edge: .top)))
    }

    private func swingInvalidationExampleHint(_ kind: BarInvalidationKind?) -> String {
        switch kind {
        case .time:
            return "Example: \"By Friday close — thesis expires if price hasn't cleared prior week high.\""
        case .behaviour:
            return "Example: \"If spot closes back below the breakout swing low on the daily.\""
        case .context:
            return "Example: \"If sector ETF rolls over before earnings, swing thesis is void.\""
        case .none:
            return ""
        }
    }

    @ViewBuilder
    private func swingInvalidationIcon(for k: BarInvalidationKind) -> some View {
        switch k {
        case .time:
            Image(systemName: "clock")
        case .behaviour:
            Image(systemName: "chart.xyaxis.line")
        case .context:
            Image(systemName: "globe")
        }
    }

    private var tradeRiskSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            BarInputField(placeholder: "Symbol (e.g. RELIANCE)", text: $symbol)
            HStack(spacing: 8) {
                BarChip(label: "BUY", selected: sideBuy) { sideBuy = true }
                BarChip(label: "SELL", selected: !sideBuy) { sideBuy = false }
                Spacer(minLength: 0)
            }
            BarInputField(placeholder: "Quantity", text: $quantityText)
            BarInputField(placeholder: "Stop loss (price)", text: $stopLossText)
        }
    }

    private var reviewSummary: String {
        let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let side = sideBuy ? "BUY" : "SELL"
        let q = quantityText
        let sl = stopLossText
        let hd = holdingDaysText
        let dd = maxDrawdownPctText
        let inv = invalidationNote.trimmingCharacters(in: .whitespacesAndNewlines)
        var lines = """
        \(sym) \(side) × \(q) @ stop \(sl)
        Kind: Swing
        Holding plan: \(hd) days · max drawdown \(dd)%
        Overnight risk accepted: \(acceptsOvernightRisk ? "yes" : "no — set on next step")
        Daily check-in: \(dailyCheckInRequired ? "on" : "off — set on next step")
        """
        if !inv.isEmpty {
            lines += "\nInvalidation: \(inv)"
        }
        lines += "\nNext: overnight toggles + protective SL consent"
        return lines
    }

    private var consentAndSubmit: some View {
        VStack(alignment: .leading, spacing: 10) {
            BarToggleRow(
                label: "Auto-place stop loss on fill",
                sub: "Same consent field as the web Bar",
                isOn: $protectiveConsent,
            )
            .disabled(viewModel.barLiveState?.blocksDeclarationSubmit == true)

            BarBigButton(
                label: viewModel.barDeclarationBusy ? "Submitting…" : "Submit swing declaration",
                style: .primary,
            ) {
                Task { await submit() }
            }
            .disabled(!submitEnabled || viewModel.barDeclarationBusy)
            .opacity(submitEnabled && !viewModel.barDeclarationBusy ? 1 : 0.45)
        }
        .padding(.top, 8)
    }

    private var submitEnabled: Bool {
        guard viewModel.barLiveState?.blocksDeclarationSubmit != true else { return false }
        guard protectiveConsent else { return false }
        guard acceptsOvernightRisk else { return false }
        return stepValid(1) && stepValid(2) && stepValid(3)
    }

    private func stepValid(_ s: Int) -> Bool {
        switch s {
        case 0:
            return BarIntradayDeclareValidator.canProceedFromEmotionalCheckIn(
                calm: calmSelection,
                confidence: confidenceSelection,
            )
        case 1:
            let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !sym.isEmpty else { return false }
            guard let q = Double(quantityText.trimmingCharacters(in: .whitespaces)), q > 0 else { return false }
            guard let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)), sl > 0 else { return false }
            return true
        case 2:
            guard let days = Int(holdingDaysText.trimmingCharacters(in: .whitespacesAndNewlines)), days > 0 else {
                return false
            }
            guard let pct = Double(maxDrawdownPctText.trimmingCharacters(in: .whitespacesAndNewlines)), pct > 0 else {
                return false
            }
            return true
        case 3:
            guard swingInvalidationKind != nil else { return false }
            let inv = invalidationNote.trimmingCharacters(in: .whitespacesAndNewlines)
            return !inv.isEmpty
        default:
            return true
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
        guard stepValid(1), stepValid(2), stepValid(3) else { return nil }
        let sym = symbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard let qty = Double(quantityText.trimmingCharacters(in: .whitespaces)),
              let sl = Double(stopLossText.trimmingCharacters(in: .whitespaces)) else { return nil }

        guard let calm = calmSelection, let conf = confidenceSelection else { return nil }
        var s1: [String: Any] = [
            "mood_stress": Double(calm),
            "mood_impulse": Double(conf),
        ]
        let inv = invalidationNote.trimmingCharacters(in: .whitespacesAndNewlines)
        if !inv.isEmpty { s1["invalidation"] = inv }
        if let k = swingInvalidationKind {
            s1["invalidation_type"] = k.rawValue
        }

        let days = Int(holdingDaysText.trimmingCharacters(in: .whitespacesAndNewlines))
        let maxDd = Double(maxDrawdownPctText.trimmingCharacters(in: .whitespacesAndNewlines))
        let swingAug = BarSwingDeclarationS1Augmentation(
            acceptsOvernightRisk: acceptsOvernightRisk,
            dailyCheckInRequired: dailyCheckInRequired,
            holdingDays: days,
            maxDrawdownPct: maxDd,
        )
        let s1Out = swingAug.merged(into: s1)

        let o: [String: Any] = [
            "symbol": sym,
            "declaration_kind": "swing",
            "side": sideBuy ? "BUY" : "SELL",
            "quantity": qty,
            "stop_loss": sl,
            "declaration_payload": [
                "v": 1,
                "s1": s1Out,
                "protective_sl_consent": protectiveConsent,
            ] as [String: Any],
        ]
        return try? JSONSerialization.data(withJSONObject: o, options: [])
    }

    private var emotionalCheckInSection: some View {
        VStack(alignment: .leading, spacing: 10) {
            BarSectionLabel(text: "Psychological calm — 1 is best")
            feelRow(
                selection: $calmSelection,
                labels: [1: "Calm", 2: "Focused", 3: "Tense", 4: "Anxious", 5: "Angry"],
                feel: calmFeelState,
            )
            if let c = calmSelection {
                if c >= 4 {
                    stateWarningBox(
                        text: "State \(c) — position size halved. Trade will be flagged.",
                        kind: .red,
                    )
                } else if c == 3 {
                    stateWarningBox(
                        text: "State 3 — trade with heightened awareness.",
                        kind: .amber,
                    )
                }
            }
            BarSectionLabel(text: "Confidence — 5 is best")
            feelRow(
                selection: $confidenceSelection,
                labels: [1: "Low", 2: "Flat", 3: "Neutral", 4: "Good", 5: "Sharp"],
                feel: confidenceFeelState,
            )
        }
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
        selection: Binding<Int?>,
        labels: [Int: String],
        feel: @escaping (Int?, Int) -> BarFeelButton.FeelState,
    ) -> some View {
        HStack(spacing: 5) {
            ForEach(1 ... 5, id: \.self) { i in
                Button {
                    selection.wrappedValue = i
                } label: {
                    BarFeelButton(
                        number: i,
                        label: labels[i] ?? "",
                        state: feel(selection.wrappedValue, i),
                    )
                    .frame(maxWidth: .infinity, minHeight: 56)
                }
                .buttonStyle(.plain)
            }
        }
    }

    private func calmFeelState(selected: Int?, index: Int) -> BarFeelButton.FeelState {
        guard selected == index else { return .unselected }
        switch index {
        case 1, 2: return .teal
        case 3: return .amber
        case 4, 5: return .red
        default: return .unselected
        }
    }

    private func confidenceFeelState(selected: Int?, index: Int) -> BarFeelButton.FeelState {
        guard selected == index else { return .unselected }
        switch index {
        case 1, 2: return .red
        case 3: return .amber
        case 4, 5: return .teal
        default: return .unselected
        }
    }
}
