import SwiftUI

/// HTML `ta-charts.js` platters — line = time series, bars = category compare.
/// Demo series match `NOTCH-ui.html` so the plots are visible even before live data.

struct BarTAPoint: Identifiable {
    var id: String { "\(x)-\(y)" }
    let x: String
    let y: Double
}

enum BarTAChartDemo {
    static let morningPnL: [BarTAPoint] = [
        .init(x: "09:15", y: 0),
        .init(x: "09:40", y: 420),
        .init(x: "10:05", y: 810),
        .init(x: "10:22", y: 180),
        .init(x: "10:31", y: -420),
        .init(x: "11:02", y: -900),
        .init(x: "11:18", y: -610),
        .init(x: "12:04", y: -1480),
        .init(x: "15:20", y: -2100),
    ]

    static let fidelitySessions: [BarTAPoint] = [
        .init(x: "30 Jul", y: 88),
        .init(x: "31 Jul", y: 90),
        .init(x: "1 Aug", y: 86),
        .init(x: "2 Aug", y: 91),
        .init(x: "4 Aug", y: 93),
        .init(x: "5 Aug", y: 89),
        .init(x: "6 Aug", y: 94),
        .init(x: "7 Aug", y: 92),
        .init(x: "8 Aug", y: 95),
        .init(x: "9 Aug", y: 90),
        .init(x: "11 Aug", y: 96),
        .init(x: "12 Aug", y: 90),
        .init(x: "13 Aug", y: 93),
        .init(x: "14 Aug", y: 94),
    ]

    static let setupMix: [BarTAPoint] = [
        .init(x: "Breakout", y: 42),
        .init(x: "Pullback", y: 28),
        .init(x: "Reversal", y: 14),
        .init(x: "Gap fill", y: 10),
        .init(x: "Mean reversion", y: 6),
    ]
}

enum BarTALineKind {
    case loss
    case fidelity
}

struct BarTALinePlot: View {
    let points: [BarTAPoint]
    var kind: BarTALineKind = .loss
    var height: CGFloat = 52
    var includeZero: Bool = true
    var yMinFixed: Double?
    var yMaxFixed: Double?
    var showDiamonds: Bool = false

    var body: some View {
        Canvas { context, size in
            guard points.count > 1 else { return }
            let ys = points.map(\.y)
            let loData = ys.min() ?? 0
            let hiData = ys.max() ?? 1
            var lo = yMinFixed ?? min(includeZero ? min(0, loData) : loData, loData)
            var hi = yMaxFixed ?? max(includeZero ? max(0, hiData) : hiData, hiData)
            if yMinFixed == nil, yMaxFixed == nil {
                let pad = (hi - lo) * 0.12
                lo -= pad
                hi += pad
            }
            if hi <= lo { hi = lo + 1 }

            let left: CGFloat = 2
            let right: CGFloat = 2
            let top: CGFloat = 4
            let bottom: CGFloat = 4
            func xAt(_ i: Int) -> CGFloat {
                left + CGFloat(i) / CGFloat(points.count - 1) * (size.width - left - right)
            }
            func yAt(_ v: Double) -> CGFloat {
                top + (1 - CGFloat((v - lo) / (hi - lo))) * (size.height - top - bottom)
            }

            if includeZero, lo < 0, hi > 0 {
                var zero = Path()
                let y0 = yAt(0)
                zero.move(to: CGPoint(x: left, y: y0))
                zero.addLine(to: CGPoint(x: size.width - right, y: y0))
                context.stroke(
                    zero,
                    with: .color(Color.white.opacity(0.35)),
                    style: StrokeStyle(lineWidth: 1, dash: [3, 3])
                )
            }

            var area = Path()
            let yZero = includeZero ? yAt(0) : yAt(lo)
            area.move(to: CGPoint(x: xAt(0), y: yZero))
            for (i, pt) in points.enumerated() {
                area.addLine(to: CGPoint(x: xAt(i), y: yAt(pt.y)))
            }
            area.addLine(to: CGPoint(x: xAt(points.count - 1), y: yZero))
            area.closeSubpath()

            let areaColor: Color = {
                switch kind {
                case .loss: return BarDS.Accent.red.opacity(0.16)
                case .fidelity: return Color.white.opacity(0.06)
                }
            }()
            context.fill(area, with: .color(areaColor))

            var line = Path()
            line.move(to: CGPoint(x: xAt(0), y: yAt(points[0].y)))
            for (i, pt) in points.enumerated() where i > 0 {
                line.addLine(to: CGPoint(x: xAt(i), y: yAt(pt.y)))
            }
            let stroke: Color = {
                switch kind {
                case .loss: return BarDS.Accent.red
                case .fidelity: return BarDS.Text.primary
                }
            }()
            context.stroke(
                line,
                with: .color(stroke),
                style: StrokeStyle(lineWidth: 1.6, lineCap: .round, lineJoin: .round)
            )

            if showDiamonds {
                for (i, pt) in points.enumerated() {
                    let c = CGPoint(x: xAt(i), y: yAt(pt.y))
                    var d = Path()
                    d.move(to: CGPoint(x: c.x, y: c.y - 3.2))
                    d.addLine(to: CGPoint(x: c.x + 3.2, y: c.y))
                    d.addLine(to: CGPoint(x: c.x, y: c.y + 3.2))
                    d.addLine(to: CGPoint(x: c.x - 3.2, y: c.y))
                    d.closeSubpath()
                    context.stroke(d, with: .color(BarDS.Text.primary), lineWidth: 1.4)
                }
            }
        }
        .frame(height: height)
        .frame(maxWidth: .infinity)
        .accessibilityHidden(true)
    }
}

struct BarTABarRow: View {
    let label: String
    let pct: Double
    let maxPct: Double
    var selected: Bool
    var action: () -> Void

    var body: some View {
        Button(action: action) {
            HStack(spacing: 8) {
                Text(label)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .lineLimit(1)
                    .frame(width: 92, alignment: .leading)
                GeometryReader { geo in
                    ZStack(alignment: .leading) {
                        Capsule()
                            .fill(Color.white.opacity(0.08))
                        Capsule()
                            .fill(selected ? Color.white : Color.white.opacity(0.55))
                            .frame(width: max(4, geo.size.width * CGFloat(pct / max(maxPct, 1))))
                    }
                }
                .frame(height: 8)
                Text("\(Int(pct))%")
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                    .frame(width: 40, alignment: .trailing)
            }
            .padding(.vertical, 6)
            .padding(.horizontal, 8)
            .background(
                RoundedRectangle(cornerRadius: 6, style: .continuous)
                    .fill(selected ? Color.white.opacity(0.06) : Color.clear)
            )
            .contentShape(Rectangle())
        }
        .buttonStyle(NotchPressButtonStyle(pressedScale: 0.98))
    }
}

struct BarTAChartFigure<Plot: View>: View {
    let kicker: String
    let takeaway: String
    var readout: String?
    @ViewBuilder var plot: () -> Plot

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(kicker.uppercased())
                .font(BarDS.bodyFont(11, weight: .medium))
                .foregroundColor(Color.white.opacity(0.5))
                .kerning(0.006 * 11)
                .padding(.bottom, 6)
            Text(takeaway)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundColor(BarDS.Text.primary)
                .lineSpacing(3)
                .fixedSize(horizontal: false, vertical: true)
                .padding(.bottom, 10)
            plot()
            if let readout {
                Text(readout)
                    .font(BarDS.monoFont(11, weight: .medium))
                    .foregroundColor(BarDS.Text.secondary)
                    .padding(.top, 6)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}
