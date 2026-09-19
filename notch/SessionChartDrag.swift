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
}
