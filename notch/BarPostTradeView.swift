import SwiftUI

private enum AdherenceTri: Equatable {
    case unset, yes, no

    mutating func cycle() {
        switch self {
        case .unset: self = .yes
        case .yes: self = .no
        case .no: self = .unset
        }
    }
}

private struct AdherenceTriState: Equatable {
    var stop: AdherenceTri = .unset
    var size: AdherenceTri = .unset
    var invalidation: AdherenceTri = .unset
    var exit: AdherenceTri = .unset
    var impulsive: AdherenceTri = .unset

    mutating func cycle(_ keyPath: WritableKeyPath<AdherenceTriState, AdherenceTri>) {
        var v = self[keyPath: keyPath]
        v.cycle()
        self[keyPath: keyPath] = v
    }
}

/// Post-trade PLAN debrief — timed moment A, adherence toggles (B), reflection (C), PATCH ingest (#5).
struct BarPostTradeView: View {
    @ObservedObject var viewModel: NotchViewModel
    @State private var postTrade = BarPostTradeState.initial
    @State private var momentABase: Date = Date()
    @State private var noteA: String = ""
    @State private var adherenceTri = AdherenceTriState()

    @State private var momentCIsCooling: Bool = true
    @State private var noteC: String = ""
    @State private var tick: Date = Date()

