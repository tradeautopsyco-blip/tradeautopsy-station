import SwiftUI

// MARK: - Zone switcher

struct LeftZoneView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        Group {
            switch viewModel.activeTab {
            case .pulse: PulseLeftView(viewModel: viewModel)
            case .brief: BriefLeftView(viewModel: viewModel)
            case .workflows: WorkflowLeftView(viewModel: viewModel)
            case .tai: TAILeftView(viewModel: viewModel)
            case .positions: PositionsLeftView(viewModel: viewModel)
            case .capture:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .transition(.opacity.combined(with: .move(edge: .leading)))
        .animation(.easeInOut(duration: 0.2), value: viewModel.activeTab)
    }
}

struct RightZoneView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        Group {
            switch viewModel.activeTab {
            case .pulse: PulseRightView(viewModel: viewModel)
            case .brief: BriefRightView(viewModel: viewModel)
            case .workflows: WorkflowRightView(viewModel: viewModel)
            case .tai: TAIRightView(viewModel: viewModel)
            case .positions: PositionsRightView(viewModel: viewModel)
            case .capture:
                Color.clear.frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .transition(.opacity.combined(with: .move(edge: .trailing)))
        .animation(.easeInOut(duration: 0.2), value: viewModel.activeTab)
    }
}

// MARK: - Expanded shell

struct ExpandedNotchView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Environment(\.accessibilityReduceTransparency) private var reduceTransparency

    var body: some View {
        VStack(spacing: 0) {
            if viewModel.notchTopInset > 0 {
                Color.clear
                    .frame(height: viewModel.notchTopInset)
            }

            ZStack {
                // Phase 10 - Reduce Transparency
                if reduceTransparency {
                    Color(hex: "#050505")
                } else {
                    Color(hex: "#050505")

                    RadialGradient(
                        colors: [
                            NotchTheme.scoreGlow(viewModel.compositeScore),
                            Color.clear,
                        ],
                        center: .top,
                        startRadius: 0,
                        endRadius: 200
                    )
                    .opacity(viewModel.activeTab == .capture ? 0.35 : 0.6)
                    .allowsHitTesting(false)
                }

                if viewModel.activeTab == .capture {
                    JournalCapturePanelView(viewModel: viewModel)
                } else {
                    HStack(spacing: 0) {
                        LeftZoneView(viewModel: viewModel)
                            .frame(width: 300)
                            .padding(.leading, 20)

                        CenterZoneView(viewModel: viewModel)
                            .frame(width: 300)

                        RightZoneView(viewModel: viewModel)
                            .frame(width: 300)
                            .padding(.trailing, 20)
                    }
                    .frame(height: 200)
                }
            }
            .overlay(
                Rectangle()
                    .fill(Color.white.opacity(0.04))
                    .frame(height: 0.5),
                alignment: .bottom
            )

            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
}
