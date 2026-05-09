import SwiftUI

struct CenterZoneView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(spacing: 14) {
            Spacer()

            ZStack {
                Circle()
                    .fill(NotchTheme.scoreGlow(viewModel.compositeScore))
                    .frame(width: 100, height: 100)
                    .blur(radius: 16)

                Circle()
                    .stroke(Color.white.opacity(0.06), lineWidth: 2.5)
                    .frame(width: 76, height: 76)

                Circle()
                    .trim(from: 0, to: CGFloat(viewModel.compositeScore))
                    .stroke(
                        AngularGradient(
                            colors: [
                                NotchTheme.scoreColor(viewModel.compositeScore).opacity(0.6),
                                NotchTheme.scoreColor(viewModel.compositeScore),
                            ],
                            center: .center,
                            startAngle: .degrees(-90),
                            endAngle: .degrees(270)
                        ),
                        style: StrokeStyle(
                            lineWidth: 2.5,
                            lineCap: .round
                        )
                    )
                    .frame(width: 76, height: 76)
                    .rotationEffect(.degrees(-90))
                    .animation(
                        .spring(response: 0.7, dampingFraction: 0.8),
                        value: viewModel.compositeScore
                    )

                VStack(spacing: 1) {
                    Text(String(format: "%.2f", viewModel.compositeScore))
                        .font(.system(
                            size: 18,
                            weight: .semibold,
                            design: .monospaced
                        ))
                        .foregroundColor(.white)
                        .glowEffect(
                            NotchTheme.scoreColor(viewModel.compositeScore),
                            radius: 6
                        )

                    Text(viewModel.behavioralState)
                        .font(.system(
                            size: 8,
                            weight: .semibold,
                            design: .rounded
                        ))
                        .foregroundColor(Color.white.opacity(0.4))
                        .tracking(1.2)
                        .textCase(.uppercase)
                }
            }
            .scaleEffect(viewModel.scoreRingPulseScale)
            .animation(.spring(response: 0.25, dampingFraction: 0.65), value: viewModel.scoreRingPulseScale)

            HStack(spacing: 4) {
                ForEach(NotchTab.allCases) { tab in
                    tabPill(tab)
                }
            }

            Spacer()
        }
    }

    @ViewBuilder
    func tabPill(_ tab: NotchTab) -> some View {
        let isSelected = viewModel.activeTab == tab

        Button {
            viewModel.selectTab(tab)
        } label: {
            HStack(spacing: isSelected ? 5 : 0) {
                Image(systemName: tab.icon)
                    .font(.system(size: 10, weight: .medium))
                    .foregroundColor(
                        isSelected
                            ? Color(hex: "#00E5C0")
                            : Color.white.opacity(0.35)
                    )

                if isSelected {
                    Text(tab.label)
                        .font(.system(
                            size: 8,
                            weight: .semibold,
                            design: .rounded
                        ))
                        .foregroundColor(Color(hex: "#00E5C0"))
                        .tracking(0.8)
                        .textCase(.uppercase)
                        .transition(.opacity.combined(with: .scale))
                }
            }
            .padding(.horizontal, isSelected ? 8 : 6)
            .padding(.vertical, 5)
            .background(
                ZStack {
                    if isSelected {
                        Capsule(style: .continuous)
                            .fill(Color(hex: "#00E5C0").opacity(0.1))
                        Capsule(style: .continuous)
                            .stroke(Color(hex: "#00E5C0").opacity(0.25), lineWidth: 0.5)
                    } else {
                        Capsule(style: .continuous)
                            .fill(Color.white.opacity(0.04))
                        Capsule(style: .continuous)
                            .stroke(Color.white.opacity(0.06), lineWidth: 0.5)
                    }
                }
            )
        }
        .buttonStyle(.plain)
        .accessibilityLabel(tab.accessibilityPillLabel)
        .animation(.spring(response: 0.25), value: isSelected)
    }
}
