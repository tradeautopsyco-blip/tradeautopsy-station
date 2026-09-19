import SwiftUI

/// Pre-trade cockpit for Kotak NFO **options** (OPT*). Mosaic + glance strip + Plan dock.
/// Futures and unknown kind stay on `BarOptionsDeclareView` (three-zone).
/// Confirm is LiveBook intent — TRADE parked, no COM ticket.
struct BarNfoOptionsCockpitView: View {
    @ObservedObject var viewModel: NotchViewModel
    @Binding var sideBuy: Bool
    @Binding var stopLossText: String
    @Binding var targetPriceText: String

    let submitReady: Bool
    let submitHint: String?
    let onConfirm: () -> Void

    @ObservedObject private var boardStore = BarPretradeBoardStore.shared
    @State private var sessionTiles: [BarNfoCockpitSeed.Tile]?
    @State private var sessionDock: BarCockpitDock?
    @State private var sessionId: String?

    private var family: BarPretradeBoardFamily { .nfo }

    private var liveId: String { boardStore.currentBoardId(for: family) }

    private var liveTiles: [BarNfoCockpitSeed.Tile] {
        if sessionId == liveId, let sessionTiles { return sessionTiles }
        return resolvedTiles()
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
            BarOptionsGlanceStrip(viewModel: viewModel)
            if boardStore.editing {
                BarPretradeEditTray(
                    unusedTitles: unusedCatalog.map { ($0.rawValue, BarNfoCockpitSeed.title(for: $0)) },
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
    }

    @ViewBuilder
    private var boardWork: some View {
        let mosaic = BarNfoCockpitMosaic(
            viewModel: viewModel,
            tiles: liveTiles,
            editing: boardStore.editing,
            onRemove: removeKind,
        )
        let plan = BarNfoCockpitPlanRail(
            viewModel: viewModel,
            sideBuy: $sideBuy,
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
            case .cockpit: return "Cockpit mosaic"
            case .hero: return "Hero + twins"
            case .focus: return "Focus + floor"
            }
        }
        return boardStore.customBoard(id: liveId, family: family)?.name ?? "Board"
    }

    private var liveBlurb: String {
        "\(liveTiles.count) tiles · Plan \(liveDock == .floor ? "floor" : "rail") · DualNoBlend INR"
    }

    private var unusedCatalog: [BarNfoCockpitSeed.Kind] {
        let present = Set(liveTiles.map(\.kind))
        return BarNfoCockpitSeed.catalog.filter { !present.contains($0) }
    }

    private func resolvedTiles() -> [BarNfoCockpitSeed.Tile] {
        if let seed = BarCockpitBoardId(rawValue: liveId) {
            return BarNfoCockpitSeed.tiles(for: seed)
        }
        guard let custom = boardStore.customBoard(id: liveId, family: family) else {
            return BarNfoCockpitSeed.tiles
        }
        return Self.nfoTiles(from: custom)
    }

    private func resolvedDock() -> BarCockpitDock {
        if let seed = BarCockpitBoardId(rawValue: liveId) {
            return BarNfoCockpitSeed.planDock(for: seed)
        }
        if let custom = boardStore.customBoard(id: liveId, family: family),
           let dock = BarCockpitDock(rawValue: custom.planDock)
        {
            return dock
        }
        return .rail
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
        guard let kind = BarNfoCockpitSeed.Kind(rawValue: raw) else { return }
        beginSession()
        var tiles = sessionTiles ?? resolvedTiles()
        guard !tiles.contains(where: { $0.kind == kind }) else { return }
        let size = BarNfoCockpitSeed.addSize(for: kind)
        let occupied = tiles.map {
            BarCockpitTicketOverlay.Occupancy(x: $0.x, y: $0.y, w: $0.w, h: $0.h)
        }
        let hole = BarCockpitTicketOverlay.firstHole(occupied, w: size.w, h: size.h)
        tiles.append(BarNfoCockpitSeed.Tile(kind: kind, x: hole.x, y: hole.y, w: hole.w, h: hole.h))
        sessionTiles = tiles
    }

    private func removeKind(_ kind: BarNfoCockpitSeed.Kind) {
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
        let tiles = sessionTiles ?? resolvedTiles()
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

    static func nfoTiles(from custom: BarPretradeCustomBoard) -> [BarNfoCockpitSeed.Tile] {
        let allow = Set(BarNfoCockpitSeed.catalog.map(\.rawValue))
        var seen = Set<String>()
        var out: [BarNfoCockpitSeed.Tile] = []
        let n = min(custom.kinds.count, min(custom.x.count, min(custom.y.count, min(custom.w.count, custom.h.count))))
        for i in 0 ..< n {
            let raw = custom.kinds[i]
            guard allow.contains(raw), !seen.contains(raw), let kind = BarNfoCockpitSeed.Kind(rawValue: raw) else {
                continue
            }
            seen.insert(raw)
            out.append(BarNfoCockpitSeed.Tile(kind: kind, x: custom.x[i], y: custom.y[i], w: custom.w[i], h: custom.h[i]))
        }
        return out.isEmpty ? BarNfoCockpitSeed.tiles : out
    }
}
