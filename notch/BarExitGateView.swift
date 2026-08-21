import SwiftUI

/// Exit gate — three sequential yes/no questions, scare-candle mirror, optional verdict cool-off (#123).
struct BarExitGateView: View {
    var onConfirmExit: () -> Void
    var onCancel: () -> Void

    @State private var gateState = BarExitGateState.initial

    private let verdictChips = ["Tilt / revenge", "Single candle fear", "Moved my stop", "Other"]

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("Before you exit")
                .font(.system(size: 15, weight: .bold, design: .rounded))
                .foregroundColor(.white.opacity(0.94))

            gateBody
        }
        .padding(18)
        .frame(maxWidth: 420)
        .onAppear { gateState = .initial }
        .onReceive(Timer.publish(every: 1, on: .main, in: .common).autoconnect()) { _ in
            if case let .coolOffVerdict(remaining, _) = gateState.flow, remaining > 0 {
                gateState = BarExitGateReducer.reduce(state: gateState, action: .tickCountdown)
            }
        }
    }

    @ViewBuilder
    private var gateBody: some View {
        switch gateState.flow {
        case .questions:
            questionsBlock
        case .scareCandleMirror:
            scareCandleBlock
        case .cleanExitReady:
            cleanExitBlock
        case .coolOffVerdict:
            coolOffBlock
        }
    }

    private var questionsBlock: some View {
        VStack(alignment: .leading, spacing: 12) {
            let idx = activeQuestionIndex
            Text(questionPrompt(for: idx))
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.82))
                .fixedSize(horizontal: false, vertical: true)

            HStack(spacing: 10) {
                answerButton(title: "Yes", value: true, index: idx)
                answerButton(title: "No", value: false, index: idx)
            }

            Text("Answer in order — you cannot skip ahead.")
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.38))

            Button("Not now") { onCancel() }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.65))
                .padding(.top, 4)
        }
    }

    private var activeQuestionIndex: Int {
        if gateState.q1 == nil { return 0 }
        if gateState.q2 == nil { return 1 }
        return 2
    }

    private func questionPrompt(for index: Int) -> String {
        switch index {
        case 0:
            return "Did your declared stop actually print on the broker — or are you bailing before that price?"
        case 1:
            return "Did your written invalidation truly fire — not just noise, discomfort, or a scare candle?"
        case 2:
            return "Are you exiting because price hit your declared target — or are you peeling early without the plan finishing?"
        default:
            return ""
        }
    }

    private func answerButton(title: String, value: Bool, index: Int) -> some View {
        Button {
            gateState = BarExitGateReducer.reduce(
                state: gateState,
                action: .answerQuestion(index: index, value: value),
            )
        } label: {
            Text(title)
                .font(.system(size: 13, weight: .semibold, design: .rounded))
                .foregroundColor(BarDS.Fill.accentInk)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 10)
                .background(BarDS.Accent.teal)
                .cornerRadius(10)
        }
        .buttonStyle(.plain)
    }

    private var scareCandleBlock: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("Scare candle check")
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .foregroundColor(BarDS.Accent.amber.opacity(0.95))

            Text(
                "You said the plan is still intact, but one candle may be driving this exit. "
                    + "TradeAutopsy data shows exits on single-candle fear often miss the target the plan was built for. "
                    + "Breathe, zoom out one timeframe, and only exit if your written invalidation truly fired.",
            )
            .font(.system(size: 12, weight: .medium, design: .rounded))
            .foregroundColor(.white.opacity(0.82))
            .fixedSize(horizontal: false, vertical: true)

            Button {
                gateState = BarExitGateReducer.reduce(state: gateState, action: .acknowledgeScareMirror)
            } label: {
                Text("I've read this — continue")
                    .font(.system(size: 13, weight: .bold, design: .rounded))
                    .foregroundColor(BarDS.Fill.accentInk)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, 12)
                    .background(BarDS.Accent.amber)
                    .cornerRadius(10)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Acknowledge scare candle mirror")

            Button("Not now") { onCancel() }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.65))
        }
    }

    private var cleanExitBlock: some View {
        VStack(alignment: .leading, spacing: 12) {
            Text("All checks answered yes — your rule set supports ending this trade deliberately.")
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.82))
                .fixedSize(horizontal: false, vertical: true)

            primaryContinue

            Button("Not now") { onCancel() }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.65))
        }
    }

    private var coolOffBlock: some View {
        let remaining = coolOffRemaining
        let chip = coolOffChip
        return VStack(alignment: .leading, spacing: 12) {
            Text("Pause — this exit doesn't match a clean rule hit.")
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .foregroundColor(Color.orange.opacity(0.9))

            Text("Pick what best describes the urge (optional). A short cool-off runs before broker exit tools unlock.")
                .font(.system(size: 11, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.62))
                .fixedSize(horizontal: false, vertical: true)

            LazyVGrid(columns: [GridItem(.adaptive(minimum: 120), spacing: 8)], spacing: 8) {
                ForEach(verdictChips, id: \.self) { c in
                    verdictChipButton(c, selected: chip == c)
                }
            }

            Text(remaining > 0 ? "Cool-off: \(remaining)s" : "Cool-off complete")
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundColor(remaining > 0 ? Color.white.opacity(0.55) : BarDS.Accent.teal.opacity(0.9))

            Button {
                if remaining == 0 { onConfirmExit() }
            } label: {
                Text("Continue to exit tools →")
            }
            .buttonStyle(.plain)
            .font(.system(size: 13, weight: .bold, design: .rounded))
            .foregroundColor(remaining == 0 ? BarDS.Fill.accentInk : Color.white.opacity(0.35))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 12)
            .background(remaining == 0 ? BarDS.Accent.teal : Color.white.opacity(0.12))
            .cornerRadius(10)
            .disabled(remaining > 0)
            .accessibilityLabel("Continue to exit tools")

            Button("Not now") { onCancel() }
                .buttonStyle(.plain)
                .font(.system(size: 12, weight: .medium, design: .rounded))
                .foregroundColor(.white.opacity(0.65))
        }
    }

    private var coolOffRemaining: Int {
        if case let .coolOffVerdict(c, _) = gateState.flow { return c }
        return 0
    }

    private var coolOffChip: String? {
        if case let .coolOffVerdict(_, chip) = gateState.flow { return chip }
        return nil
    }

    private func verdictChipButton(_ title: String, selected: Bool) -> some View {
        Button {
            gateState = BarExitGateReducer.reduce(state: gateState, action: .selectVerdictChip(title))
        } label: {
            Text(title)
                .font(.system(size: 10, weight: .medium, design: .rounded))
                .foregroundColor(selected ? BarDS.Accent.teal : Color.white.opacity(0.75))
                .padding(.horizontal, 10)
                .padding(.vertical, 6)
                .background(
                    RoundedRectangle(cornerRadius: 6)
                        .fill(selected ? BarDS.Accent.teal.opacity(0.14) : Color.white.opacity(0.06)),
                )
        }
        .buttonStyle(.plain)
    }

    private var primaryContinue: some View {
        Button {
            onConfirmExit()
        } label: {
            Text("Continue to exit tools →")
                .font(.system(size: 13, weight: .bold, design: .rounded))
                .foregroundColor(BarDS.Fill.accentInk)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
                .background(BarDS.Accent.teal)
                .cornerRadius(10)
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Continue to exit tools")
    }
}
