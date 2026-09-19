import SwiftUI

/// Board switcher + Edit board / Done. Confirm stays on the Plan dock.
struct BarPretradeBoardChrome: View {
    let title: String
    let blurb: String
    let seedIds: [BarCockpitBoardId]
    let currentId: String
    let editing: Bool
    let canDelete: Bool
    let onSelect: (String) -> Void
    let onToggleEdit: () -> Void
    let onReset: () -> Void
    let onSaveAs: () -> Void
    let onDelete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(spacing: 8) {
                ForEach(seedIds, id: \.rawValue) { seed in
                    chip(
                        label: seedLabel(seed),
                        on: currentId == seed.rawValue,
                        action: { onSelect(seed.rawValue) },
                    )
                }
                Spacer(minLength: 8)
                chip(label: editing ? "Done" : "Edit board", on: editing, action: onToggleEdit)
                if editing {
                    chip(label: "Save as", on: false, action: onSaveAs)
                    chip(label: "Reset", on: false, action: onReset)
                    if canDelete {
                        chip(label: "Delete", on: false, action: onDelete)
                    }
                }
            }
            Text(blurb)
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private func seedLabel(_ seed: BarCockpitBoardId) -> String {
        switch seed {
        case .cockpit: return "Cockpit"
        case .hero: return "Hero"
        case .focus: return "Focus"
        }
    }

    private func chip(label: String, on: Bool, action: @escaping () -> Void) -> some View {
        Button(action: action) {
            Text(label)
                .font(BarDS.monoFont(10, weight: .medium))
                .foregroundColor(on ? BarDS.Text.primary : BarDS.Text.muted)
                .padding(.vertical, 5)
                .padding(.horizontal, 8)
                .background(BarDS.Fill.card)
                .clipShape(RoundedRectangle(cornerRadius: 8, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: 8, style: .continuous)
                        .stroke(on ? BarDS.Text.primary.opacity(0.35) : BarDS.Border.card, lineWidth: BarDS.borderThin),
                )
        }
        .buttonStyle(.plain)
    }
}

struct BarPretradeEditTray: View {
    let unusedTitles: [(id: String, title: String)]
    let dock: BarCockpitDock
    let onAdd: (String) -> Void
    let onDock: (BarCockpitDock) -> Void

    var body: some View {
        HStack(spacing: 8) {
            Text("Catalog")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.8)
            if unusedTitles.isEmpty {
                Text("All tiles on the board · Plan stays")
                    .font(BarDS.monoFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            } else {
                ForEach(unusedTitles, id: \.id) { item in
                    Button("＋ \(item.title)") { onAdd(item.id) }
                        .font(BarDS.monoFont(10, weight: .medium))
                        .buttonStyle(.plain)
                        .foregroundColor(BarDS.Text.primary)
                }
            }
            Spacer(minLength: 8)
            Text("Plan dock")
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.8)
            dockChip("Rail", .rail)
            dockChip("Floor", .floor)
        }
    }

    private func dockChip(_ label: String, _ value: BarCockpitDock) -> some View {
        Button(label) { onDock(value) }
            .font(BarDS.monoFont(10, weight: .medium))
            .foregroundColor(dock == value ? BarDS.Text.primary : BarDS.Text.muted)
            .buttonStyle(.plain)
    }
}
