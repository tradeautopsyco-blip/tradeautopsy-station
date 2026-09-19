import CoreGraphics
import Foundation

/// Cash-style and USDT-premium price strings for Session BUY/SL/TP drag commit.
enum SessionChartPriceFormat {
    static func string(from value: Double) -> String {
        let magnitude = abs(value)
        var places = 2
        if magnitude > 0, magnitude < 1 {
            places = min(8, max(2, 3 - Int(floor(log10(magnitude)))))
        }
        var text = String(format: "%.\(places)f", value)
        if magnitude > 0, magnitude < 1 {
            while places > 2, text.hasSuffix("0") {
                text.removeLast()
                places -= 1
            }
        }
        if magnitude > 0, Double(text) == 0 {
            text = String(format: "%.2e", value)
        }
        return text
    }
}

enum SessionChartLine: Equatable {
    case sl
    case tp
    case entry
}

/// Prototype `hitLine` order: SL, then TP, then entry. Slop is 8px.
struct SessionChartHit: Equatable {
    var entryY: CGFloat
    var slY: CGFloat
    var tpY: CGFloat
    var slop: CGFloat

    func line(atY y: CGFloat) -> SessionChartLine? {
        let pairs: [(SessionChartLine, CGFloat)] = [(.sl, slY), (.tp, tpY), (.entry, entryY)]
        for (id, py) in pairs {
            if abs(y - py) < slop { return id }
        }
        return nil
    }
}

/// Price plot: y=0 is maxPrice, y=height is minPrice.
struct SessionChartScale: Equatable {
    var minPrice: Double
    var maxPrice: Double
    var height: CGFloat

    func price(atY y: CGFloat) -> Double {
        guard height > 0, maxPrice > minPrice else { return minPrice }
        let t = Double(min(max(y / height, 0), 1))
        return maxPrice - t * (maxPrice - minPrice)
    }

    func y(forPrice price: Double) -> CGFloat {
        guard height > 0, maxPrice > minPrice else { return 0 }
        let t = (price - minPrice) / (maxPrice - minPrice)
        return height - CGFloat(t) * height
    }

    /// Include candles, plan prices, bound last, and the drag ghost so BUY at last
    /// sits on the last candle instead of the volume gutter.
    static func fitted(prices: [Double], height: CGFloat) -> SessionChartScale {
        let lo = prices.min() ?? 0
        let hi = prices.max() ?? 1
        let minY = lo
        var maxY = hi
        if maxY <= minY { maxY = minY + 1 }
        let pad = (maxY - minY) * 0.12
        return SessionChartScale(minPrice: minY - pad, maxPrice: maxY + pad, height: max(1, height))
    }
}

/// Plot chrome: right axis band, SL/TP extent from the right, last-tag reserve.
enum SessionChartOverlay {
    static let axisWidth: CGFloat = 40
    /// SL/TP start this fraction across the plot (30% from the right).
    static let dashedStartFraction: CGFloat = 0.7
    static let lastTagReserve: CGFloat = 12
    static let ladderCount = 5

    static func plotWidth(total: CGFloat) -> CGFloat {
        max(1, total - axisWidth)
    }

    /// Last line Y only when quote last is bound. Nil last → no line.
    static func lastLineY(last: Double?, scale: SessionChartScale) -> CGFloat? {
        guard let last else { return nil }
        let y = scale.y(forPrice: last)
        guard y >= 0, y <= scale.height else { return nil }
        return y
    }

    static func ladderPrices(minPrice: Double, maxPrice: Double, count: Int = ladderCount) -> [Double] {
        guard count >= 2, maxPrice > minPrice else { return [] }
        return (0..<count).map { i in
            minPrice + (maxPrice - minPrice) * Double(i) / Double(count - 1)
        }
    }

    /// Drop ladder ticks whose Y collides with the reserved last tag.
    static func visibleTicks(
        prices: [Double],
        lastY: CGFloat?,
        scale: SessionChartScale,
        reserve: CGFloat = lastTagReserve
    ) -> [Double] {
        guard let lastY else { return prices }
        return prices.filter { abs(scale.y(forPrice: $0) - lastY) >= reserve }
    }

    static func dashedLineStartX(plotWidth: CGFloat) -> CGFloat {
        plotWidth * dashedStartFraction
    }
}

/// Snap a dragged plan price to the candle under the pointer.
enum SessionChartMagnet {
    static let slop: CGFloat = 6

    static func candleIndex(x: CGFloat, plotWidth: CGFloat, count: Int) -> Int? {
        guard count > 0, plotWidth > 0, x >= 0, x < plotWidth else { return nil }
        let slot = plotWidth / CGFloat(count)
        let i = Int(floor(x / slot))
        return (0..<count).contains(i) ? i : nil
    }

