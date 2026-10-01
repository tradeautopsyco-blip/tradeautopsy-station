import SwiftUI

struct BarDeskHotkeyPreferencesView: View {
    @State private var keyCodes: [String: String] = [:]
    @State private var savedMessage: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            BarSectionLabel(text: "Hotkeys")
            Text("Type a key code and save. Blank rows stay unbound. With no saved rows, ⌥Space still toggles Notch and ⌥⇧Space still opens Station. Nothing here binds Kill, Confirm, or Cancel until you type a code. Saved rows use ⌥, except Open Station which uses ⌥⇧.")
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
        .onAppear(perform: load)
    }

    private func binding(for actionId: String) -> Binding<String> {
        Binding(
            get: { keyCodes[actionId] ?? "" },
            set: { keyCodes[actionId] = $0 }
        )
    }

    private func load() {
        var map: [String: String] = [:]
        for row in DeskHotkeyPreferences.load() {
            map[row.actionId] = String(row.keyCode)
        }
        keyCodes = map
    }

    private func save() {
        var items: [DeskHotkeyBinding] = []
        for action in DeskHotkeyPreferences.configurableActions {
            let raw = (keyCodes[action.id] ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            guard let code = UInt32(raw), code > 0 else { continue }
            let modifiers: UInt32 = action.id == "open_station"
                ? UInt32(optionKey | shiftKey)
                : UInt32(optionKey)
            items.append(
                DeskHotkeyBinding(
                    actionId: action.id,
                    keyCode: code,
                    carbonModifiers: modifiers
                )
            )
        }
        DeskHotkeyPreferences.save(items)
        savedMessage = items.isEmpty ? "Cleared custom bindings." : "Saved \(items.count) binding(s)."
    }
}

private let optionKey: Int = 0x800
private let shiftKey: Int = 0x200
