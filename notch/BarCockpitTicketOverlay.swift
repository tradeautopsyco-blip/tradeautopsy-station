import Foundation

/// Prototype `ticket=C` overlay: inject a 2×1 Ticket tile into the first mosaic hole.
/// Layout spec — not SwiftUI. COLS = 4 (dashboard mosaic).
enum BarCockpitTicketOverlay {
    static let columns = 4
    static let ticketW = 2
    static let ticketH = 1

    struct Occupancy: Equatable {
        var x: Int
        var y: Int
        var w: Int
        var h: Int
    }

    /// Prototype `firstHole`: scan y then x; first 2×1 that `canPlace`; else `(0, rowCount)`.
    static func firstHole(_ occupied: [Occupancy], w: Int = ticketW, h: Int = ticketH) -> Occupancy {
        let rowCount = occupied.map { $0.y + $0.h }.max() ?? 0
        let rows = rowCount + 4
        for y in 0 ..< rows {
            for x in 0 ... (columns - w) where x >= 0 {
                if canPlace(occupied, x: x, y: y, w: w, h: h) {
                    return Occupancy(x: x, y: y, w: w, h: h)
                }
            }
        }
        return Occupancy(x: 0, y: rowCount, w: w, h: h)
    }

    static func canPlace(_ occupied: [Occupancy], x: Int, y: Int, w: Int, h: Int) -> Bool {
        let candidate = Occupancy(x: x, y: y, w: w, h: h)
        return occupied.allSatisfy { !overlaps($0, candidate) }
    }

    static func overlaps(_ a: Occupancy, _ b: Occupancy) -> Bool {
        a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
    }

    static func occupancy(x: Int, y: Int, w: Int, h: Int) -> Occupancy {
        Occupancy(x: x, y: y, w: w, h: h)
    }
}

extension BarNfoCockpitSeed {
    static func occupancies(_ tiles: [Tile] = tiles) -> [BarCockpitTicketOverlay.Occupancy] {
        tiles.map { BarCockpitTicketOverlay.Occupancy(x: $0.x, y: $0.y, w: $0.w, h: $0.h) }
    }

    /// Crypto Options ticket-C overlay. NFO production seed stays `tiles` (no ticket).
    static func tilesWithTicketC() -> [Tile] {
        let hole = BarCockpitTicketOverlay.firstHole(occupancies(), w: 2, h: 1)
        return tiles + [Tile(kind: .ticket, x: hole.x, y: hole.y, w: hole.w, h: hole.h)]
    }
}

extension BarCashCockpitSeed {
    static func occupancies(for asset: BarDeclareAssetClass) -> [BarCockpitTicketOverlay.Occupancy] {
        tiles(for: asset).map {
            BarCockpitTicketOverlay.Occupancy(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
    }
}