    /// Nearest of open/high/low/close within `slop` pixels; otherwise the free price.
    static func snap(
        price: Double,
        open: Double,
        high: Double,
        low: Double,
        close: Double,
        scale: SessionChartScale,
        slop: CGFloat = slop
    ) -> Double {
        let y = scale.y(forPrice: price)
        var bestPrice = price
        var bestDist = slop
        for candidate in [open, high, low, close] {
            let dy = abs(scale.y(forPrice: candidate) - y)
            if dy < bestDist {
                bestDist = dy
                bestPrice = candidate
            }
        }
        return bestPrice
    }
}

/// Time left in the last kline. Missing interval → no countdown.
enum SessionChartBarClose {
    static func intervalMs(_ raw: String?) -> Int64? {
        let s = raw?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        guard !s.isEmpty else { return nil }
        let lower = s.lowercased()
        if lower.hasSuffix("min") {
            let n = Int64(lower.dropLast(3).trimmingCharacters(in: .whitespaces)) ?? 0
            return n > 0 ? n * 60_000 : nil
        }
        if lower.count >= 2, let n = Int64(lower.dropLast()), n > 0 {
            switch lower.last {
            case "s": return n * 1_000
            case "m": return n * 60_000
            case "h": return n * 3_600_000
            case "d": return n * 86_400_000
            case "w": return n * 7 * 86_400_000
            default: break
            }
        }
        return nil
    }

    static func remainingMs(openTimeMs: Int64, interval: String?, nowMs: Int64) -> Int64? {
        guard let ms = intervalMs(interval) else { return nil }
        return (openTimeMs + ms) - nowMs
    }

    static func remainingLabel(openTimeMs: Int64, interval: String?, nowMs: Int64) -> String? {
        guard let left = remainingMs(openTimeMs: openTimeMs, interval: interval, nowMs: nowMs) else {
            return nil
        }
        let clamped = max(0, left)
        let totalSec = clamped / 1000
        let m = totalSec / 60
        let s = totalSec % 60
        if m > 0 {
            return String(format: "%d:%02d", m, s)
        }
        return "\(s)s"
    }
}

/// Hovered-bar readout. Live last stays on the glance tile.
enum SessionChartLegend {
    struct Model: Equatable {
        var headline: String
        var ohlcv: String?
        var ohlcvUp: Bool
        var change: String?
        var changeUp: Bool
    }

    static func compactVolume(_ volume: Double) -> String {
        if volume >= 1_000_000 {
            return String(format: "%.2fM", volume / 1_000_000)
        }
        if volume >= 1_000 {
            return String(format: "%.2fK", volume / 1_000)
        }
        if volume == floor(volume) {
            return String(format: "%.0f", volume)
        }
        return String(format: "%.2f", volume)
    }

    static func change(close: Double, prevClose: Double?) -> (text: String, up: Bool)? {
        guard let prevClose, prevClose != 0 else { return nil }
        let delta = close - prevClose
        let pct = (delta / prevClose) * 100
        let sign = delta >= 0 ? "+" : "-"
        let mag = SessionChartPriceFormat.string(from: abs(delta))
        return ("\(sign)\(mag) (\(sign)\(String(format: "%.2f", abs(pct)))%)", delta >= 0)
    }

    static func model(
        symbol: String,
        interval: String?,
        open: Double,
        high: Double,
        low: Double,
        close: Double,
        volume: Double,
        prevClose: Double?
    ) -> Model {
        let intervalBit = {
            let t = interval?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
            return t.isEmpty ? "" : " · \(t)"
        }()
        let headline = "\(symbol.isEmpty ? "—" : symbol)\(intervalBit)"
        let ohlc =
            "O \(SessionChartPriceFormat.string(from: open)) "
            + "H \(SessionChartPriceFormat.string(from: high)) "
            + "L \(SessionChartPriceFormat.string(from: low)) "
            + "C \(SessionChartPriceFormat.string(from: close))"
        let volBit = volume > 0 ? " V \(compactVolume(volume))" : ""
        let moved = change(close: close, prevClose: prevClose)
        return Model(
            headline: headline,
            ohlcv: ohlc + volBit,
            ohlcvUp: close >= open,
            change: moved?.text,
            changeUp: moved?.up ?? true
        )
    }
}

/// Quote last is bound when the envelope lit a number. Unavailable family stays dark.
enum SessionChartQuoteLast {
    static func isBound(status: String) -> Bool {
        switch status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() {
        case "unavailable", "quotes_http", "session", "quotes_unusable", "":
            return false
        default:
            return true
        }
    }

    static func value(_ last: Double?, status: String) -> Double? {
        guard let last, last > 0, isBound(status: status) else { return nil }
        return last
    }
}
