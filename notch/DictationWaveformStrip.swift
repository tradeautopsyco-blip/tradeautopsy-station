import SwiftUI

/// Nine-dot RMS strip — §6.3 / §6.7 Reduce Motion trims animation.
struct DictationWaveformStrip: View {
    let levels: [Float]
    let isActive: Bool
    var reduceMotion: Bool

    var body: some View {
        HStack(spacing: 4) {
            ForEach(Array(levels.enumerated()), id: \.offset) { _, level in
                let h = 10 + CGFloat(level) * 12
                Capsule(style: .continuous)
                    .fill(
                        isActive
                            ? BarDS.Accent.teal.opacity(0.25 + Double(level) * 0.75)
                            : Color.white.opacity(0.08 + Double(level) * 0.35)
                    )
                    .frame(width: 3, height: h)
                    .modifier(ConditionalAnimation(reduceMotion: reduceMotion, value: level))
            }
        }
        .accessibilityLabel(isActive ? "Recording waveform, active" : "Waveform inactive")
        .accessibilityHidden(!isActive)
    }
}

private struct ConditionalAnimation: ViewModifier {
    let reduceMotion: Bool
    let value: Float

    func body(content: Content) -> some View {
        if reduceMotion {
            content
        } else {
            content.animation(.spring(response: 0.16, dampingFraction: 0.85), value: value)
        }
    }
}

/// Mic latch + waveform; use under TAI **or** the journal capture panel (`dictationUsesCaptureDraft`).
struct DictationMicAndWaveform: View {
    @ObservedObject var viewModel: NotchViewModel
    var reduceMotion: Bool

    var body: some View {
        HStack(spacing: 10) {
            DictationWaveformStrip(
                levels: viewModel.dictationWaveform,
                isActive: viewModel.isDictating,
                reduceMotion: reduceMotion
            )

            Button {
                viewModel.toggleDictationLatch(reduceMotion: reduceMotion)
            } label: {
                Image(systemName: viewModel.dictationPermissionDenied ? "mic.slash" : "mic.fill")
                    .font(.system(size: 16, weight: .semibold))
                    .foregroundColor(
                        viewModel.isDictating
                            ? BarDS.Accent.teal
                            : Color.white.opacity(0.45)
                    )
                    .frame(width: 32, height: 32)
                    .background(
                        Circle()
                            .fill(
                                viewModel.isDictating
                                    ? BarDS.Accent.teal.opacity(0.15)
                                    : Color.white.opacity(0.06)
                            )
                    )
            }
            .buttonStyle(.plain)
            .accessibilityLabel(viewModel.isDictating ? "Stop dictation" : "Start dictation")
            .accessibilityHint("Uses on-device speech recognition, no data leaves this Mac")
            .accessibilityAddTraits(.isButton)
        }
        .onChange(of: viewModel.isDictating) { _, on in
            NotchVoiceOver.announce(
                on ? "Dictation recording" : "Dictation ready",
                assertive: false
            )
        }
    }
}
