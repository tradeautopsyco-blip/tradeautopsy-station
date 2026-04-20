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
                    VStack(alignment: .leading, spacing: 1) {
                        Text("ENTRIES BLOCKED")
                            .font(.system(size: 9, weight: .bold, design: .monospaced))
                            .foregroundColor(Color(hex: "#FF3B30"))
                            .tracking(0.8)
                        Text("EXIT POSITIONS: ALLOWED")
                            .font(.system(size: 8, weight: .medium, design: .monospaced))
                            .foregroundColor(Color(hex: "#00E5C0"))
                            .tracking(0.5)
                    }
                    Spacer()
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
                dataRow("TOTAL EXPOSURE", viewModel.formatINR(viewModel.totalExposure))
                dataRow("OPEN ORDERS", "\(viewModel.openOrders)")
            }
            .padding(10)
            .glassCard(radius: 10)

            Spacer()

            VStack(spacing: 6) {
                dangerButton("Exit All Positions") {
                    Task { await viewModel.exitAllPositions() }
                }
                ghostButton("Cancel All Orders") {
                    Task { await viewModel.cancelAllOrders() }
                }
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
