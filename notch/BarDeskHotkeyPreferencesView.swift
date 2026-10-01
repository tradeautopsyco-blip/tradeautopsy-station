import SwiftUI

struct BarDeskHotkeyPreferencesView: View {
    @State private var toggleKeyCode: String = ""
    @State private var openKeyCode: String = ""
    @State private var savedMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarSectionLabel(text: "Hotkeys")
            Text("Record key codes for each action. ⌥Space / ⌥⇧Space stay the system defaults until Station applies custom bindings in a future build — no invented default map.")
                .font(BarDS.bodyFont(11, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
            ForEach(DeskHotkeyPreferences.configurableActions, id: \.id) { action in
                HStack {
                    Text(action.label)
                        .font(BarDS.bodyFont(12, weight: .semibold))
                    Spacer()
                    BarInputField(
                        placeholder: "Key code",
                        text: binding(for: action.id)
                    )
                    .frame(maxWidth: 100)
                }
            }
            Button("Save bindings") {
                save()
            }
            .buttonStyle(.plain)
            .font(BarDS.bodyFont(11, weight: .semibold))
            .foregroundColor(BarDS.Accent.teal)
            if let savedMessage {
                Text(savedMessage)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal)
            }
        }
    }

    private func binding(for actionId: String) -> Binding<String> {
        switch actionId {
        case "toggle_notch": return $toggleKeyCode
        case "open_station": return $openKeyCode
        default: return .constant("")
        }
    }

    private func save() {
        var items: [DeskHotkeyBinding] = []
        if let code = UInt32(toggleKeyCode.trimmingCharacters(in: .whitespacesAndNewlines)), code > 0 {
            items.append(DeskHotkeyBinding(actionId: "toggle_notch", keyCode: code, carbonModifiers: UInt32(optionKey)))
        }
        if let code = UInt32(openKeyCode.trimmingCharacters(in: .whitespacesAndNewlines)), code > 0 {
            items.append(DeskHotkeyBinding(actionId: "open_station", keyCode: code, carbonModifiers: UInt32(optionKey | shiftKey)))
        }
        DeskHotkeyPreferences.save(items)
        savedMessage = items.isEmpty ? "Cleared custom bindings." : "Saved \(items.count) binding(s)."
    }
}

private let optionKey: Int = 0x800
private let shiftKey: Int = 0x200
