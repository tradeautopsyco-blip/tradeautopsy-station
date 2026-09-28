import SwiftUI

/// Last-N density for the Session tile. Gapless logical index — not wall-clock.
enum SessionChartLayout {
    static let minSlot: CGFloat = 3
    static let floorCount = 12
    static let capCount = 80
    static let volumeFraction: CGFloat = 0.2

    static func visibleCount(width: CGFloat, total: Int) -> Int {
        guard total > 0 else { return 0 }
        let fromWidth = width > 0 ? Int((width / minSlot).rounded(.down)) : floorCount
        let clamped = min(capCount, max(floorCount, fromWidth))
        return min(total, max(clamped, 1))
    }

    static func lastN(_ candles: [DeskSessionCandle], width: CGFloat) -> [DeskSessionCandle] {
        let n = visibleCount(width: width, total: candles.count)
        guard n > 0 else { return [] }
        if candles.count <= n { return candles }
        return Array(candles.suffix(n))
    }

    static func istLabel(_ openTimeMs: Int64) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(openTimeMs) / 1000)
        return istTime.string(from: date)
    }

    private static let istTime: DateFormatter = {
        let f = DateFormatter()
        f.locale = Locale(identifier: "en_IN")
        f.timeZone = TimeZone(identifier: "Asia/Kolkata") ?? TimeZone(secondsFromGMT: 5 * 3600 + 30 * 60)
        f.dateFormat = "HH:mm"
        return f
    }()
}

/// Optional drag onto Session BUY/SL/TP. Commit writes typed Risk fields on release.
struct SessionChartDragBindings {
    var sideBuy: Bool
    var entryText: Binding<String>
    var stopText: Binding<String>
    var targetText: Binding<String>
}

/// Read-only plan levels for Working (no drag). Same line semantics as declare chart.
struct SessionChartPlanLevels: Equatable {
    var sideBuy: Bool
    var entry: Double?
    var stop: Double?
    var target: Double?
}

private struct SessionChartBar {
    let openTimeMs: Int64
    let open: Double
    let high: Double
    let low: Double
    let close: Double
    let volume: Double
}

/// Venue OHLC bars for Session. Last-N from tile width, volume overlay, IST labels.
/// Drag BUY/SL/TP commits on release. No pan, zoom, interval picker, or place chrome.
struct BarOptionsSessionChart: View {
    let candles: [DeskSessionCandle]
    var drag: SessionChartDragBindings? = nil
    /// Working desk — declared entry/stop/target without editable drag.
    var fixedPlanLevels: SessionChartPlanLevels? = nil
    /// Bound quote last only. Nil → no last line, even if history close exists.
    var last: Double? = nil
    var symbol: String = ""
    var interval: String? = nil

    @State private var dragging: SessionChartLine?
    @State private var ghost: Double?
    @State private var hoverIndex: Int?

