import Foundation
import Testing
@testable import Notch

/// Wave 5 — cancel declaration control on armed PLAN (not hard delete).
struct BarCancelDeclarationChromeTests {
    @Test func controlVisibleWhenArmedWithDeclarationId() {
        #expect(
            BarCancelDeclarationChrome.showsControl(phase: .armed, declarationId: "550e8400-e29b-41d4-a716-446655440000")
                == true,
        )
    }

    @Test func controlHiddenWhenNotArmed() {
        #expect(
            BarCancelDeclarationChrome.showsControl(phase: .declaration, declarationId: "550e8400-e29b-41d4-a716-446655440000)
                == false,
        )
    }

    @Test func controlHiddenWithoutDeclarationId() {
        #expect(BarCancelDeclarationChrome.showsControl(phase: .armed, declarationId: nil) == false)
        #expect(BarCancelDeclarationChrome.showsControl(phase: .armed, declarationId: "  ") == false)
    }

    @Test func entryCopyIsCancelDeclarationNotDelete() {
        #expect(BarCancelDeclarationChrome.entryButtonTitle == "Cancel declaration")
        #expect(!BarCancelDeclarationChrome.entryButtonTitle.lowercased().contains("delete"))
        #expect(BarCancelDeclarationChrome.notFlattenHint.lowercased().contains("journal"))
    }

    @Test func reasonChipsIncludeHarnessScratchSlug() {
        let slugs = BarCancelDeclarationChrome.reasonChips.map(\.slug)
        #expect(slugs.contains("scratch"))
    }

    @Test func normalizedReasonChipRejectsEmpty() {
        #expect(BarCancelDeclarationChrome.normalizedReasonChip("") == nil)
        #expect(BarCancelDeclarationChrome.normalizedReasonChip("  ") == nil)
        #expect(BarCancelDeclarationChrome.normalizedReasonChip("scratch") == "scratch")
    }
}
