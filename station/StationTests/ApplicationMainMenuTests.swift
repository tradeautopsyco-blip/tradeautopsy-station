import AppKit
import Testing
@testable import Station

@MainActor
struct ApplicationMainMenuTests {
    @Test func buildProvidesEditMenuWithPasteShortcut() {
        let editMenu = ApplicationMainMenu.editMenu(in: ApplicationMainMenu.build())
        #expect(editMenu != nil)

        let pasteItem = editMenu?.items.first { $0.title == "Paste" }
        #expect(pasteItem != nil)
        #expect(pasteItem?.keyEquivalent == "v")
        #expect(pasteItem?.action == #selector(NSText.paste(_:)))
    }
}
