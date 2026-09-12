import Foundation

/// Closed P&L floor-chart layout. Maps `TodayClosedChartPoint` onto unit space.
/// Last pill formats the last cumulative only — not a second PnL writer. Open MTM is not an input.
/// NSE session chrome: 09:15–15:30 IST from kotak-nse-bse-cash lock (fetch 2026-08-22). MIS 15:20 is not a tick.
public struct TodayClosedFloorChartLayout: Equatable, Sendable {
    public struct PlotPoint: Equatable, Sendable {
        public let x: Double
        public let y: Double
        public let cumulativeClosedPnL: Double
    }

    public let plotPoints: [PlotPoint]
    public let floorY: Double?
    public let tickLabels: [String]
    public let tickXs: [Double]
    public let lastPillText: String?
    public let fillIsLoss: Bool?
    public let showsNseSessionTicks: Bool
    /// Unit Y of PnL = 0. Area fill meets this line, not the chart bottom.
    public let zeroY: Double

    private static let nseLabels = ["09:15", "10:15", "11:15", "12:15", "13:15", "14:15", "15:30"]
    private static let sessionStartMinutes = 9 * 60 + 15
    private static let sessionSpanMinutes = 375.0

    public static func build(
        points: [TodayClosedChartPoint],
        floor: Double?,
        quoteCurrency: String?,
        brokerSlug: String?,
        now: Date,
        calendar: Calendar
    ) -> TodayClosedFloorChartLayout {
        _ = now
        _ = calendar
        let showsNse = Self.showsNseSessionTicks(
            quoteCurrency: quoteCurrency,
            brokerSlug: brokerSlug
        )
        // Settings floor is a positive loss budget. Closed-PnL axis draws it as −floor
        // (kotak-nse-bse-cash lock). A +12,000 floor must not squash the series to the bottom.
        let closedPnlFloor = floor.map { -$0 }
        let yScale = YScale(points: points, floor: closedPnlFloor)
        let plotPoints: [PlotPoint]
        if showsNse {
            plotPoints = points.compactMap { point in
                guard let date = parseClosedAt(point.closedAt) else { return nil }
                return PlotPoint(
                    x: nseSessionX(date),
                    y: yScale.y(point.cumulativeClosedPnL),
                    cumulativeClosedPnL: point.cumulativeClosedPnL
                )
            }
        } else {
            plotPoints = spanMappedPlotPoints(points, yScale: yScale)
        }
        let last = plotPoints.last?.cumulativeClosedPnL
        return TodayClosedFloorChartLayout(
            plotPoints: plotPoints,
            floorY: closedPnlFloor.map { yScale.y($0) },
            tickLabels: showsNse ? nseLabels : [],
            tickXs: showsNse ? nseTickXs() : [],
            lastPillText: last.map { compactWholePill($0, quoteCurrency: quoteCurrency) },
            fillIsLoss: last.map { $0 < 0 },
            showsNseSessionTicks: showsNse,
            zeroY: yScale.y(0)
        )
    }

    private static func showsNseSessionTicks(quoteCurrency: String?, brokerSlug: String?) -> Bool {
        brokerSlug == "kotak_neo" && quoteCurrency?.uppercased() == "INR"
    }

    private static func nseTickXs() -> [Double] {
        [0, 60, 120, 180, 240, 300, 375].map { Double($0) / sessionSpanMinutes }
    }

    private static func nseSessionX(_ date: Date) -> Double {
        var ist = Calendar(identifier: .gregorian)
        ist.timeZone = TimeZone(identifier: "Asia/Kolkata")!
        let comps = ist.dateComponents([.hour, .minute], from: date)
        let minutes = (comps.hour ?? 0) * 60 + (comps.minute ?? 0)
        let t = (Double(minutes - sessionStartMinutes)) / sessionSpanMinutes
        return min(1, max(0, t))
    }

    private static func spanMappedPlotPoints(
        _ points: [TodayClosedChartPoint],
        yScale: YScale
    ) -> [PlotPoint] {
        let dated = points.compactMap { point -> (Date, TodayClosedChartPoint)? in
            guard let date = parseClosedAt(point.closedAt) else { return nil }
            return (date, point)
        }
        guard !dated.isEmpty else { return [] }
        if dated.count == 1 {
            let point = dated[0].1
            return [
                PlotPoint(
                    x: 0.5,
                    y: yScale.y(point.cumulativeClosedPnL),
                    cumulativeClosedPnL: point.cumulativeClosedPnL
                ),
            ]
        }
        let times = dated.map(\.0)
        let tMin = times.min()!
        let tMax = times.max()!
        let span = tMax.timeIntervalSince(tMin)
        return dated.map { date, point in
            let x = span == 0 ? 0.5 : date.timeIntervalSince(tMin) / span
            return PlotPoint(
                x: x,
                y: yScale.y(point.cumulativeClosedPnL),
                cumulativeClosedPnL: point.cumulativeClosedPnL
            )
        }
    }

    /// Compact whole money of the last cumulative. Not a second PnL writer.
    private static func compactWholePill(_ value: Double, quoteCurrency: String?) -> String {
        let absValue = abs(value)
        let magnitude: String
        if absValue >= 1000 {
            let thousands = absValue / 1000
            let tenths = (thousands * 10).rounded() / 10
            if tenths == tenths.rounded() {
                magnitude = "\(Int(tenths.rounded()))k"
            } else {
                magnitude = String(format: "%.1fk", tenths)
            }
        } else {
            magnitude = "\(Int(absValue.rounded()))"
        }
        let symbol: String
        switch quoteCurrency?.uppercased() {
        case "INR":
            symbol = "₹"
        case "USD":
            symbol = "$"
        case .some(let code) where !code.isEmpty:
            symbol = code
        default:
            symbol = ""
        }
        if value < 0 {
            return "-\(symbol)\(magnitude)"
        }
        return "\(symbol)\(magnitude)"
    }

    private static func parseClosedAt(_ iso: String) -> Date? {
        let fractional = ISO8601DateFormatter()
        fractional.formatOptions = [.withInternetDateTime, .withFractionalSeconds]
        let plain = ISO8601DateFormatter()
        plain.formatOptions = [.withInternetDateTime]
        return fractional.date(from: iso) ?? plain.date(from: iso)
    }

    private struct YScale {
        let minY: Double
        let span: Double

        init(points: [TodayClosedChartPoint], floor: Double?) {
            var ys = points.map(\.cumulativeClosedPnL)
            if let floor {
                ys.append(floor)
            }
            ys.append(0)
            let minY = min(ys.min() ?? 0, 0)
            let maxY = max(ys.max() ?? 1, minY + 1)
            self.minY = minY
            self.span = maxY - minY
        }

        func y(_ value: Double) -> Double {
            (value - minY) / span
        }
    }
}
