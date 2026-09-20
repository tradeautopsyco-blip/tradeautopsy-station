import SwiftUI

struct PositionsLeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if viewModel.killSwitchActive {
                HStack(spacing: 8) {
                    Image(systemName: "lock.fill")
                        .font(.system(size: 10, weight: .bold))
                        .foregroundColor(Color(hex: "#FF3B30"))
                        .accessibilityHidden(true)
                    VStack(alignment: .leading, spacing: 1) {
                        Text("ENTRIES BLOCKED")
                            .font(.system(size: 9, weight: .bold, design: .monospaced))
                            .foregroundColor(Color(hex: "#FF3B30"))
                            .tracking(0.8)
                        Text("EXIT POSITIONS: ALLOWED")
                            .font(.system(size: 8, weight: .medium, design: .monospaced))
                            .foregroundColor(Color(hex: "#00E5C0"))
                            .tracking(0.5)
                        if let cs = viewModel.killSwitchCountdownSecs {
                            Text("Countdown: \(cs)s")
                                .font(.system(size: 8, weight: .medium, design: .monospaced))
                                .foregroundColor(Color.white.opacity(0.55))
                                .accessibilityHidden(true)
                        }
                    }
                    Spacer()
                }
                .accessibilityElement(children: .combine)
                .accessibilityLabel(
                    "Kill switch active. New entries blocked. Exits allowed."
                )
                .accessibilityAddTraits(.isStaticText)
                .onChange(of: viewModel.killSwitchActive) { _, active in
                    guard active else { return }
                    NotchVoiceOver.announce(
                        "Kill switch active. New entries blocked. Exits allowed.",
                        assertive: true
                    )
                }
                .padding(10)
                .background(Color(hex: "#FF3B30").opacity(0.08))
                .cornerRadius(10)
                .overlay(
                    RoundedRectangle(cornerRadius: 10)
                        .stroke(Color(hex: "#FF3B30").opacity(0.2), lineWidth: 0.5)
                )
            }

            sectionHeader("OPEN POSITIONS")

            if viewModel.positions.isEmpty {
                emptyState("No open positions")
            } else {
                VStack(spacing: 5) {
                    ForEach(viewModel.positions) { pos in
                        HStack(spacing: 8) {
                            VStack(alignment: .leading, spacing: 1) {
                                Text(pos.symbol)
                                    .font(.system(
                                        size: 11,
                                        weight: .semibold,
                                        design: .rounded
                                    ))
                                    .foregroundColor(.white)
                                Text("\(pos.qty) qty · \(pos.direction)")
                                    .font(.system(size: 9, weight: .regular))
                                    .foregroundColor(Color.white.opacity(0.35))
                            }
                            Spacer()
                            VStack(alignment: .trailing, spacing: 1) {
                                Text(formatPnL(pos.unrealizedPnL))
                                    .font(.system(
                                        size: 11,
                                        weight: .semibold,
                                        design: .monospaced
                                    ))
                                    .foregroundColor(
                                        pos.unrealizedPnL >= 0
                                            ? Color(hex: "#00E5C0")
                                            : Color(hex: "#FF3B30")
                                    )
                                if abs(pos.unrealizedPnL) > 100 {
                                    Image(systemName: "exclamationmark.triangle.fill")
                                        .font(.system(size: 8))
                                        .foregroundColor(Color(hex: "#FF9500"))
                                }
                            }
                        }
                        .padding(9)
                        .glassCard(radius: 8)
                        .accessibilityLabel(
                            "\(pos.symbol) \(pos.direction) \(pos.qty) quantity, unrealized \(formatPnL(pos.unrealizedPnL))"
                        )
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func formatPnL(_ v: Double) -> String {
        let raw = viewModel.formatINR(v)
        if v >= 0 { return "+\(raw)" }
        return raw
    }
}

struct PositionsRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            VStack(alignment: .leading, spacing: 2) {
                Text("UNREALIZED").microLabel()
                Text(formatPnL(viewModel.totalUnrealizedPnL))
                    .font(.system(size: 24, weight: .light, design: .monospaced))
                    .foregroundColor(
                        viewModel.totalUnrealizedPnL >= 0
                            ? Color(hex: "#00E5C0")
                            : Color(hex: "#FF3B30")
                    )
                    .glowEffect(
                        viewModel.totalUnrealizedPnL >= 0
                            ? Color(hex: "#00E5C0").opacity(0.15)
                            : Color(hex: "#FF3B30").opacity(0.15),
                        radius: 8
                    )
            }
            .padding(12)
            .glassCard(radius: 10)
            .frame(maxWidth: .infinity, alignment: .leading)

            VStack(spacing: 4) {
                dataRow("TOTAL EXPOSURE", viewModel.totalExposureText)
                dataRow("OPEN ORDERS", "\(viewModel.openOrders)")
            }
            .padding(10)
            .glassCard(radius: 10)

            Spacer()

            Text("No flatten from this list. Kill is separate. Stop is broker sync pause.")
                .font(.system(size: 10, weight: .medium))
                .foregroundColor(Color.white.opacity(0.45))
                .fixedSize(horizontal: false, vertical: true)

            Spacer()

            if !viewModel.isAuthenticated {
                Text("Sign in from Station to view live positions.")
                    .font(.system(size: 10, weight: .semibold))
                    .foregroundColor(Color(hex: "#00E5C0"))
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.top, 4)
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func dataRow(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label).microLabel()
            Spacer()
            Text(value)
                .font(.system(size: 11, weight: .medium, design: .monospaced))
                .foregroundColor(.white)
        }
    }

    private func formatPnL(_ v: Double) -> String {
        let raw = viewModel.formatINR(v)
        if v >= 0 { return "+\(raw)" }
        return raw
    }
}
