import Foundation
import Testing
@testable import Notch

/// Prototype options glance `strip()`: Last / Index S / Delta / Gamma·Theta / OI / Margin.
/// Honesty map — not SwiftUI.
struct BarOptionsGlanceStripTests {
    @Test func kindsMatchPrototypeSixCells() {
        #expect(BarOptionsGlanceStripSeed.kinds == [
            .last, .index, .delta, .gammaTheta, .oi, .margin,
        ])
    }

    @Test func cryptoIndexPublishesVenueString() {
        let inputs = BarOptionsGlanceHonesty.Inputs.crypto(
            indexStatus: "success",
            indexPrice: "114250.12",
        )
        #expect(BarOptionsGlanceHonesty.value(.index, inputs: inputs) == .text("114250.12"))
    }

    @Test func nfoIndexStaysUnavailableChip() {
        let inputs = BarOptionsGlanceHonesty.Inputs.nfo()
        #expect(BarOptionsGlanceHonesty.value(.index, inputs: inputs) == .chip(.unavailable))
    }

    @Test func cryptoDeltaPublishesVenueString() {
        let inputs = BarOptionsGlanceHonesty.Inputs.crypto(
            greeksDisplay: true,
            greeksStatus: "success",
            delta: "0.42",
        )
        #expect(BarOptionsGlanceHonesty.value(.delta, inputs: inputs) == .text("0.42"))
    }

    @Test func nfoDeltaStaysUnavailableChip() {
        #expect(BarOptionsGlanceHonesty.value(.delta, inputs: .nfo()) == .chip(.unavailable))
    }

    @Test func cryptoGammaThetaJoinsVenueStrings() {
        let inputs = BarOptionsGlanceHonesty.Inputs.crypto(
            greeksDisplay: true,
            greeksStatus: "success",
            gamma: "0.01",
            theta: "-0.02",
        )
        #expect(BarOptionsGlanceHonesty.value(.gammaTheta, inputs: inputs) == .text("0.01 · -0.02"))
    }

    @Test func nfoGammaThetaStaysUnavailableChip() {
        #expect(BarOptionsGlanceHonesty.value(.gammaTheta, inputs: .nfo()) == .chip(.unavailable))
    }

    @Test func cryptoOiPublishesSumOpenInterestDigitForDigit() {
        let inputs = BarOptionsGlanceHonesty.Inputs.crypto(oi: "123456")
        #expect(BarOptionsGlanceHonesty.value(.oi, inputs: inputs) == .text("123456"))
    }

    @Test func nfoOiZeroIsAReading() {
        let inputs = BarOptionsGlanceHonesty.Inputs.nfo(oi: "0")
        #expect(BarOptionsGlanceHonesty.value(.oi, inputs: inputs) == .text("0"))
    }

    @Test func nfoOiNoteDoesNotNameEapi() {
        #expect(!BarOptionsGlanceCopy.nfoOiNote.contains("sumOpenInterest"))
        #expect(!BarOptionsGlanceCopy.nfoOiNote.contains("eapi"))
        #expect(BarOptionsGlanceCopy.nfoOiNote.contains("open_int"))
    }

    @Test func marginIsUnavailableOnBothDesks() {
        #expect(BarOptionsGlanceHonesty.value(.margin, inputs: .crypto()) == .chip(.unavailable))
        #expect(BarOptionsGlanceHonesty.value(.margin, inputs: .nfo()) == .chip(.unavailable))
    }
}
