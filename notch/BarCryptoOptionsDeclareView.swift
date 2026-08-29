import SwiftUI

/// Three-zone Options Pre-trade for the Binance crypto desk (dated European contracts,
/// `BTC-260925-90000-C`). Money is USDT; there is no lot size, no NRML, no product code.
/// Chain / OI / Greeks / σ / history stay honest-dark — Binance sends no `book=`, so the
/// options glance is skipped. Last is the one live strip. Rung 1 runs on typed numbers.
struct BarCryptoOptionsDeclareView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 12) {
            analyticsZone
            consequenceZone
            planAndConfirm
        }
    }

    // MARK: Zone A

    private var analyticsZone: some View {
        optionsZone(
            title: "Analytics",
            note: "market/quote · market/option_chain · derived/ohlcv",
        ) {
            VStack(alignment: .leading, spacing: 12) {
                panelHead("Session", trailing: sessionTrailing)
                lastStrip
                sessionChartHole
                Text("History · the ohlcv obtain is licensed to the Kotak desk. Binance session bars need a live TickBook.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                panelHead("Chain · around your strike", trailing: chainTrailing)
                chainHost
            }
        }
    }

    /// Only vocabulary on this strip is the desk's own wire status — nothing is hardcoded.
    private var sessionTrailing: String {
        HonestyStatus.fromWire(viewModel.deskLastStatus) == .unavailable
            || viewModel.deskLastStatus.lowercased() == "unavailable"
            ? "quote unavailable"
            : viewModel.deskLastStatus
    }

    /// Last is live on this desk. The number shown is the entry the desk prefilled or you
    /// typed — never a second quote invented here.
    private var lastStrip: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Last · your entry")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                Text("market/quote · prefills the entry premium while the desk quotes")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 8)
            if let entry = entryPrice {
                Text(usdt(entry))
                    .font(BarDS.monoFont(15, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
            } else if let honesty = HonestyStatus.fromWire(viewModel.deskLastStatus) {
                HonestyChip(status: honesty)
            } else {
                Text(viewModel.deskLastStatus)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
        .padding(11)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var sessionChartHole: some View {
        VStack(spacing: 6) {
            Text("no session series")
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text("market/ohlcv · no Binance obtain in this slice")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
        .frame(maxWidth: .infinity, minHeight: 88)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var chainPresentation: BarOptionsChainPresentation {
        BarOptionsChainPresentation.from(
            underlying: viewModel.barDeclarationSymbol,
            chainStatus: viewModel.deskChainStatus,
        )
    }

    private var chainTrailing: String {
        switch chainPresentation {
        case .nothingDeclared: return "—"
        case .unavailable: return "unavailable"
        case .empty: return "empty"
        case .lit: return viewModel.deskChainStatus
        }
    }

    @ViewBuilder
    private var chainHost: some View {
        switch chainPresentation {
        case .nothingDeclared:
            emptyBlock(title: "Nothing declared yet", body: "Type an underlying and the chain around your strike appears here.")
        case .unavailable:
            VStack(alignment: .leading, spacing: 8) {
                HonestyChip(status: .unavailable)
                Text("BoundedSnapshot is complete-or-refused. Binance names no catalog book, so the options glance is skipped and no strike grid can be drawn.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
        case .empty:
            emptyBlock(title: "No contracts", body: "Expiry list is empty — not a guessed strike grid.")
        case .lit:
            emptyBlock(
                title: "chain snapshot live · no strike grid (raw strike/expiry)",
                body: "Rows come from the Binance options catalog. A missing row means that contract is absent.",
            )
        }
    }

    // MARK: Zone B

    private var consequenceZone: some View {
        optionsZone(
            title: "Consequence",
            note: "derived/greeks · inherits chain + Binance options catalog",
        ) {
            VStack(alignment: .leading, spacing: 12) {
                panelHead("Position", trailing: netPremiumLine)
                legsHost
                greeksGrid
                Text(greeksProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                panelHead("At expiry", trailing: "scale anchored to your declared limit")
                payoffHole

                ladderHeader
                ladderRungs
                marginRow
                Text(ladderProv)
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                legendRow
            }
        }
    }

    private var hasLegs: Bool { !viewModel.optionLegs.isEmpty }

    private var netPremiumLine: String { "premium unavailable" }

    @ViewBuilder
    private var legsHost: some View {
        if viewModel.optionLegs.isEmpty {
            emptyBlock(title: "No legs yet", body: "Pick a strike, a side, and a number of contracts — then add the leg. The ladder below fills as you go.")
        } else {
            VStack(spacing: 7) {
                ForEach(Array(viewModel.optionLegs.enumerated()), id: \.offset) { index, leg in
                    HStack(spacing: 10) {
                        Text(leg.sideBuy ? "BUY" : "SELL")
                            .font(BarDS.monoFont(10, weight: .bold))
                            .foregroundColor(leg.sideBuy ? BarDS.Accent.teal : BarDS.Accent.red)
                            .padding(.vertical, 3)
                            .padding(.horizontal, 7)
                            .background((leg.sideBuy ? BarDS.Accent.teal : BarDS.Accent.red).opacity(0.10))
                            .clipShape(RoundedRectangle(cornerRadius: 5, style: .continuous))
                        VStack(alignment: .leading, spacing: 2) {
                            Text("\(leg.underlying) \(leg.strike) \(rightWord(leg.right))")
                                .font(BarDS.monoFont(12, weight: .medium))
                                .foregroundColor(BarDS.Text.primary)
                            Text("\(leg.lots) contract\(leg.lots == 1 ? "" : "s") · \(leg.expiry.isEmpty ? "expiry unset" : leg.expiry)")
                                .font(BarDS.monoFont(10, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        Spacer(minLength: 8)
                        Text("px unavailable")
                            .font(BarDS.monoFont(11, weight: .medium))
                            .foregroundColor(BarDS.Accent.red)
                        Button {
                            viewModel.optionLegs.remove(at: index)
                        } label: {
                            Text("×")
                                .font(BarDS.bodyFont(15, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Remove leg")
                    }
                    .padding(.vertical, 9)
                    .padding(.horizontal, 11)
                    .background(BarDS.Fill.elevated)
                    .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
                    .overlay(
                        RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                            .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
                    )
                }
            }
        }
    }

    /// Wire keeps the `CE`/`PE` vocabulary; the crypto surface never prints an NFO code.
    private func rightWord(_ right: String) -> String {
        right == "PE" ? "Put" : "Call"
    }

    private var greeksGrid: some View {
        LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())], spacing: 1) {
            ForEach(["Delta", "Gamma", "Theta / day", "Vega / 1 vol"], id: \.self) { label in
                VStack(alignment: .leading, spacing: 4) {
                    Text(label.uppercased())
                        .font(BarDS.monoFont(9.5, weight: .regular))
                        .foregroundColor(BarDS.Text.muted)
                    HonestyChip(status: hasLegs ? .inheritedDark : .empty)
                }
                .padding(8)
                .frame(maxWidth: .infinity, alignment: .leading)
                .background(BarDS.Fill.elevated)
            }
        }
        .background(BarDS.Border.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
    }

    private var greeksProv: String {
        if hasLegs {
            return "derived/greeks · inherited dark — named input market/option_chain is skipped on the Binance desk (no catalog book). A Greek without the chain is a lie, so this stays empty."
        }
        return "derived/greeks — waiting on a declared leg."
    }

    private var payoffHole: some View {
        VStack(spacing: 6) {
            Text("chain unavailable")
                .font(BarDS.monoFont(11, weight: .medium))
                .foregroundColor(BarDS.Accent.red)
            Text("derived/payoff inherits market/option_chain")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
        .frame(maxWidth: .infinity, minHeight: 88)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var declaredMax: Double? {
        Double(viewModel.declMaxPlannedLossText.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    private var entryPrice: Double? {
        Double(viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines))
    }

    /// Contracts, never lots — Binance options carry no lot size.
    private var planUnits: Double? {
        if let first = viewModel.optionLegs.first {
            return Double(first.lots)
        }
        let t = viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let contracts = Int(t), contracts > 0 else { return nil }
        return Double(contracts)
    }

    /// Same shape as the NFO rung 1 — typed numbers, else the declared limit as the anchor.
    /// Kept local so no INR-named helper renders a USDT figure.
    private var stopRung: Double? {
        if let rung = BarPlanLadder.rung1(
            units: planUnits,
            entry: entryPrice,
            stop: Double(stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)),
            sideBuy: sideBuy,
        ) {
            return rung
        }
        if let declaredMax {
            return -abs(declaredMax)
        }
        return nil
    }

    private var ladderHeader: some View {
        VStack(alignment: .leading, spacing: 4) {
            if let rung = stopRung {
                Text(usdt(rung))
                    .font(BarDS.monoFont(27, weight: .medium))
                    .foregroundColor(overLimit(rung) ? BarDS.Accent.red : BarDS.Text.primary)
                Text(worstCap(rung))
                    .font(BarDS.bodyFont(11.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            } else {
                Text("—")
                    .font(BarDS.monoFont(27, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
                Text(hasLegs
                    ? "The chain is a hole, so only the rung you typed can light. That is the honest answer."
                    : "Add a leg and a contract count to see what this can cost you.")
                    .font(BarDS.bodyFont(11.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Text("horizon · \(horizonNote)")
                .font(BarDS.monoFont(10.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
    }

    private func overLimit(_ rung: Double) -> Bool {
        guard let max = declaredMax else { return false }
        return abs(rung) > max
    }

    private func worstCap(_ rung: Double) -> String {
        if let max = declaredMax, abs(rung) > max {
            return String(format: "worst case %@ — %.1f× the loss you planned for", horizonPhrase, abs(rung) / max)
        }
        if declaredMax != nil {
            return "worst case \(horizonPhrase) — inside the limit you declared"
        }
        return "Set a stop and a max planned loss to light the first rung."
    }

    /// Crypto trades round the clock — the horizon counts days, not exchange sessions.
    private var horizonPhrase: String {
        viewModel.declHorizonDays == 1 ? "in the next 24h" : "over \(viewModel.declHorizonDays) days"
    }

    private var horizonNote: String {
        let days = viewModel.declHorizonDays
        let left = days == 1 ? "next 24h" : "\(days) days"
        return "\(left) · σ unavailable"
    }

    /// Rung 1 is typed. Everything below it needs the chain, so it stays dark.
    private var ladderRungList: [BarCryptoLadderRung] {
        let stopTrim = stopLossText.trimmingCharacters(in: .whitespacesAndNewlines)
        let stopSub = stopTrim.isEmpty
            ? "set a stop premium to light this rung"
            : "premium at \(stopTrim) USDT · typed, no market data needed"
        let rung1 = hasLegs ? stopRung : nil
        return [
            BarCryptoLadderRung(
                label: "At your stop",
                sub: stopSub,
                amountUSDT: rung1,
                honesty: rung1 == nil ? .empty : nil,
            ),
            BarCryptoLadderRung(
                label: "1σ adverse \(horizonPhrase)",
                sub: "σ scaled from chain IV",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "2σ / gap \(horizonPhrase)",
                sub: "σ scaled from chain IV",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "Terminal at expiry",
                sub: "payoff needs premiums from the chain",
                amountUSDT: nil,
                honesty: .unavailable,
            ),
            BarCryptoLadderRung(
                label: "Distance to breach your limit",
                sub: declaredMax == nil ? "set a max planned loss" : "needs Greeks to solve for the move",
                amountUSDT: nil,
                honesty: declaredMax == nil ? .empty : .unavailable,
            ),
        ]
    }

    private var ladderRungs: some View {
        VStack(spacing: 0) {
            if !hasLegs {
                emptyBlock(title: "The ladder is empty", body: "Every rung below fills from something you declare. Nothing is guessed for you.")
                    .padding(.vertical, 8)
            } else {
                ForEach(Array(ladderRungList.enumerated()), id: \.offset) { _, rung in
                    ladderRungRow(rung)
                }
            }
        }
    }

    private func ladderRungRow(_ rung: BarCryptoLadderRung) -> some View {
        HStack(alignment: .center, spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(rung.label)
                    .font(BarDS.bodyFont(12, weight: .medium))
                    .foregroundColor(rung.honesty == nil ? BarDS.Text.primary : BarDS.Text.muted)
                Text(rung.sub)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 8)
            if let amt = rung.amountUSDT {
                VStack(alignment: .trailing, spacing: 2) {
                    Text(usdt(amt))
                        .font(BarDS.monoFont(15, weight: .medium))
                        .foregroundColor(overLimit(amt) ? BarDS.Accent.red : BarDS.Text.primary)
                    if let max = declaredMax, max > 0 {
                        Text(String(format: "%.1f× declared", abs(amt) / max))
                            .font(BarDS.monoFont(10.5, weight: .regular))
                            .foregroundColor(BarDS.Text.muted)
                    }
                }
            } else if let honesty = rung.honesty {
                HonestyChip(status: honesty)
            }
        }
        .padding(.vertical, 9)
        .overlay(alignment: .bottom) {
            Rectangle().fill(BarDS.Border.row).frame(height: 0.5)
        }
    }

    private var marginRow: some View {
        HStack(alignment: .center, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text("Margin blocked")
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                Text("the venue's margin engine is Binance's number, never ours")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            Spacer(minLength: 8)
            HonestyChip(status: .unavailable)
        }
        .padding(11)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private var ladderProv: String {
        "rung 1 · your typed numbers only. Rungs 2–5 stay dark — σ / IV / greeks unspecified."
    }

    private var legendRow: some View {
        VStack(alignment: .leading, spacing: 4) {
            legendItem(BarDS.Accent.teal.opacity(0.35), "within your declared limit")
            legendItem(BarDS.Accent.red.opacity(0.45), "past your declared limit")
            legendItem(nil, "inherited dark — waiting on an input", dashed: true)
            legendItem(BarDS.Accent.red.opacity(0.4), "unavailable — source refused")
        }
        .padding(.top, 4)
    }

    private func legendItem(_ fill: Color?, _ label: String, dashed: Bool = false) -> some View {
        HStack(spacing: 6) {
            RoundedRectangle(cornerRadius: 2)
                .stroke(BarDS.Text.muted, style: StrokeStyle(lineWidth: 1, dash: dashed ? [3, 2] : []))
                .background(fill ?? Color.clear)
                .frame(width: 9, height: 9)
            Text(label)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
    }

    // MARK: Zone C

    private var planAndConfirm: some View {
        VStack(alignment: .leading, spacing: 12) {
            planZone
            confirmBar
        }
    }

    private var planZone: some View {
        optionsZone(title: "Plan", note: "what you are declaring") {
            VStack(alignment: .leading, spacing: 16) {
                groupLab("State check")
                stateCheck

                groupLab("Contract")
                contractFields
                contractPills

                groupLab("Risk")
                riskFields

                groupLab("Horizon — how far ahead the σ rungs look")
                horizonPills
                Text("Set by your style chip. Edit it and every σ rung recomputes — the horizon is part of the number, not a setting.")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .fixedSize(horizontal: false, vertical: true)

                groupLab("Invalidation")
                premortem
            }
        }
    }

    private var stateCheck: some View {
        let calm = viewModel.declEmotionalCalm
        let conf = viewModel.declEmotionalConfidence
        if (1 ... 5).contains(calm), (1 ... 5).contains(conf) {
            let cw = ["Calm", "Focused", "Tense", "Anxious", "Angry"][calm - 1]
            let cf = ["Low", "Flat", "Neutral", "Good", "Sharp"][conf - 1]
            return AnyView(
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
            )
        }
        return AnyView(
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
                scaleRow(
                    title: "Confidence — 5 is best",
                    labels: ["Low", "Flat", "Neutral", "Good", "Sharp"],
                    value: Binding(
                        get: { viewModel.declEmotionalConfidence },
                        set: { viewModel.declEmotionalConfidence = $0 },
                    ),
                )
            }
        )
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

    private var contractFields: some View {
        VStack(spacing: 9) {
            labeledField("Underlying", placeholder: "BTC", text: $viewModel.barDeclarationSymbol)
            HStack(spacing: 9) {
                labeledField("Expiry · YYMMDD", placeholder: "260925", text: $viewModel.declOptionExpiry)
                labeledField("Strike", placeholder: "90000", text: $viewModel.declOptionStrike)
            }
            labeledField("Contracts", placeholder: "1", text: $viewModel.declLots)
            Text("Binance sizes in contracts and settles in USDT — no lot size, no product code.")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private var contractPills: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 7) {
                BarChip(label: "Call", selected: viewModel.declOptionRight == "CE") {
                    viewModel.declOptionRight = "CE"
                }
                BarChip(label: "Put", selected: viewModel.declOptionRight == "PE") {
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
        }
    }

    /// `lots` is the payload's size field; on this desk it carries contracts.
    private func addLeg() {
        let contracts = Int(viewModel.declLots.trimmingCharacters(in: .whitespacesAndNewlines)) ?? 0
        let strike = viewModel.declOptionStrike.trimmingCharacters(in: .whitespacesAndNewlines)
        let und = viewModel.barDeclarationSymbol.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        guard contracts > 0, !strike.isEmpty, !und.isEmpty else { return }
        viewModel.barDeclarationSymbol = und
        viewModel.optionLegs.append(
            BarIntradayDeclarationPayload.OptionLeg(
                underlying: und,
                expiry: viewModel.declOptionExpiry.trimmingCharacters(in: .whitespacesAndNewlines),
                strike: strike,
                right: viewModel.declOptionRight == "PE" ? "PE" : "CE",
                sideBuy: sideBuy,
                lots: contracts,
            ),
        )
    }

    private var riskFields: some View {
        VStack(spacing: 9) {
            labeledField("Entry — premium in USDT", placeholder: "0.0012", text: $viewModel.declEntryPrice)
            labeledField("Stop — exact premium in USDT", placeholder: "0.0006", text: $stopLossText)
            labeledField("Target — premium in USDT", placeholder: "0.0030", text: $targetPriceText)
            labeledField("Max planned loss USDT", placeholder: "250", text: $viewModel.declMaxPlannedLossText)
        }
    }

    private var horizonPills: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 7) {
                horizonChip(.today)
                horizonChip(.threeSessions)
                horizonChip(.expiry)
            }
            HStack(spacing: 9) {
                Slider(
                    value: Binding(
                        get: { Double(viewModel.declHorizonDays) },
                        set: {
                            viewModel.declHorizonDays = BarPlanHorizon.clamp(Int($0.rounded()))
                            viewModel.declHorizonMode = BarPlanHorizon.mode(
                                forDays: viewModel.declHorizonDays,
                                dte: nil,
                            )
                        },
                    ),
                    in: Double(BarPlanHorizon.dayRange.lowerBound) ... Double(BarPlanHorizon.dayRange.upperBound),
                    step: 1,
                )
                Text(horizonSliderLabel)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .frame(minWidth: 88, alignment: .trailing)
            }
            HStack(spacing: 6) {
                Text("DTE")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                HonestyChip(status: .unavailable)
            }
        }
    }

    private var horizonSliderLabel: String {
        let d = viewModel.declHorizonDays
        if d == 1 { return "1 day (next 24h)" }
        return "\(d) days"
    }

    private func horizonChip(_ mode: BarPlanHorizon.Mode) -> some View {
        let selected = viewModel.declHorizonMode == mode
        return BarChip(label: mode.chipLabel, selected: selected) {
            viewModel.declHorizonMode = mode
            if let days = BarPlanHorizon.days(for: mode, dte: nil) {
                viewModel.declHorizonDays = days
            }
            // expiry with unknown DTE: chip records intent, days stay as-is
        }
    }

    /// USDT twin of `BarOptionsPremortem` — the shared helper prints ₹.
    private var premortemQuestion: String {
        if hasLegs, let max = declaredMax {
            return "This trade hit your declared limit of \(usdt(max)). What happened?"
        }
        return "Write what would prove this trade wrong."
    }

    private var premortemHint: String {
        if hasLegs, declaredMax != nil {
            return "σ rungs are dark, so this is seeded from your declared limit instead of the chain."
        }
        return "Fill the contract and size, and this question gets sharper."
    }

    private var premortem: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(premortemQuestion)
                .font(BarDS.bodyFont(13.5, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
                .fixedSize(horizontal: false, vertical: true)
            Text(premortemHint)
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
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
        .padding(12)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.subtle, lineWidth: BarDS.borderThin),
        )
    }

    /// Submit is gated by the caller's readiness, the busy flag, and the live kill switch.
    private var submitBlocked: Bool {
        viewModel.barLiveState?.blocksDeclarationSubmit == true
    }

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
            .disabled(!submitReady || viewModel.barDeclarationBusy || submitBlocked)
            .opacity(submitReady && !viewModel.barDeclarationBusy && !submitBlocked ? 1 : 0.3)
            if !submitReady, let hint = submitHint, !viewModel.barDeclarationBusy {
                Text(hint)
                    .font(BarDS.monoFont(10.5, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .frame(maxWidth: .infinity, alignment: .trailing)
            }
        }
        .padding(.top, 4)
    }

    // MARK: money

    /// USDT never rounds a small premium away: `0.001` prints `0.001 USDT`, not `0.00`.
    /// Two places at and above 1, then enough places to keep four significant digits.
    private func usdt(_ value: Double) -> String {
        let magnitude = abs(value)
        var places = 2
        if magnitude > 0, magnitude < 1 {
            places = min(8, max(2, 3 - Int(floor(log10(magnitude)))))
        }
        var text = String(format: "%.\(places)f", magnitude)
        while places > 2, text.hasSuffix("0") {
            text.removeLast()
            places -= 1
        }
        // Below the last fixed place, print the exponent rather than a false zero.
        if magnitude > 0, Double(text) == 0 {
            text = String(format: "%.2e", magnitude)
        }
        return "\(text) USDT"
    }

    // MARK: chrome

    private func optionsZone<Content: View>(
        title: String,
        note: String,
        @ViewBuilder content: () -> Content,
    ) -> some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 10) {
                Text(title.uppercased())
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .kerning(1.6)
                Rectangle().fill(BarDS.Border.card).frame(height: 1)
                Text(note)
                    .font(BarDS.monoFont(11, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
                    .lineLimit(1)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
            content()
                .padding(14)
        }
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func panelHead(_ title: String, trailing: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Text(title)
                .font(BarDS.bodyFont(12, weight: .semibold))
                .foregroundColor(BarDS.Text.secondary)
            Spacer(minLength: 8)
            Text(trailing)
                .font(BarDS.monoFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .lineLimit(2)
                .multilineTextAlignment(.trailing)
        }
    }

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

    private func emptyBlock(title: String, body: String) -> some View {
        VStack(spacing: 3) {
            Text(title)
                .font(BarDS.bodyFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
            Text(body)
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .multilineTextAlignment(.center)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(16)
        .frame(maxWidth: .infinity)
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(style: StrokeStyle(lineWidth: 1, dash: [5, 4]))
                .foregroundColor(BarDS.Border.card),
        )
    }
}

/// Crypto twin of `BarOptionsLadderRung` — same shape, USDT amount instead of ₹.
private struct BarCryptoLadderRung {
    var label: String
    var sub: String
    var amountUSDT: Double?
    var honesty: HonestyStatus?
}