    private let momentASeconds: TimeInterval = 120

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 16) {
                Text("Trade closed")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                    .foregroundColor(BarDS.Text.primary)

                Text("Three beats — finish in order. The last step saves your debrief to TradeAutopsy.")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)

                momentA
                momentB
                momentC
                if postTrade.fidelityRingVisible {
                    fidelitySection
                    submitRow
                }

                Button("Dismiss (no save)") {
                    viewModel.dismissBarDebrief()
                }
                .buttonStyle(.plain)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.hint)
                .padding(.top, 4)
            }
        }
        .onAppear {
            postTrade = .initial
            momentABase = Date()
            noteA = ""
            adherenceTri = AdherenceTriState()
            momentCIsCooling = true
            noteC = ""
            tick = Date()
        }
        .onReceive(Timer.publish(every: 1, on: .main, in: .common).autoconnect()) { tick = $0 }
    }

    private var adherence: BarPostTradeAdherenceAnswers {
        BarPostTradeAdherenceAnswers(
            stopAsDeclared: adherenceTri.stop == .yes,
            sizeAsDeclared: adherenceTri.size == .yes,
            invalidationRespected: adherenceTri.invalidation == .yes,
            exitPerPlan: adherenceTri.exit == .yes,
            noImpulsiveAdd: adherenceTri.impulsive == .yes,
        )
    }

    private var adherenceAllYes: Bool {
        adherenceTri.stop == .yes && adherenceTri.size == .yes && adherenceTri.invalidation == .yes
            && adherenceTri.exit == .yes && adherenceTri.impulsive == .yes
    }

    private var adherenceAnsweredCount: Int {
        let a = [adherenceTri.stop, adherenceTri.size, adherenceTri.invalidation, adherenceTri.exit, adherenceTri.impulsive]
        return a.filter { $0 != .unset }.count
    }

    private var adherenceNoCount: Int {
        let a = [adherenceTri.stop, adherenceTri.size, adherenceTri.invalidation, adherenceTri.exit, adherenceTri.impulsive]
        return a.filter { $0 == .no }.count
    }

    private var momentAElapsed: TimeInterval {
        tick.timeIntervalSince(momentABase)
    }

    private var momentACanContinue: Bool {
        postTrade.momentAAcknowledged || momentAElapsed >= momentASeconds
    }

    private var momentA: some View {
        BarCard {
            Text("Moment A — outcome")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .bold))
                .foregroundColor(postTrade.momentAAcknowledged ? BarDS.Accent.teal : BarDS.Text.primary)
                .padding(.bottom, 8)

            Text("Win or loss aside: did the market prove your thesis wrong, or did you exit early?")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 8)

            if !postTrade.momentAAcknowledged {
                let remain = max(0, momentASeconds - momentAElapsed)
                let mm = Int(remain) / 60
                let ss = Int(remain) % 60
                Text("Read window \(String(format: "%d:%02d", mm, ss))")
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .semibold))
                    .foregroundColor(BarDS.Accent.teal.opacity(0.9))
                    .accessibilityLabel("Moment A countdown \(mm) minutes \(ss) seconds")
                    .padding(.bottom, 8)

                BarInputField(placeholder: "Optional note", text: $noteA)
                    .padding(.bottom, 4)

                tealContinueButton(title: "Continue", enabled: momentACanContinue) {
                    postTrade = BarPostTradeReducer.reduce(state: postTrade, action: .acknowledgeMomentA)
                }
                .disabled(!momentACanContinue)
            } else {
                Text("Recorded")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
    }

    private var momentB: some View {
        let canAct = postTrade.momentAAcknowledged
        let allOn = adherenceAllYes
        return BarCard {
            Text("Moment B — process")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .bold))
                .foregroundColor(postTrade.momentBAcknowledged ? BarDS.Accent.teal : BarDS.Text.primary)
                .padding(.bottom, 8)

            Text("Affirm each line — only honest yes advances.")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 8)

            if !postTrade.momentBAcknowledged {
                adherenceCycleRow("Stop as declared", "Matches your declared stop for this plan.", \.stop, canAct)
                adherenceCycleRow("Size as declared", "No drift vs declared size.", \.size, canAct)
                adherenceCycleRow("Invalidation respected", "You honored the invalidation you wrote.", \.invalidation, canAct)
                adherenceCycleRow("Exit per plan", "Exit matched thesis / risk plan.", \.exit, canAct)
                adherenceCycleRow("No impulsive add / scale-in", "No unplanned adds while stressed.", \.impulsive, canAct)

                if adherenceAnsweredCount >= 3 {
                    adherenceProcessBadge
                        .padding(.vertical, 6)
                }

                tealContinueButton(title: "Continue", enabled: canAct && allOn) {
                    postTrade = BarPostTradeReducer.reduce(state: postTrade, action: .acknowledgeMomentB)
                }
                .disabled(!canAct || !allOn)
            } else {
                Text("Recorded")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
        .opacity(canAct || postTrade.momentBAcknowledged ? 1 : 0.45)
    }

    private var adherenceProcessBadge: some View {
        let title: String
        let color: Color
        if adherenceNoCount >= 2 {
            title = "Broke rules"
            color = BarDS.Accent.red
        } else if adherenceNoCount == 1 {
            title = "Mixed process"
            color = BarDS.Accent.amber
        } else {
            title = "Good process"
            color = BarDS.Accent.teal
        }
        return Text(title)
            .font(BarDS.bodyFont(11, weight: .semibold))
            .foregroundColor(color)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(8)
            .background(color.opacity(0.1))
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
    }

    private func adherenceCycleRow(
        _ title: String,
        _ sub: String,
        _ path: WritableKeyPath<AdherenceTriState, AdherenceTri>,
        _ canAct: Bool,
    ) -> some View {
        let v = adherenceTri[keyPath: path]
        return Button {
            guard canAct else { return }
            adherenceTri.cycle(path)
        } label: {
            HStack(alignment: .top, spacing: 10) {
                Group {
                    switch v {
                    case .unset:
                        Image(systemName: "circle")
                            .foregroundColor(BarDS.Text.hint)
                    case .yes:
                        Image(systemName: "checkmark.circle.fill")
                            .foregroundColor(BarDS.Accent.teal)
                    case .no:
                        Image(systemName: "xmark.circle.fill")
                            .foregroundColor(BarDS.Accent.red)
                    }
                }
                .font(.system(size: 16))
                .frame(width: 22, alignment: .center)

                VStack(alignment: .leading, spacing: 3) {
                    Text(title)
                        .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                        .foregroundColor(BarDS.Text.primary)
                    Text(sub)
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                        .foregroundColor(BarDS.Text.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
                Spacer(minLength: 0)
            }
            .padding(.vertical, 6)
        }
        .buttonStyle(.plain)
        .disabled(!canAct)
        .opacity(canAct ? 1 : 0.55)
    }

    private var momentC: some View {
        let canAct = postTrade.momentBAcknowledged
        return BarCard {
            Text("Moment C — one sentence")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .bold))
                .foregroundColor(postTrade.momentCAcknowledged ? BarDS.Accent.teal : BarDS.Text.primary)
                .padding(.bottom, 8)

            Picker("Context", selection: $momentCIsCooling) {
                Text("Cooling / scratch").tag(true)
                Text("Clean win").tag(false)
            }
            .pickerStyle(.segmented)
            .disabled(!canAct)
            .tint(BarDS.Accent.teal)
            .padding(.bottom, 8)

            Text("One line: what will you do differently next time in the same emotional state?")
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 8)

            if !postTrade.momentCAcknowledged {
                BarInputField(placeholder: "Required note", text: $noteC)
                    .disabled(!canAct)
                    .opacity(canAct ? 1 : 0.55)
                    .padding(.bottom, 4)

                let noteOk = !noteC.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                tealContinueButton(title: "Continue", enabled: canAct && noteOk) {
                    postTrade = BarPostTradeReducer.reduce(state: postTrade, action: .acknowledgeMomentC)
                }
                .disabled(!canAct || !noteOk)
            } else {
                Text("Recorded")
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
        .opacity(canAct || postTrade.momentCAcknowledged ? 1 : 0.45)
    }

    private var fidelitySection: some View {
        VStack(alignment: .leading, spacing: 10) {
            Text("Plan fidelity")
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .bold))
                .foregroundColor(BarDS.Text.primary)

            Text("You finished all three moments. Submit to lock this debrief into your behavioral stream.")
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.hint)
                .fixedSize(horizontal: false, vertical: true)

            ZStack {
                Circle()
                    .stroke(Color.white.opacity(0.12), lineWidth: 4)
                    .frame(width: 64, height: 64)
                Circle()
                    .trim(from: 0, to: 1)
                    .stroke(BarDS.Accent.teal.opacity(0.88), style: StrokeStyle(lineWidth: 4, lineCap: .round))
                    .frame(width: 64, height: 64)
                    .rotationEffect(.degrees(-90))
            }
            .accessibilityLabel("Fidelity ring complete")
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .background(BarDS.Semantic.tealBg())
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Semantic.tealBorder(), lineWidth: BarDS.borderThin),
        )
        .padding(.bottom, 8)
    }

    private var submitRow: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let err = viewModel.barPostTradeDebriefLastError, !err.isEmpty {
                Text(err)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.amber)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Button {
                let declFromMatch = viewModel.barLiveState?.matchedDeclarationId?
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                let declPending = viewModel.barLiveState?.pendingDeclaration?.id
                    .trimmingCharacters(in: .whitespacesAndNewlines)
                let declId: String? = {
                    if let m = declFromMatch, !m.isEmpty { return m }
                    if let p = declPending, !p.isEmpty { return p }
                    return nil
                }()
                let payload = BarPostTradeDebriefPayload.buildJSONObject(
                    momentANote: noteA.trimmingCharacters(in: .whitespacesAndNewlines),
                    adherence: adherence,
                    momentCContext: momentCIsCooling ? "cooling" : "win",
                    momentCNote: noteC.trimmingCharacters(in: .whitespacesAndNewlines),
                    declarationId: declId,
                    completedAtMs: Int(Date().timeIntervalSince1970 * 1000),
                )
                guard let data = try? JSONSerialization.data(withJSONObject: payload) else { return }
                Task { await viewModel.submitBarPostTradeDebrief(body: data) }
            } label: {
                HStack(spacing: 8) {
                    if viewModel.barPostTradeDebriefBusy {
                        ProgressView().scaleEffect(0.7)
                    }
                    Text(viewModel.barPostTradeDebriefBusy ? "Saving…" : "Submit debrief")
                        .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                }
                .foregroundColor(BarDS.Fill.sidebar)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 10)
                .background(BarDS.Accent.teal)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
            }
            .buttonStyle(.plain)
            .disabled(viewModel.barPostTradeDebriefBusy)
        }
    }

    private func tealContinueButton(title: String, enabled: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(title)
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .semibold))
                .foregroundColor(BarDS.Fill.sidebar)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 10)
                .background(enabled ? BarDS.Accent.teal : Color.white.opacity(0.2))
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        }
        .buttonStyle(.plain)
    }
}
