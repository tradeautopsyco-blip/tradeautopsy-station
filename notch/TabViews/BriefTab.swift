import SwiftUI

struct BriefLeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            sectionHeader("PRE-MARKET")

            HStack(spacing: 6) {
                marketIndex(
                    "NIFTY",
                    value: viewModel.niftyValue,
                    change: viewModel.niftyChange
                )
                marketIndex(
                    "BNF",
                    value: viewModel.bnfValue,
                    change: viewModel.bnfChange
                )
                marketIndex(
                    "VIX",
                    value: viewModel.vixValue,
                    change: nil,
                    accent: viewModel.vixValue < 15
                        ? Color(hex: "#00E5C0")
                        : viewModel.vixValue < 20
                            ? Color(hex: "#FF9500")
                            : Color(hex: "#FF3B30")
                )
            }

            divider()

            sectionHeader("RECOMMENDATION")
            Text(viewModel.morningBrief?.recommendation ?? "Fetching brief...")
                .font(.system(size: 10, weight: .regular, design: .rounded))
                .foregroundColor(Color.white.opacity(0.6))
                .lineLimit(3)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(12)
        .glassCard(radius: 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    private func marketIndex(
        _ name: String,
        value: Double,
        change: Double?,
        accent: Color = .white
    ) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(name).microLabel()
            Text(formatNumber(value))
                .font(.system(size: 12, weight: .semibold, design: .monospaced))
                .foregroundColor(accent)
            if let change {
                Text(formatChange(change))
                    .font(.system(size: 9, weight: .medium, design: .monospaced))
                    .foregroundColor(change >= 0
                        ? Color(hex: "#00E5C0")
                        : Color(hex: "#FF3B30"))
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func formatNumber(_ v: Double) -> String {
        String(format: "%.2f", v)
    }

    private func formatChange(_ c: Double) -> String {
        if abs(c) < 0.0001 { return "—" }
        return String(format: "%+.2f%%", c)
    }
}

struct BriefRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if !viewModel.edgeSymbols.isEmpty {
                sectionHeader("EDGE TODAY")
                VStack(spacing: 4) {
                    ForEach(viewModel.edgeSymbols, id: \.self) { sym in
                        HStack {
                            Text("▸")
                                .font(.system(size: 8, weight: .bold))
                                .foregroundColor(Color(hex: "#00E5C0"))
                            Text(sym)
                                .font(.system(size: 11, weight: .semibold, design: .rounded))
                                .foregroundColor(.white)
                            Spacer()
                        }
                    }
                }
                divider()
            }

            sectionHeader("WEDNESDAY ACTIVE")
            Text("1.35× score multiplier")
                .font(.system(size: 10, weight: .medium, design: .monospaced))
                .foregroundColor(Color(hex: "#FF9500"))

            Spacer()

            ghostButton("Open Full Brief") {
                viewModel.openDeepLink("tradeautopsy://dashboard")
            }
        }
        .padding(12)
        .glassCard(radius: 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
