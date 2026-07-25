import Notch
import SwiftUI

public struct SessionPulseStrip: View {
    @ObservedObject private var viewModel: NotchViewModel
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(viewModel: NotchViewModel, coordinator: StationAppCoordinator) {
        self.viewModel = viewModel
        self.coordinator = coordinator
    }

    private var presentation: SessionPulseStripPresentation {
        let todayDegraded = coordinator.todayViewModel.presentation.state == .agentDown
            || coordinator.todayViewModel.presentation.state == .syncUnavailable
        let isDegraded = SessionPulseStripPresentation.isDegraded(
            agentHealthWarning: coordinator.agentHealthWarning,
            brokerSessionActive: viewModel.brokerSessionActive,
            todayDegraded: todayDegraded
        )
        let quote = viewModel.deskQuoteCurrency
            ?? coordinator.todayViewModel.lastDeskQuoteCurrency
            ?? "USD"
        return SessionPulseStripPresentation.build(
            sessionPnLUsd: coordinator.todayViewModel.sessionPnLUsd,
            unrealizedTotal: viewModel.totalUnrealizedPnL,
            positions: viewModel.positions,
            isDegraded: isDegraded,
            formatUSD: { DeskMoneyFormatting.formatSigned($0, quoteCurrency: quote) }
        )
    }

    public var body: some View {
        Button(action: coordinator.openLiveTradeFromPulseStrip) {
            HStack(spacing: 16) {
                if presentation.showsDegradedIndicator {
                    Circle()
                        .fill(BarDS.Accent.amber)
                        .frame(width: 6, height: 6)
                        .accessibilityLabel("Data unavailable")
                }

                metric(label: "Session P&L", value: presentation.sessionPnLText, style: presentation.sessionPnLStyle)
                metric(label: "Unrealized", value: presentation.unrealizedPnLText, style: presentation.unrealizedPnLStyle)

                if let count = presentation.positionCountText {
                    metric(label: "Positions", value: count, style: .neutral)
                }

                if let symbol = presentation.singlePositionSymbol {
                    Text(symbol)
                        .font(BarDS.monoFont(BarDS.FontSize.chip, weight: .medium))
                        .foregroundStyle(BarDS.Text.primary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(BarDS.Fill.input)
                        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small))
                        .accessibilityLabel("Open position symbol \(symbol)")
                }

                Spacer(minLength: 0)
            }
            .padding(.horizontal, 16)
            .frame(maxWidth: .infinity)
            .frame(height: 38)
            .background(BarDS.Fill.appPanel)
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(height: BarDS.borderThin)
            }
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("sessionPulseStrip")
    }

    @ViewBuilder
    private func metric(label: String, value: String, style: SessionPulseStripPresentation.PnLStyle) -> some View {
        HStack(spacing: 6) {
            Text(label)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS))
                .foregroundStyle(BarDS.Text.secondary)
            Text(value)
                .font(BarDS.monoFont(BarDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(color(for: style))
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label) \(value)")
    }

    private func color(for style: SessionPulseStripPresentation.PnLStyle) -> Color {
        switch style {
        case .positive:
            return BarDS.Accent.green
        case .negative:
            return BarDS.Accent.red
        case .neutral:
            return BarDS.Text.primary
        }
    }
}
