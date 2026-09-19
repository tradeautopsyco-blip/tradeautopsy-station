import SwiftUI

/// Pre-trade cockpit for spot / equity / USDM / Coin-M (and leftover Options on `.standardForm`).
/// Glance strip + session/depth mosaic + Plan dock. Confirm is LiveBook — TRADE parked.
struct BarCashCockpitView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var quantityText: String
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    @ObservedObject private var boardStore = BarPretradeBoardStore.shared
    @State private var sessionTiles: [BarCashCockpitSeed.Tile]?
    @State private var sessionDock: BarCockpitDock?
    @State private var sessionId: String?

    private var family: BarPretradeBoardFamily {
        BarPretradeBoardFamily.family(for: viewModel.declareAssetClass)
    }

    private var liveId: String { boardStore.currentBoardId(for: family) }

    private var liveTiles: [BarCashCockpitSeed.Tile] {
        let base: [BarCashCockpitSeed.Tile]
        if sessionId == liveId, let sessionTiles {
            base = sessionTiles
        } else {
            base = resolvedTiles()
        }
        return withTicketOverlay(base)
    }

    private var liveDock: BarCockpitDock {
        if sessionId == liveId, let sessionDock { return sessionDock }
        return resolvedDock()
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarPretradeBoardChrome(
                title: liveName,
                blurb: liveBlurb,
                seedIds: BarCockpitBoardId.allCases,
                currentId: liveId,
                editing: boardStore.editing,
                canDelete: BarCockpitBoardId(rawValue: liveId) == nil,
                onSelect: selectBoard,
                onToggleEdit: { boardStore.editing.toggle() },
                onReset: resetBoard,
                onSaveAs: saveAsCustom,
                onDelete: deleteCustom,
            )
            BarCashGlanceStrip(viewModel: viewModel)
            if boardStore.editing {
                BarPretradeEditTray(
                    unusedTitles: unusedCatalog.map { ($0.rawValue, BarCashCockpitSeed.title(for: $0)) },
                    dock: liveDock,
                    onAdd: addKind,
                    onDock: { dock in
                        beginSession()
                        sessionDock = dock
                    },
                )
            }
            boardWork
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .onChange(of: liveId) { _, _ in
            sessionTiles = nil
            sessionDock = nil
            sessionId = nil
        }
        .onChange(of: viewModel.declareAssetClass) { _, _ in
            sessionTiles = nil
            sessionDock = nil
            sessionId = nil
        }
    }

    @ViewBuilder
    private var boardWork: some View {
        let mosaic = BarCashCockpitMosaic(
            viewModel: viewModel,
            sideBuy: $sideBuy,
            quantityText: $quantityText,
            stopLossText: $stopLossText,
            targetPriceText: $targetPriceText,
            tiles: liveTiles,
            editing: boardStore.editing,
            onRemove: removeKind,
        )
        let plan = BarCashCockpitPlanRail(
            viewModel: viewModel,
            sideBuy: $sideBuy,
            quantityText: $quantityText,
            stopLossText: $stopLossText,
            targetPriceText: $targetPriceText,
            submitReady: submitReady,
            submitHint: submitHint,
            onConfirm: onConfirm,
        )
        if liveDock == .floor {
            VStack(alignment: .leading, spacing: 8) {
                mosaic
                plan
                    .frame(maxWidth: .infinity, maxHeight: 280, alignment: .topLeading)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        } else {
            HStack(alignment: .top, spacing: 10) {
                mosaic
                plan
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
    }

    private var liveName: String {
        if let seed = BarCockpitBoardId(rawValue: liveId) {
            switch seed {
            case .cockpit: return BarCashCockpitSeed.showsDepth(for: viewModel.declareAssetClass) ? "Session + depth" : "Session + last"
            case .hero: return "Session owns"
            case .focus: return "Focus + floor"
            }
        }
        return boardStore.customBoard(id: liveId, family: family)?.name ?? "Board"
    }

    private var liveBlurb: String {
        "\(liveTiles.count) tiles · Plan \(liveDock == .floor ? "floor" : "rail")"
    }

    private var unusedCatalog: [BarCashCockpitSeed.Kind] {
        let present = Set(liveTiles.map(\.kind))
        return BarCashCockpitSeed.catalog(for: viewModel.declareAssetClass).filter { !present.contains($0) }
    }

    private func resolvedTiles() -> [BarCashCockpitSeed.Tile] {
        if let seed = BarCockpitBoardId(rawValue: liveId) {
            return BarCashCockpitSeed.tiles(for: viewModel.declareAssetClass, board: seed)
        }
        guard let custom = boardStore.customBoard(id: liveId, family: family) else {
            return BarCashCockpitSeed.tiles(for: viewModel.declareAssetClass)
        }
        return Self.cashTiles(from: custom, asset: viewModel.declareAssetClass)
    }

    private func resolvedDock() -> BarCockpitDock {
        if let seed = BarCockpitBoardId(rawValue: liveId) {
            return BarCashCockpitSeed.planDock(for: seed)
        }
        if let custom = boardStore.customBoard(id: liveId, family: family),
           let dock = BarCockpitDock(rawValue: custom.planDock)
        {
            return dock
        }
        return .rail
    }

    private var showsTicketC: Bool {
        BarDeskTicketSurface.usesVenueTicket(
            for: viewModel.declareAssetClass,
            slug: viewModel.resolvedDeskSlug,
            instrumentId: viewModel.deskSelectedInstrumentId,
        )
    }

    private func withTicketOverlay(_ tiles: [BarCashCockpitSeed.Tile]) -> [BarCashCockpitSeed.Tile] {
        guard showsTicketC, !tiles.contains(where: { $0.kind == .ticket }) else { return tiles }
        let occupied = tiles.map {
            BarCockpitTicketOverlay.Occupancy(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
        let size = BarCashCockpitSeed.ticketOverlaySize(for: viewModel.declareAssetClass)
        let hole = BarCockpitTicketOverlay.firstHole(occupied, w: size.w, h: size.h)
        return tiles + [BarCashCockpitSeed.Tile(kind: .ticket, x: hole.x, y: hole.y, w: hole.w, h: hole.h)]
    }

    private func selectBoard(_ id: String) {
        sessionTiles = nil
        sessionDock = nil
        sessionId = nil
        boardStore.select(id, family: family)
    }

    private func beginSession() {
        if sessionId != liveId {
            sessionTiles = resolvedTiles()
            sessionDock = resolvedDock()
            sessionId = liveId
        }
    }

    private func addKind(_ raw: String) {
        guard let kind = BarCashCockpitSeed.Kind(rawValue: raw) else { return }
        let allow = BarCashCockpitSeed.catalog(for: viewModel.declareAssetClass)
        guard allow.contains(kind) else { return }
        beginSession()
        var tiles = sessionTiles ?? resolvedTiles()
        guard !tiles.contains(where: { $0.kind == kind }) else { return }
        let size = BarCashCockpitSeed.addSize(for: kind, asset: viewModel.declareAssetClass)
        let occupied = tiles.map {
            BarCockpitTicketOverlay.Occupancy(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
        let hole = BarCockpitTicketOverlay.firstHole(occupied, w: size.w, h: size.h)
        tiles.append(BarCashCockpitSeed.Tile(kind: kind, x: hole.x, y: hole.y, w: hole.w, h: hole.h))
        sessionTiles = tiles
    }

    private func removeKind(_ kind: BarCashCockpitSeed.Kind) {
        guard kind != .ticket else { return }
        beginSession()
        sessionTiles = (sessionTiles ?? resolvedTiles()).filter { $0.kind != kind }
    }

    private func resetBoard() {
        sessionTiles = nil
        sessionDock = nil
        sessionId = nil
        boardStore.resetToCockpit(family: family)
        boardStore.editing = false
    }

    private func saveAsCustom() {
        beginSession()
        let tiles = (sessionTiles ?? resolvedTiles()).filter { $0.kind != .ticket }
        let dock = sessionDock ?? resolvedDock()
        let board = BarPretradeCustomBoard(
            id: "custom-\(UUID().uuidString.prefix(8))",
            name: "Custom",
            planDock: dock.rawValue,
            kinds: tiles.map(\.kind.rawValue),
            x: tiles.map(\.x),
            y: tiles.map(\.y),
            w: tiles.map(\.w),
            h: tiles.map(\.h),
        )
        boardStore.saveCustom(board, family: family)
        sessionId = board.id
        sessionTiles = tiles
        sessionDock = dock
        boardStore.editing = false
    }

    private func deleteCustom() {
        boardStore.deleteCustom(id: liveId, family: family)
        sessionTiles = nil
        sessionDock = nil
        sessionId = nil
    }

    static func cashTiles(from custom: BarPretradeCustomBoard, asset: BarDeclareAssetClass) -> [BarCashCockpitSeed.Tile] {
        let allow = Set(BarCashCockpitSeed.catalog(for: asset).map(\.rawValue))
        var seen = Set<String>()
        var out: [BarCashCockpitSeed.Tile] = []
        let n = min(custom.kinds.count, min(custom.x.count, min(custom.y.count, min(custom.w.count, custom.h.count))))
        for i in 0 ..< n {
            let raw = custom.kinds[i]
            guard allow.contains(raw), !seen.contains(raw), let kind = BarCashCockpitSeed.Kind(rawValue: raw) else {
                continue
            }
            seen.insert(raw)
            out.append(BarCashCockpitSeed.Tile(kind: kind, x: custom.x[i], y: custom.y[i], w: custom.w[i], h: custom.h[i]))
        }
        return out.isEmpty ? BarCashCockpitSeed.tiles(for: asset) : out
    }
}
