import SwiftUI

/// Plan rail for the cash / last-only cockpit (`cashPlan()`). Confirm is LiveBook intent.
struct BarCashCockpitPlanRail: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    private static let setupChips: [BarIntradaySetupChip] = [
        .breakout, .pullback, .reversal, .gapFill, .meanReversion, .supportBounce,
    ]

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 14) {
                Text("PLAN")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(1.6)
                Text("what you are declaring · \(contractLabel) · DualNoBlend \(unit)")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("State check")
                stateCheck

                groupLab("Numbers")
                numbersFields

                if isOptions {
                    groupLab("Contract")
                    optionsFields
                } else {
                    groupLab("Setup")
                    setupChipsRow
                    groupLab("Invalidation")
                    invalidationKindRow
                    premortem
                }

                if isOptions {
                    groupLab("Invalidation")
                    premortem
                }

                if viewModel.showsConfirmControl {
                    confirmBar
                }
            }
            .padding(.vertical, 4)
        }
        .frame(minWidth: 260, idealWidth: 300, maxWidth: 360, alignment: .topLeading)
    }

    private var isOptions: Bool { viewModel.declareAssetClass == .options }

    private var isNamedFutures: Bool { viewModel.declareAssetClass.isNamedComFutures }

    private var showsVenueTicket: Bool {
        BarDeskTicketSurface.usesVenueTicket(
            for: viewModel.declareAssetClass,
            slug: viewModel.resolvedDeskSlug,
            instrumentId: viewModel.deskSelectedInstrumentId,
        )
    }

    private var unit: String {
        switch viewModel.declareAssetClass {
        case .equity: return "INR"
        case .coinm: return "USD"
        default: return "USDT"
        }
    }

    private var contractLabel: String {
        let s = viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines)
        return s.isEmpty ? "—" : s
    }

    // MARK: - State

    @ViewBuilder
    private var stateCheck: some View {
        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        if (1 ... 5).contains(calm), (1 ... 5).contains(conf) {
            let cw = ["Calm", "Focused", "Tense", "Anxious", "Angry"][calm - 1]
            let cf = ["Low", "Flat", "Neutral", "Good", "Sharp"][conf - 1]
            HStack(spacing: 10) {
                Text("CALM")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                Text("\(calm) · \(cw)")
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                Text("CONFIDENCE")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                Text("\(conf) · \(cf)")
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                Spacer(minLength: 8)
                Button("change") {
                    viewModel.declEmotionalCalm = 0
                    viewModel.declEmotionalConfidence = 0
                }
                .buttonStyle(.plain)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            }
            .padding(9)
            .background(BarDS.Fill.elevated)
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        } else {
            VStack(alignment: .leading, spacing: 10) {
                Text("Two readings. Answer honestly — it changes what the system flags.")
                    .font(BarDS.bodyFont(12.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                scaleRow(
                    title: "Psychological calm — 1 is best",
                    labels: ["Calm", "Focused", "Tense", "Anxious", "Angry"],
                    value: Binding(
                        get: { viewModel.declEmotionalCalm },
                        set: { viewModel.declEmotionalCalm = $0 },
                    ),
                )
                if viewModel.declEmotionalCalm >= 4 {
                    Text("State \(viewModel.declEmotionalCalm) — size halved. Trade will be flagged.")
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Accent.red)
                        .fixedSize(horizontal: false, vertical: true)
                }
                scaleRow(
                    title: "Confidence — 5 is best",
                    labels: ["Low", "Flat", "Neutral", "Good", "Sharp"],
                    value: Binding(
                        get: { viewModel.declEmotionalConfidence },
                        set: { viewModel.declEmotionalConfidence = $0 },
                    ),
                )
            }
        }
    }

    private func scaleRow(title: String, labels: [String], value: Binding<Int>) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(title.uppercased())
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            HStack(spacing: 7) {
                ForEach(1 ... 5, id: \.self) { i in
                    Button {
                        value.wrappedValue = i
                    } label: {
                        VStack(spacing: 2) {
                            Text("\(i)")
                                .font(BarDS.monoFont(15, weight: .medium))
                                .foregroundColor(value.wrappedValue == i ? BarDS.Accent.teal : BarDS.Text.primary)
                            Text(labels[i - 1])
                                .font(BarDS.bodyFont(10, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 9)
                        .background(value.wrappedValue == i ? BarDS.Accent.teal.opacity(0.06) : Color.clear)
                        .overlay(
                            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                                .stroke(
                                    value.wrappedValue == i ? BarDS.Accent.teal.opacity(0.35) : BarDS.Border.card,
                                    lineWidth: BarDS.borderThin,
                                ),
                        )
                    }
                    .buttonStyle(.plain)
                }
            }
        }
    }

    // MARK: - Ticket / numbers

    @ViewBuilder
    private var numbersFields: some View {
        VStack(alignment: .leading, spacing: 9) {
            if !showsVenueTicket {
                symbolField
                if !isOptions {
                    labeledField("Quantity", placeholder: "Quantity", text: $quantityText)
                    Text(quantityNote)
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                        .fixedSize(horizontal: false, vertical: true)
                    HStack(spacing: 7) {
                        BarChip(label: "Buy", selected: sideBuy) { sideBuy = true }
                        BarChip(label: "Sell", selected: !sideBuy) { sideBuy = false }
                    }
                }
            } else {
                symbolField
            }
            if let ltpErr = viewModel.barLtpFetchError, !ltpErr.isEmpty {
                Text(ltpErr)
                    .font(BarDS.bodyFont(11, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
            }
            labeledField("Entry — \(unit)", placeholder: "Entry price", text: $viewModel.declEntryPrice)
            labeledField("Stop — exact price", placeholder: "Stop loss — exact price", text: $stopLossText)
            labeledField("Target", placeholder: "Target price", text: $targetPriceText)
            HStack {
                Text("Max planned loss")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                Text(maxLossText)
                    .font(BarDS.monoFont(12, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
            }
            if !isOptions, let r = rrRatio {
                HStack {
                    Text("Risk : Reward")
                        .font(BarDS.bodyFont(12, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
                    Spacer(minLength: 8)
                    Text(String(format: "1 : %.1f", r))
                        .font(BarDS.monoFont(12, weight: .medium))
                        .foregroundColor(r >= 2.0 ? BarDS.Accent.green : BarDS.Text.primary)
                }
            }
            if viewModel.declareAssetClass == .spot || viewModel.declareAssetClass == .equity {
                BarChip(
                    label: "Protective SL on fill",
                    selected: viewModel.declProtectiveSLConsent,
                ) {
                    viewModel.declProtectiveSLConsent.toggle()
                }
            } else if isNamedFutures {
                Text("Auto-place SL on USDM/Coin-M stays dark · no POST STOP_MARKET")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        }
    }

    private var quantityNote: String {
        viewModel.declareAssetClass == .equity
            ? "No lot. No NRML. DualNoBlend INR."
            : "No lot. DualNoBlend \(unit). Not a dated option."
    }

    private var maxLossText: String {
        if let loss = planMaxLoss {
            return String(format: "%.1f %@", loss, unit)
        }
        return "—"
    }

    private var planUnits: Double? {
        if isOptions {
            if let first = viewModel.optionLegs.first { return Double(first.lots) }
            let t = viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)
            guard let lots = Int(t), lots > 0 else { return nil }
            return Double(lots)
        }
        guard let q = Double(quantityText.trimmingCharacters(in: .whitespacesAndNewlines)), q > 0 else {
            return nil
        }
        return q
    }

    private var planMaxLoss: Double? {
        BarPlanLadder.maxPlannedLossINR(
            units: planUnits,
            entry: Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)),
            stop: Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)),
            sideBuy: sideBuy,
        )
    }

    private var rrRatio: Double? {
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

    // MARK: - Options leftover

    private var optionsFields: some View {
        VStack(alignment: .leading, spacing: 9) {
            labeledField("Expiry (YYYY-MM-DD)", placeholder: "2026-09-29", text: $viewModel.declOptionExpiry)
            labeledField("Strike", placeholder: "Strike", text: $viewModel.declOptionStrike)
            labeledField("Lots", placeholder: "1", text: $viewModel.declLots)
            labeledField("Max planned loss \(unit)", placeholder: "15000", text: $viewModel.declMaxPlannedLossText)
            HStack(spacing: 7) {
                BarChip(label: "CE", selected: viewModel.declOptionRight == "CE") {
                    viewModel.declOptionRight = "CE"
                }
                BarChip(label: "PE", selected: viewModel.declOptionRight == "PE") {
                    viewModel.declOptionRight = "PE"
                }
                Spacer(minLength: 8)
                BarChip(label: "Buy", selected: sideBuy) { sideBuy = true }
                BarChip(label: "Sell", selected: !sideBuy) { sideBuy = false }
            }
            Button(action: addLeg) {
                Text("＋ Add this leg")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .padding(.vertical, 6)
                    .padding(.horizontal, 13)
                    .overlay(
                        Capsule().stroke(BarDS.Border.chipUnselected, lineWidth: BarDS.borderThin),
                    )
            }
            .buttonStyle(.plain)
            if viewModel.optionLegs.isEmpty {
                Text("No legs yet. Pick strike, side, lots — then add.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            } else {
                ForEach(Array(viewModel.optionLegs.enumerated()), id: \.offset) { index, leg in
                    HStack {
                        Text("\(leg.sideBuy ? "BUY" : "SELL") \(leg.underlying) \(leg.strike) \(leg.right)")
                            .font(BarDS.monoFont(11, weight: .medium))
                            .foregroundColor(BarDS.Text.primary)
                        Spacer()
                        Button {
                            viewModel.optionLegs.remove(at: index)
                        } label: {
                            Text("×")
                                .foregroundColor(BarDS.Text.muted)
                        }
                        .buttonStyle(.plain)
                    }
                }
            }
        }
    }

    private func addLeg() {
        let lots = Int(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)) ?? 0
        let strike = viewModel.declOptionStrike.trimmingCharacters(in: .whitespacesAndNewlines)
        let und = viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard lots > 0, !strike.isEmpty, !und.isEmpty else { return }
        viewModel.barDeclarationSymbol = und
        viewModel.optionLegs.append(
            BarIntradayDeclarationPayload.OptionLeg(
                underlying: und,
                expiry: viewModel.declOptionExpiry.trimmingCharacters(in: .whitespacesAndNewlines),
                strike: strike,
                right: viewModel.declOptionRight == "PE" ? "PE" : "CE",
                sideBuy: sideBuy,
                lots: lots,
            ),
        )
    }

    // MARK: - Setup / invalidation

    private var setupChipsRow: some View {
        BarFlowLayout(spacing: 5, rowSpacing: 5) {
            ForEach(Self.setupChips, id: \.rawValue) { chip in
                BarChip(label: chip.rawValue, selected: viewModel.declSetupType == chip.rawValue) {
                    viewModel.declSetupType = chip.rawValue
                }
            }
        }
    }

    private var selectedInvalidationKind: BarInvalidationKind? {
        let t = viewModel.declInvalidationType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        return BarInvalidationKind(rawValue: t)
    }

    private var invalidationKindRow: some View {
        HStack(spacing: 7) {
            ForEach(BarInvalidationKind.allCases) { k in
                let on = selectedInvalidationKind == k
                Button {
                    viewModel.declInvalidationType = k.rawValue
                } label: {
                    VStack(spacing: 4) {
                        invalidationIcon(for: k)
                            .font(.system(size: 14))
                            .foregroundColor(on ? BarDS.Accent.blue : BarDS.Text.secondary)
                        Text(k.chipTitle)
                            .font(BarDS.bodyFont(11, weight: .regular))
                            .foregroundColor(on ? BarDS.Accent.blue : BarDS.Text.secondary)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 8)
                    .background(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .fill(on ? BarDS.Accent.blueFill : Color.white.opacity(0.03)),
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 6, style: .continuous)
                            .stroke(on ? BarDS.Accent.blueBorder : Color.white.opacity(0.10), lineWidth: 0.5),
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

    private var premortem: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("Premortem")
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            ZStack(alignment: .topLeading) {
                if viewModel.declInvalidationCondition.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                    Text("It failed because…")
                        .font(BarDS.bodyFont(12, weight: .regular))
                        .foregroundColor(BarDS.Text.hint)
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
            .padding(8)
            .background(BarDS.Fill.elevated)
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(BarDS.Border.input, lineWidth: BarDS.borderThin),
            )
        }
    }

    // MARK: - Confirm

    private var confirmBar: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let e = viewModel.barDeclarationLastError, !e.isEmpty {
                Text(e)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.red)
            }
            BarBigButton(
                label: viewModel.barDeclarationBusy ? "Submitting…" : "Confirm — enter trade",
                style: .primary,
                action: onConfirm,
            )
            .disabled(!submitReady || viewModel.barDeclarationBusy || !viewModel.showsConfirmControl)
            .opacity(submitReady && !viewModel.barDeclarationBusy ? 1 : 0.3)
            if !submitReady, let hint = submitHint, !viewModel.barDeclarationBusy {
                Text(hint)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .padding(.top, 4)
    }

    // MARK: - Symbol

    private var symbolField: some View {
        VStack(alignment: .leading, spacing: 0) {
            VStack(alignment: .leading, spacing: 5) {
                Text("SYMBOL")
                    .font(BarDS.monoFont(9.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                BarInputField(
                    placeholder: "BTC or RELIANCE",
                    text: $viewModel.barDeclarationSymbol,
                    marginBottom: 0,
                    onCommit: { viewModel.commitDeskSymbol() },
                )
            }
            .onChange(of: viewModel.barDeclarationSymbol) { _, newValue in
                viewModel.searchSymbols(newValue)
            }
            if viewModel.showSymbolSuggestions {
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
                        Divider()
                    }
                }
                .background(.regularMaterial)
                .clipShape(RoundedRectangle(cornerRadius: 6))
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

    // MARK: - chrome

    private func groupLab(_ text: String) -> some View {
        Text(text.uppercased())
            .font(BarDS.monoFont(10, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
            .kerning(1.4)
    }

    private func labeledField(_ label: String, placeholder: String, text: Binding<String>) -> some View {
        VStack(alignment: .leading, spacing: 5) {
            Text(label.uppercased())
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            BarInputField(placeholder: placeholder, text: text, marginBottom: 0)
        }
    }
}