    var body: some View {
        GeometryReader { geo in
            let plotW = SessionChartOverlay.plotWidth(total: geo.size.width)
            let visible = SessionChartLayout.lastN(candles, width: plotW)
            let parsed = Self.parse(visible)
            let scale = self.scale(parsed: parsed, height: geo.size.height)
            ZStack(alignment: .topLeading) {
                Canvas { context, size in
                    guard !parsed.isEmpty, size.width > 0, size.height > 0 else { return }
                    let plotWidth = SessionChartOverlay.plotWidth(total: size.width)
                    let volumeH = size.height * SessionChartLayout.volumeFraction
                    let maxVol = parsed.map(\.volume).max() ?? 0
                    let n = parsed.count
                    let slot = plotWidth / CGFloat(max(n, 1))
                    let bodyW = max(2, slot * 0.45)
                    if let drag {
                        drawRRBoxes(
                            context: context,
                            plotWidth: plotWidth,
                            scale: scale,
                            drag: drag
                        )
                    } else if let fixedPlanLevels {
                        drawRRBoxes(
                            context: context,
                            plotWidth: plotWidth,
                            scale: scale,
                            levels: fixedPlanLevels
                        )
                    }
                    for (i, bar) in parsed.enumerated() {
                        let x = slot * (CGFloat(i) + 0.5)
                        let up = bar.close >= bar.open
                        let color = up ? BarDS.Accent.teal : BarDS.Accent.red
                        if maxVol > 0, volumeH > 0 {
                            let vh = CGFloat(bar.volume / maxVol) * volumeH
                            let rect = CGRect(
                                x: x - bodyW / 2,
                                y: size.height - vh,
                                width: bodyW,
                                height: max(1, vh)
                            )
                            context.fill(Path(rect), with: .color(BarDS.Text.muted.opacity(0.35)))
                        }
                        let yHigh = scale.y(forPrice: bar.high)
                        let yLow = scale.y(forPrice: bar.low)
                        let yOpen = scale.y(forPrice: bar.open)
                        let yClose = scale.y(forPrice: bar.close)
                        var wick = Path()
                        wick.move(to: CGPoint(x: x, y: yHigh))
                        wick.addLine(to: CGPoint(x: x, y: yLow))
                        context.stroke(wick, with: .color(color), lineWidth: 1)
                        let top = min(yOpen, yClose)
                        let bodyH = max(1, abs(yClose - yOpen))
                        let rect = CGRect(x: x - bodyW / 2, y: top, width: bodyW, height: bodyH)
                        context.fill(Path(rect), with: .color(color.opacity(up ? 0.85 : 1)))
                    }
                    let formingUp = parsed.last.map { $0.close >= $0.open } ?? true
                    drawLastLine(
                        context: context,
                        plotWidth: plotWidth,
                        scale: scale,
                        formingUp: formingUp
                    )
                    if let drag {
                        drawPlanLines(
                            context: context,
                            plotWidth: plotWidth,
                            scale: scale,
                            drag: drag
                        )
                    } else if let fixedPlanLevels {
                        drawPlanLines(
                            context: context,
                            plotWidth: plotWidth,
                            scale: scale,
                            levels: fixedPlanLevels
                        )
                    }
                    drawAxis(
                        context: context,
                        plotWidth: plotWidth,
                        scale: scale,
                        formingUp: formingUp
                    )
                }
                legendOverlay(parsed: parsed)
                if visible.count >= 2 {
                    HStack {
                        ForEach(labelIndices(count: visible.count), id: \.self) { idx in
                            if idx > 0 { Spacer(minLength: 0) }
                            Text(SessionChartLayout.istLabel(visible[idx].openTimeMs))
                                .font(BarDS.monoFont(8, weight: .regular))
                                .foregroundColor(BarDS.Text.muted)
                            if idx < visible.count - 1 { Spacer(minLength: 0) }
                        }
                    }
                    .padding(.leading, 8)
                    .padding(.trailing, SessionChartOverlay.axisWidth)
                    .frame(maxHeight: .infinity, alignment: .bottom)
                    .padding(.bottom, 2)
                }
                if let lastBar = parsed.last {
                    TimelineView(.periodic(from: .now, by: 1)) { timeline in
                        let nowMs = Int64(timeline.date.timeIntervalSince1970 * 1000)
                        if let label = SessionChartBarClose.remainingLabel(
                            openTimeMs: lastBar.openTimeMs,
                            interval: interval,
                            nowMs: nowMs
                        ) {
                            Text(label)
                                .font(BarDS.monoFont(8, weight: .medium))
                                .foregroundColor(BarDS.Text.secondary)
                                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
                                .padding(.trailing, 2)
                                .padding(.top, 2)
                        }
                    }
                }
            }
            .contentShape(Rectangle())
            .gesture(dragGesture(scale: scale, parsed: parsed, plotWidth: plotW))
            .onContinuousHover { phase in
                switch phase {
                case .active(let loc):
                    if dragging == nil {
                        hoverIndex = SessionChartMagnet.candleIndex(
                            x: loc.x,
                            plotWidth: plotW,
                            count: parsed.count
                        )
                    }
                case .ended:
                    hoverIndex = nil
                }
            }
        }
        .padding(.horizontal, 8)
        .padding(.vertical, 6)
        .accessibilityLabel("Session klines")
    }

