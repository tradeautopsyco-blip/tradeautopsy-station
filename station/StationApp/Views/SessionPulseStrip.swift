import SwiftUI

public struct SessionPulseStrip: View {
    @ObservedObject private var viewModel: SessionModel
    @ObservedObject private var coordinator: StationAppCoordinator

    public init(viewModel: SessionModel, coordinator: StationAppCoordinator) {
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

        let configuredSlugs = coordinator.brokersViewModel.configuredBrokerSlugs
        let honesty = DeskHonesty.resolve(activeSlugs: configuredSlugs)
        let dualChips: [SessionPulseStripPresentation.DualDeskChip]?
        let showActiveMoney: Bool
        let quote: String

        switch honesty {
        case let .dualNoBlend(profiles):
            dualChips = SessionPulseStripPresentation.dualChips(
                profiles: profiles,
                activeSlug: viewModel.activeBrokerSlug
            )
            if let hero = DeskHonesty.heroQuoteCurrency(
                activeSlugs: viewModel.activeBrokerSlug.map { [$0] } ?? []
            ) {
                quote = hero
                showActiveMoney = true
            } else if let mapped = DeskMoneyFormatting.quoteCurrency(
                forBrokerSlug: viewModel.activeBrokerSlug
            ) {
                quote = mapped
                showActiveMoney = true
            } else {
                quote = "USD"
                showActiveMoney = false
            }
        case let .single(quoteCurrency, _, _):
            dualChips = nil
            quote = quoteCurrency
            showActiveMoney = true
        case .none:
            dualChips = nil
            quote = viewModel.deskQuoteCurrency
                ?? coordinator.todayViewModel.lastDeskQuoteCurrency
                ?? "USD"
            showActiveMoney = true
        }

        return SessionPulseStripPresentation.build(
            sessionPnLUsd: coordinator.todayViewModel.sessionPnLUsd,
            unrealizedTotal: viewModel.totalUnrealizedPnL,
            positions: viewModel.positions,
            isDegraded: isDegraded,
            formatUSD: { DeskMoneyFormatting.formatSigned($0, quoteCurrency: quote) },
            dualChips: dualChips,
            showActiveMoney: showActiveMoney
        )
    }

    public var body: some View {
        Button(action: coordinator.openLiveTradeFromPulseStrip) {
            HStack(spacing: 16) {
                if presentation.showsDegradedIndicator {
                    Circle()
                        .fill(StationDS.Accent.amber)
                        .frame(width: 6, height: 6)
                        .accessibilityLabel("Data unavailable")
                }

                if let chips = presentation.dualChips, !chips.isEmpty {
                    HStack(spacing: 8) {
                        ForEach(chips) { chip in
                            Text(chip.label)
                                .font(StationDS.monoFont(StationDS.FontSize.chip, weight: .medium))
                                .foregroundStyle(
                                    chip.isActive ? StationDS.Text.primary : StationDS.Text.muted
                                )
                                .padding(.horizontal, 8)
                                .padding(.vertical, 3)
                                .background(StationDS.Fill.input)
                                .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                                .accessibilityLabel(
                                    chip.isActive
                                        ? "Active desk \(chip.label)"
                                        : "Desk \(chip.label)"
                                )
                        }
                    }
                    .accessibilityIdentifier("sessionPulseDualChips")
                }

                metric(label: "Session P&L", value: presentation.sessionPnLText, style: presentation.sessionPnLStyle)
                metric(label: "Unrealized", value: presentation.unrealizedPnLText, style: presentation.unrealizedPnLStyle)

                if let count = presentation.positionCountText {
                    metric(label: "Positions", value: count, style: .neutral)
                }

                if let symbol = presentation.singlePositionSymbol {
                    Text(symbol)
                        .font(StationDS.monoFont(StationDS.FontSize.chip, weight: .medium))
                        .foregroundStyle(StationDS.Text.primary)
                        .padding(.horizontal, 8)
                        .padding(.vertical, 3)
                        .background(StationDS.Fill.input)
                        .clipShape(RoundedRectangle(cornerRadius: StationDS.Radius.small))
                        .accessibilityLabel("Open position symbol \(symbol)")
                }

                Spacer(minLength: 0)
            }
            .padding(.horizontal, 16)
            .frame(maxWidth: .infinity)
            .frame(height: 38)
            .background(StationDS.Fill.glass)
            .overlay(alignment: .bottom) {
                Rectangle()
                    .fill(StationDS.Border.divider)
                    .frame(height: StationDS.borderThin)
            }
        }
        .buttonStyle(.plain)
        .accessibilityIdentifier("sessionPulseStrip")
    }

    @ViewBuilder
    private func metric(label: String, value: String, style: SessionPulseStripPresentation.PnLStyle) -> some View {
        HStack(spacing: 6) {
            Text(label)
                .font(StationDS.bodyFont(StationDS.FontSize.chrome))
                .foregroundStyle(StationDS.Text.secondary)
                .kerning(0.012 * StationDS.FontSize.chrome)
            Text(value)
                .font(StationDS.monoFont(StationDS.FontSize.bodySmall, weight: .medium))
                .foregroundStyle(color(for: style))
        }
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(label) \(value)")
    }

    private func color(for style: SessionPulseStripPresentation.PnLStyle) -> Color {
        switch style {
        case .positive:
            return StationDS.Accent.green
        case .negative:
            return StationDS.Accent.red
        case .neutral:
            return StationDS.Text.primary
        }
    }
}
