import CoreGraphics
import SwiftUI

/// Read-only crosshair overlay for bound session charts (`founder-backlog.md` §4).
enum SessionChartCrosshair {
    static func draw(
        context: inout GraphicsContext,
        plotWidth: CGFloat,
        height: CGFloat,
        candleCenterX: CGFloat,
        barHigh: Double,
        barLow: Double,
        scale: SessionChartScale
    ) {
        var vPath = Path()
        vPath.move(to: CGPoint(x: candleCenterX, y: 0))
        vPath.addLine(to: CGPoint(x: candleCenterX, y: height))
        context.stroke(vPath, with: .color(Color.white.opacity(0.22)), lineWidth: 1)

        let mid = (barHigh + barLow) / 2
        let y = scale.y(forPrice: mid)
        var hPath = Path()
        hPath.move(to: CGPoint(x: 0, y: y))
        hPath.addLine(to: CGPoint(x: plotWidth, y: y))
        context.stroke(hPath, with: .color(Color.white.opacity(0.18)), style: StrokeStyle(lineWidth: 1, dash: [4, 3]))
    }
}