    @ViewBuilder
    private func legendOverlay(parsed: [SessionChartBar]) -> some View {
        let idx = hoverIndex.flatMap { parsed.indices.contains($0) ? $0 : nil }
            ?? parsed.indices.last
        if let idx {
            let bar = parsed[idx]
            let prev = idx > 0 ? parsed[idx - 1].close : nil
            let model = SessionChartLegend.model(
                symbol: symbol,
                interval: interval,
                open: bar.open,
                high: bar.high,
                low: bar.low,
                close: bar.close,
                volume: bar.volume,
                prevClose: prev
            )
            VStack(alignment: .leading, spacing: 1) {
                Text(model.headline)
                    .foregroundColor(BarDS.Text.secondary)
                if let ohlcv = model.ohlcv {
                    Text(ohlcv)
                        .foregroundColor(model.ohlcvUp ? BarDS.Accent.teal : BarDS.Accent.red)
                }
                if let change = model.change {
                    Text(change)
                        .foregroundColor(model.changeUp ? BarDS.Accent.teal : BarDS.Accent.red)
                }
            }
            .font(BarDS.monoFont(9, weight: .regular))
            .padding(.leading, 6)
            .padding(.top, 4)
            .allowsHitTesting(false)
        }
    }

    private func scale(parsed: [SessionChartBar], height: CGFloat) -> SessionChartScale {
        let volumeH = height * SessionChartLayout.volumeFraction
        let priceH = max(1, height - volumeH)
        var prices: [Double] = []
        for bar in parsed {
            prices.append(contentsOf: [bar.high, bar.low, bar.open, bar.close])
        }
        if let last {
            prices.append(last)
        }
        if let drag {
            for text in [drag.entryText.wrappedValue, drag.stopText.wrappedValue, drag.targetText.wrappedValue] {
                if let p = Double(text.trimmingCharacters(in: .whitespacesAndNewlines)) {
                    prices.append(p)
                }
            }
        } else if let fixedPlanLevels {
            for p in [fixedPlanLevels.entry, fixedPlanLevels.stop, fixedPlanLevels.target].compactMap({ $0 }) {
                prices.append(p)
            }
        }
        if let ghost {
            prices.append(ghost)
        }
        return SessionChartScale.fitted(prices: prices, height: priceH)
    }

