import CoreGraphics
import Testing
@testable import Notch

struct BarNotchVolumeSlotTests {
    @Test func islandCentersOnHousingAndHangsBelowCamera() {
        let screen = CGRect(x: 0, y: 0, width: 1512, height: 982)
        let visible = CGRect(x: 0, y: 0, width: 1512, height: 938)
        let left = CGRect(x: 0, y: 950, width: 658, height: 32)
        let right = CGRect(x: 854, y: 950, width: 658, height: 32)
        let housing = BarNotchVolumeSlot.hardwareNotch(
            screenFrame: screen,
            leftMenu: left,
            rightMenu: right,
        )
        #expect(housing?.width == 200)
        #expect(housing?.minX == 656)

        let rect = BarNotchVolumeSlot.collapsedFrame(
            screenFrame: screen,
            visibleFrame: visible,
            notchLeft: housing!.minX,
            notchWidth: housing!.width,
            notchInset: 38,
            fallbackWidth: 176,
        )
        #expect(rect.maxY == screen.maxY)
        #expect(rect.height == 44 + BarNotchVolumeSlot.hang)
        #expect(rect.width == 200)
        #expect(rect.minX == housing!.minX)
        #expect(abs(rect.midX - housing!.midX) < 0.5)
    }

    @Test func closedWidthDoesNotGrowPastHousing() {
        let screen = CGRect(x: 0, y: 0, width: 1512, height: 982)
        let visible = CGRect(x: 0, y: 0, width: 1512, height: 938)
        let rect = BarNotchVolumeSlot.collapsedFrame(
            screenFrame: screen,
            visibleFrame: visible,
            notchLeft: 656,
            notchWidth: 196,
            notchInset: 38,
            fallbackWidth: 176,
        )
        #expect(rect.width == 196)
        #expect(rect.minX == 656)
        #expect(rect.maxX == 852)
    }

    @Test func fullWidthGapIsNotANotch() {
        let screen = CGRect(x: 0, y: 0, width: 1512, height: 982)
        let housing = BarNotchVolumeSlot.hardwareNotch(
            screenFrame: screen,
            leftMenu: .zero,
            rightMenu: .zero,
        )
        #expect(housing == nil)
    }
}
