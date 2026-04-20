import SwiftUI

extension NotchTab: Identifiable {
    var id: String { rawValue }
}

extension NotchTab {
    var icon: String { sfSymbol }

    var label: String { pillLabel }

    var sfSymbol: String {
        switch self {
        case .pulse: return "waveform.path.ecg"
        case .brief: return "sun.horizon.fill"
        case .workflows: return "arrow.triangle.2.circlepath"
        case .tai: return "brain.head.profile"
        case .positions: return "chart.bar.fill"
        }
    }

    var pillLabel: String {
        switch self {
        case .pulse: return "PULSE"
        case .brief: return "BRIEF"
        case .workflows: return "FLOWS"
        case .tai: return "TAI"
        case .positions: return "POS"
        }
    }
}

struct NotchRootView: View {
    @ObservedObject var vm: NotchViewModel

    private var expansion: CGFloat {
        vm.isExpanded ? 1.0 : 0.0
    }

    var body: some View {
        ZStack {
            NotchTheme.bgApp

            vm.backgroundPulseColor
                .opacity(vm.backgroundPulseOpacity)
                .allowsHitTesting(false)

            if vm.killSwitchActive {
                Color.taDanger.opacity(0.12)
                    .allowsHitTesting(false)
            }

            Group {
                if vm.isExpanded {
                    ExpandedNotchView(viewModel: vm)
                        .transition(.opacity.combined(with: .move(edge: .top)))
                } else {
                    CollapsedNotchView(viewModel: vm)
                        .transition(.opacity)
                }
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .clipShape(NotchShape(expansionProgress: expansion))
        .overlay {
            if vm.isExpanded, !vm.hasPhysicalNotch {
                RoundedRectangle(cornerRadius: 20, style: .continuous)
                    .stroke(Color.white.opacity(0.12), lineWidth: 1)
            }
        }
        .shadow(
            color: vm.isExpanded && !vm.hasPhysicalNotch ? Color.black.opacity(0.6) : .clear,
            radius: 24,
            y: 8
        )
        .animation(NotchTheme.springExpand, value: vm.isExpanded)
        .animation(NotchTheme.springExpand, value: expansion)
        .background(
            MouseTracker { hovering in
                vm.handleHover(hovering)
            }
        )
    }
}