    private func drawRRBoxes(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        drag: SessionChartDragBindings
    ) {
        let entry = Double(drag.entryText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        let sl = Double(drag.stopText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        let tp = Double(drag.targetText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        drawRRBoxes(
            context: context,
            plotWidth: plotWidth,
            scale: scale,
            entry: entry,
            sl: sl,
            tp: tp,
            sideBuy: drag.sideBuy
        )
    }

    private func drawRRBoxes(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        levels: SessionChartPlanLevels
    ) {
        drawRRBoxes(
            context: context,
            plotWidth: plotWidth,
            scale: scale,
            entry: levels.entry,
            sl: levels.stop,
            tp: levels.target,
            sideBuy: levels.sideBuy
        )
    }

    private func drawRRBoxes(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        entry: Double?,
        sl: Double?,
        tp: Double?,
        sideBuy: Bool
    ) {
        guard let entry, let sl, let tp,
              let ratio = BarIntradayDeclareValidator.riskRewardRatio(
                  entry: entry,
                  stop: sl,
                  target: tp,
                  sideBuy: sideBuy
              )
        else { return }
        let yEntry = scale.y(forPrice: entry)
        let ySL = scale.y(forPrice: sl)
        let yTP = scale.y(forPrice: tp)
        let riskRect = CGRect(
            x: 0,
            y: min(yEntry, ySL),
            width: plotWidth,
            height: max(1, abs(yEntry - ySL))
        )
        let rewardRect = CGRect(
            x: 0,
            y: min(yEntry, yTP),
            width: plotWidth,
            height: max(1, abs(yEntry - yTP))
        )
        context.fill(Path(riskRect), with: .color(BarDS.Accent.red.opacity(0.12)))
        context.fill(Path(rewardRect), with: .color(BarDS.Accent.teal.opacity(0.12)))
        let label = String(format: "R:R %.2f", ratio)
        context.draw(
            Text(label)
                .font(BarDS.monoFont(8, weight: .medium))
                .foregroundColor(BarDS.Accent.teal),
            at: CGPoint(x: plotWidth * 0.5, y: min(yEntry, yTP) + 10)
        )
    }

    private func drawLastLine(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        formingUp: Bool
    ) {
        guard let y = SessionChartOverlay.lastLineY(last: last, scale: scale) else { return }
        let color = formingUp ? BarDS.Accent.teal : BarDS.Accent.red
        var path = Path()
        path.move(to: CGPoint(x: 0, y: y))
        path.addLine(to: CGPoint(x: plotWidth, y: y))
        context.stroke(
            path,
            with: .color(color),
            style: StrokeStyle(lineWidth: 1, dash: [4, 4])
        )
    }

    private func drawPlanLines(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        drag: SessionChartDragBindings
    ) {
        let entry = Double(drag.entryText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        let sl = Double(drag.stopText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        let tp = Double(drag.targetText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
        drawPlanLines(
            context: context,
            plotWidth: plotWidth,
            scale: scale,
            entry: entry,
            sl: sl,
            tp: tp,
            sideBuy: drag.sideBuy
        )
        if let ghost, dragging != nil {
            strokePlanLine(
                context: context,
                plotWidth: plotWidth,
                y: scale.y(forPrice: ghost),
                color: BarDS.Text.primary.opacity(0.28),
                badge: "",
                dashed: true,
                axisTag: nil
            )
        }
    }

    private func drawPlanLines(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        levels: SessionChartPlanLevels
    ) {
        drawPlanLines(
            context: context,
            plotWidth: plotWidth,
            scale: scale,
            entry: levels.entry,
            sl: levels.stop,
            tp: levels.target,
            sideBuy: levels.sideBuy
        )
    }

    private func drawPlanLines(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        entry: Double?,
        sl: Double?,
        tp: Double?,
        sideBuy: Bool
    ) {
        if let entry {
            strokePlanLine(
                context: context,
                plotWidth: plotWidth,
                y: scale.y(forPrice: entry),
                color: BarDS.Accent.blue,
                badge: sideBuy ? "BUY" : "SELL",
                dashed: false,
                axisTag: SessionChartPriceFormat.string(from: entry)
            )
        }
        if let sl {
            strokePlanLine(
                context: context,
                plotWidth: plotWidth,
                y: scale.y(forPrice: sl),
                color: BarDS.Accent.red,
                badge: "SL",
                dashed: true,
                axisTag: SessionChartPriceFormat.string(from: sl)
            )
        }
        if let tp {
            strokePlanLine(
                context: context,
                plotWidth: plotWidth,
                y: scale.y(forPrice: tp),
                color: BarDS.Accent.teal,
                badge: "TP",
                dashed: true,
                axisTag: SessionChartPriceFormat.string(from: tp)
            )
        }
    }

    private func strokePlanLine(
        context: GraphicsContext,
        plotWidth: CGFloat,
        y: CGFloat,
        color: Color,
        badge: String,
        dashed: Bool,
        axisTag: String?
    ) {
        let startX = dashed ? SessionChartOverlay.dashedLineStartX(plotWidth: plotWidth) : 0
        var path = Path()
        path.move(to: CGPoint(x: startX, y: y))
        path.addLine(to: CGPoint(x: plotWidth, y: y))
        context.stroke(
            path,
            with: .color(color),
            style: StrokeStyle(lineWidth: 1, dash: dashed ? [4, 4] : [])
        )
        if !badge.isEmpty {
            context.draw(
                Text(badge)
                    .font(BarDS.monoFont(8, weight: .medium))
                    .foregroundColor(color),
                at: CGPoint(x: dashed ? startX + 14 : 18, y: y - 8)
            )
        }
        if let axisTag {
            context.draw(
                Text(axisTag)
                    .font(BarDS.monoFont(8, weight: .medium))
                    .foregroundColor(color),
                at: CGPoint(x: plotWidth + SessionChartOverlay.axisWidth * 0.5, y: y)
            )
        }
    }

    private func drawAxis(
        context: GraphicsContext,
        plotWidth: CGFloat,
        scale: SessionChartScale,
        formingUp: Bool
    ) {
        let lastY = SessionChartOverlay.lastLineY(last: last, scale: scale)
        let ticks = SessionChartOverlay.visibleTicks(
            prices: SessionChartOverlay.ladderPrices(minPrice: scale.minPrice, maxPrice: scale.maxPrice),
            lastY: lastY,
            scale: scale
        )
        for price in ticks {
            let y = scale.y(forPrice: price)
            context.draw(
                Text(SessionChartPriceFormat.string(from: price))
                    .font(BarDS.monoFont(8, weight: .regular))
                    .foregroundColor(BarDS.Text.muted),
                at: CGPoint(x: plotWidth + SessionChartOverlay.axisWidth * 0.5, y: y)
            )
        }
        if let last, let lastY {
            let color = formingUp ? BarDS.Accent.teal : BarDS.Accent.red
            let tag = SessionChartPriceFormat.string(from: last)
            let tagSize = CGSize(width: SessionChartOverlay.axisWidth - 2, height: 12)
            let tagRect = CGRect(
                x: plotWidth + 1,
                y: lastY - tagSize.height / 2,
                width: tagSize.width,
                height: tagSize.height
            )
            context.fill(Path(roundedRect: tagRect, cornerRadius: 2), with: .color(color.opacity(0.9)))
            context.draw(
                Text(tag)
                    .font(BarDS.monoFont(8, weight: .medium))
                    .foregroundColor(BarDS.Fill.accentInk),
                at: CGPoint(x: plotWidth + SessionChartOverlay.axisWidth * 0.5, y: lastY)
            )
        }
    }

    private func dragGesture(
        scale: SessionChartScale,
        parsed: [SessionChartBar],
        plotWidth: CGFloat
    ) -> some Gesture {
        DragGesture(minimumDistance: 0)
            .onChanged { value in
                guard let drag else { return }
                let y = value.location.y
                if dragging == nil {
                    let entry = Double(drag.entryText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
                    let sl = Double(drag.stopText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
                    let tp = Double(drag.targetText.wrappedValue.trimmingCharacters(in: .whitespacesAndNewlines))
                    let hit = SessionChartHit(
                        entryY: entry.map { scale.y(forPrice: $0) } ?? -999,
                        slY: sl.map { scale.y(forPrice: $0) } ?? -999,
                        tpY: tp.map { scale.y(forPrice: $0) } ?? -999,
                        slop: 8
                    )
                    dragging = hit.line(atY: y)
                }
                guard dragging != nil else {
                    hoverIndex = SessionChartMagnet.candleIndex(
                        x: value.location.x,
                        plotWidth: plotWidth,
                        count: parsed.count
                    )
                    return
                }
                var price = scale.price(atY: y)
                if let idx = SessionChartMagnet.candleIndex(
                    x: value.location.x,
                    plotWidth: plotWidth,
                    count: parsed.count
                ) {
                    let bar = parsed[idx]
                    price = SessionChartMagnet.snap(
                        price: price,
                        open: bar.open,
                        high: bar.high,
                        low: bar.low,
                        close: bar.close,
                        scale: scale
                    )
                }
                ghost = price
                commit(price, on: dragging, drag: drag)
            }
            .onEnded { _ in
                dragging = nil
                ghost = nil
            }
    }

    private func commit(_ price: Double, on line: SessionChartLine?, drag: SessionChartDragBindings) {
        guard let line else { return }
        let text = SessionChartPriceFormat.string(from: price)
        switch line {
        case .entry: drag.entryText.wrappedValue = text
        case .sl: drag.stopText.wrappedValue = text
        case .tp: drag.targetText.wrappedValue = text
        }
    }

    private func labelIndices(count: Int) -> [Int] {
        if count <= 1 { return [0] }
        if count == 2 { return [0, 1] }
        return [0, count / 2, count - 1]
    }

    private static func parse(_ candles: [DeskSessionCandle]) -> [SessionChartBar] {
        candles.compactMap { c in
            guard let h = Double(c.high), let l = Double(c.low),
                  let o = Double(c.open), let cl = Double(c.close)
            else { return nil }
            let vol = Double(c.volume) ?? 0
            return SessionChartBar(
                openTimeMs: c.openTimeMs,
                open: o,
                high: h,
                low: l,
                close: cl,
                volume: vol
            )
        }
    }
}
