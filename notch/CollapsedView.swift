import SwiftUI

/// Collapsed notch — hover expands. Non–physical-notch Macs get a glass pill (see `hasPhysicalNotch`).
struct CollapsedNotchView: View {
    @ObservedObject var viewModel: NotchViewModel

    private var coreStrip: some View {
        HStack(spacing: 10) {
            scoreIndicator

            HStack(spacing: 6) {
                Text(String(format: "%.2f", viewModel.compositeScore))
                    .font(.system(size: 13, weight: .semibold, design: .monospaced))
                    .foregroundColor(.white)

                Text(viewModel.behavioralState)
                    .font(.system(size: 10, weight: .medium, design: .rounded))
                    .foregroundColor(Color.white.opacity(0.45))
                    .tracking(0.5)
            }

            if viewModel.sessionPnL != 0 {
                Rectangle()
                    .fill(Color.white.opacity(0.1))
                    .frame(width: 0.5, height: 12)

                Text(viewModel.formattedSessionPnL)
                    .font(.system(size: 11, weight: .medium, design: .monospaced))
                    .foregroundColor(
                        viewModel.sessionPnL > 0
                            ? Color(hex: "#00E5C0")
                            : Color(hex: "#FF3B30")
                    )
            }

            HStack(spacing: 3) {
                ForEach(0 ..< 3, id: \.self) { _ in
                    Circle()
                        .fill(Color.white.opacity(0.2))
                        .frame(width: 3, height: 3)
                }
            }
            .padding(.leading, 2)
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 6)
        .frame(height: 32)
    }

    private var scoreIndicator: some View {
        Group {
            if viewModel.shouldPulse {
                TimelineView(.animation(minimumInterval: 1.0 / 30.0)) { timeline in
                    let t = timeline.date.timeIntervalSinceReferenceDate
                    let scale = 1.0 + 0.12 * sin(t * (2 * .pi / 1.2))
                    scoreIndicatorCore(scale: scale)
                }
            } else {
                scoreIndicatorCore(scale: 1.0)
                    .animation(.spring(response: 0.3), value: viewModel.compositeScore)
            }
        }
    }

    private func scoreIndicatorCore(scale: CGFloat) -> some View {
        ZStack {
            Circle()
                .fill(NotchTheme.scoreGlow(viewModel.compositeScore))
                .frame(width: 18, height: 18)
                .blur(radius: 4)
                .opacity(viewModel.compositeScore > 0.20 ? 1 : 0)

            Circle()
                .fill(NotchTheme.scoreColor(viewModel.compositeScore))
                .frame(width: 8, height: 8)
        }
        .scaleEffect(scale)
    }

    var body: some View {
        Group {
            if viewModel.hasPhysicalNotch {
                coreStrip
                    .background(
                        Capsule(style: .continuous)
                            .fill(Color.black.opacity(0.0))
                    )
            } else {
                coreStrip
                    .padding(.horizontal, 4)
                    .background(
                        Capsule(style: .continuous)
                            .fill(Color(hex: "#0A0A0A").opacity(0.95))
                            .overlay(
                                Capsule(style: .continuous)
                                    .stroke(Color.white.opacity(0.1), lineWidth: 0.5)
                            )
                            .shadow(
                                color: Color.black.opacity(0.6),
                                radius: 12, x: 0, y: 4
                            )
                    )
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
