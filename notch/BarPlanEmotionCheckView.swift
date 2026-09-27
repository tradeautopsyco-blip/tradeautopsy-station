import SwiftUI

/// Immediate tap feedback for 1–5 emotion picks (no sluggish default button animation).
private struct BarEmotionPickButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? 0.94 : 1)
            .opacity(configuration.isPressed ? 0.88 : 1)
            .animation(.linear(duration: 0.07), value: configuration.isPressed)
    }
}

/// Four 1–5 sliders + planned/reactive. Shared across cash / NFO / options Plan rails.
struct BarPlanEmotionCheckView: View {
    @ObservedObject var viewModel: NotchViewModel

    private var allFilled: Bool {
        BarPlanGateStrip.emotionFilled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            frustration: viewModel.declEmotionalFrustration,
            excitement: viewModel.declEmotionalExcitement
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if allFilled {
                compactSummary
            } else {
                Text("Four readings. Answer honestly — it flags the ticket, it does not change size.")
                    .font(BarDS.bodyFont(12.5, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                    .fixedSize(horizontal: false, vertical: true)
                scaleRow(
                    title: "Calm — 1 is best",
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
                scaleRow(
                    title: "Frustration — 1 is none",
                    labels: ["None", "Mild", "Tight", "Hot", "Revenge"],
                    value: Binding(
                        get: { viewModel.declEmotionalFrustration },
                        set: { viewModel.declEmotionalFrustration = $0 },
                    ),
                )
                scaleRow(
                    title: "Excitement — 1 is flat",
                    labels: ["Flat", "Curious", "Eager", "Amped", "Chase"],
                    value: Binding(
                        get: { viewModel.declEmotionalExcitement },
                        set: { viewModel.declEmotionalExcitement = $0 },
                    ),
                )
            }

            HStack(spacing: 7) {
                BarChip(
                    label: BarPlanStance.planned.chipTitle,
                    selected: viewModel.declStance == BarPlanStance.planned.rawValue,
                ) {
                    viewModel.declStance = BarPlanStance.planned.rawValue
                }
                BarChip(
                    label: BarPlanStance.reactive.chipTitle,
                    selected: viewModel.declStance == BarPlanStance.reactive.rawValue,
                ) {
                    viewModel.declStance = BarPlanStance.reactive.rawValue
                }
            }
        }
    }

    private var compactSummary: some View {
        HStack(spacing: 8) {
            summaryChip("C", viewModel.declEmotionalCalm)
            summaryChip("K", viewModel.declEmotionalConfidence)
            summaryChip("F", viewModel.declEmotionalFrustration)
            summaryChip("X", viewModel.declEmotionalExcitement)
            Spacer(minLength: 8)
            Button("change") {
                viewModel.declEmotionalCalm = 0
                viewModel.declEmotionalConfidence = 0
                viewModel.declEmotionalFrustration = 0
                viewModel.declEmotionalExcitement = 0
            }
            .buttonStyle(.plain)
            .font(BarDS.bodyFont(11, weight: .regular))
            .foregroundColor(BarDS.Text.muted)
        }
        .padding(9)
        .background(BarDS.Fill.elevated)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
    }

    private func summaryChip(_ letter: String, _ value: Int) -> some View {
        Text("\(letter)\(value)")
            .font(BarDS.monoFont(12, weight: .medium))
            .foregroundColor(BarDS.Text.primary)
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
                    .buttonStyle(BarEmotionPickButtonStyle())
                    .contentShape(Rectangle())
                }
            }
        }
    }
}

struct BarPlanIntentField: View {
    @Binding var text: String

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("INTENT")
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            BarInputField(placeholder: "Why this, in one sentence.", text: $text, marginBottom: 0)
        }
    }
}

struct BarPlanGateStripView: View {
    @ObservedObject var viewModel: NotchViewModel
    let exitFilled: Bool

    private var emotionFilled: Bool {
        BarPlanGateStrip.emotionFilled(
            calm: viewModel.declEmotionalCalm,
            confidence: viewModel.declEmotionalConfidence,
            frustration: viewModel.declEmotionalFrustration,
            excitement: viewModel.declEmotionalExcitement
        )
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("GATE")
                .font(BarDS.monoFont(9.5, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
            gateRow("Capital — risk money, not rent", binding: $viewModel.declGateCapital)
            gateRow("Size is at the stop", binding: $viewModel.declGateOnePercent)
            gateRow("Still inside the day", binding: $viewModel.declGateMaxLoss)
            gateRow("Hedge decided / none", binding: $viewModel.declGateHedge)
            autoRow("Emotion", on: emotionFilled)
            autoRow("Exit written", on: exitFilled)
            if viewModel.declareAssetClass == .spot || viewModel.declareAssetClass == .equity {
                gateRow("Protective SL on fill", binding: $viewModel.declProtectiveSLConsent)
            }
            gateRow("I will debrief this", binding: $viewModel.declGateReview)
        }
    }

    private func gateRow(_ label: String, binding: Binding<Bool>) -> some View {
        Button {
            binding.wrappedValue.toggle()
        } label: {
            HStack(spacing: 8) {
                Image(systemName: binding.wrappedValue ? "checkmark.square.fill" : "square")
                    .foregroundColor(binding.wrappedValue ? BarDS.Accent.teal : BarDS.Text.muted)
                Text(label)
                    .font(BarDS.bodyFont(12, weight: .regular))
                    .foregroundColor(BarDS.Text.primary)
                Spacer(minLength: 0)
            }
        }
        .buttonStyle(.plain)
    }

    private func autoRow(_ label: String, on: Bool) -> some View {
        HStack(spacing: 8) {
            Image(systemName: on ? "checkmark.square.fill" : "square")
                .foregroundColor(on ? BarDS.Accent.teal : BarDS.Text.muted)
            Text(label)
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
            Spacer(minLength: 0)
        }
    }
}
