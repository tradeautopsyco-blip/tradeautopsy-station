import SwiftUI

/// Mosaic row math: honor the cockpit viewport so Ticket is not clipped off-screen.
enum BarCockpitMosaicMetrics {
    static func fittedSize(
        proposed: CGSize,
        rows: Int,
        spacing: CGFloat,
        minRowHeight: CGFloat
    ) -> CGSize {
        let rows = max(1, rows)
        if proposed.height > 0, proposed.height.isFinite {
            return CGSize(width: max(0, proposed.width), height: proposed.height)
        }
        let height = CGFloat(rows) * minRowHeight + CGFloat(max(0, rows - 1)) * spacing
        return CGSize(width: max(0, proposed.width), height: max(height, minRowHeight))
    }

    static func rowHeight(in height: CGFloat, rows: Int, spacing: CGFloat) -> CGFloat {
        let rows = max(1, rows)
        guard height > 0, height.isFinite else { return 1 }
        let inner = height - CGFloat(max(0, rows - 1)) * spacing
        return max(1, inner / CGFloat(rows))
    }
}

/// Prototype mosaic: 4 columns, tiles placed at `{x,y,w,h}` (zero-based).
struct BarCockpitMosaicPlacement: Equatable {
    var x: Int
    var y: Int
    var w: Int
    var h: Int

    var isCompact: Bool { w == 1 || h == 1 }
}

struct BarCockpitMosaicGrid: Layout {
    var columns: Int = 4
    var placements: [BarCockpitMosaicPlacement]
    var spacing: CGFloat = 8
    var minRowHeight: CGFloat = 96

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache _: inout ()) -> CGSize {
        let rows = max(1, placements.map { $0.y + $0.h }.max() ?? 1)
        return BarCockpitMosaicMetrics.fittedSize(
            proposed: CGSize(width: proposal.width ?? 0, height: proposal.height ?? -1),
            rows: rows,
            spacing: spacing,
            minRowHeight: minRowHeight
        )
    }

    func placeSubviews(
        in bounds: CGRect,
        proposal: ProposedViewSize,
        subviews: Subviews,
        cache _: inout ()
    ) {
        let rows = max(1, placements.map { $0.y + $0.h }.max() ?? 1)
        let colW = max(
            0,
            (bounds.width - CGFloat(max(0, columns - 1)) * spacing) / CGFloat(columns)
        )
        let rowH = BarCockpitMosaicMetrics.rowHeight(
            in: bounds.height,
            rows: rows,
            spacing: spacing
        )
        for (index, subview) in subviews.enumerated() {
            guard index < placements.count else { break }
            let p = placements[index]
            let x = bounds.minX + CGFloat(p.x) * (colW + spacing)
            let y = bounds.minY + CGFloat(p.y) * (rowH + spacing)
            let w = CGFloat(p.w) * colW + CGFloat(max(0, p.w - 1)) * spacing
            let h = CGFloat(p.h) * rowH + CGFloat(max(0, p.h - 1)) * spacing
            subview.place(
                at: CGPoint(x: x, y: y),
                anchor: .topLeading,
                proposal: ProposedViewSize(width: w, height: h)
            )
        }
    }
}
