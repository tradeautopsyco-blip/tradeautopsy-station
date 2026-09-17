import CoreGraphics
import Testing
@testable import Notch

struct CollapsedIslandMotionTests {
    @Test func rubberbandZeroOvershootIsZero() {
        #expect(CollapsedIslandMotion.rubberband(overshoot: 0, dimension: 200) == 0)
    }

    @Test func rubberbandResistsPastTheSlot() {
        let pulled: CGFloat = 100
        let resisted = CollapsedIslandMotion.rubberband(overshoot: pulled, dimension: 200)
        #expect(resisted > 0)
        #expect(resisted < pulled)
    }

    @Test func projectUsesAppleDeceleration() {
        let projected = CollapsedIslandMotion.project(velocity: 1000)
        #expect(abs(projected - 499) < 0.5)
    }

    @Test func springHomeTargetIsAlwaysSlotOrigin() {
        let slot = CGRect(x: 656, y: 958, width: 240, height: 24)
        let parked = CGPoint(x: 10, y: 10)
        #expect(CollapsedIslandMotion.springHomeTarget(slot: slot) == slot.origin)
        #expect(CollapsedIslandMotion.springHomeTarget(slot: slot) != parked)
    }

    @Test func rubberbandedOriginStaysShortOfUnconstrained() {
        let slot = CGRect(x: 100, y: 900, width: 244, height: 24)
        let unconstrained = CGPoint(x: 200, y: 850)
        let banded = CollapsedIslandMotion.rubberbandedOrigin(unconstrained: unconstrained, slot: slot)
        #expect(abs(banded.x - slot.origin.x) < abs(unconstrained.x - slot.origin.x))
        #expect(abs(banded.y - slot.origin.y) < abs(unconstrained.y - slot.origin.y))
    }

    @Test func flickUsesUnderdampedSpring() {
        #expect(CollapsedIslandMotion.dampingRatio(releaseSpeed: 500) == 0.8)
        #expect(CollapsedIslandMotion.dampingRatio(releaseSpeed: 100) == 1.0)
    }

    @Test func springSettlesOnSlotOrigin() {
        var spring = CollapsedIslandMotion.SpringState(
            x: 700,
            y: 940,
            vx: 0,
            vy: 0,
            target: CGPoint(x: 656, y: 958),
            dampingRatio: 1.0,
        )
        for _ in 0..<180 {
            spring.step(dt: 1.0 / 60.0)
            if spring.isSettled { break }
        }
        #expect(spring.isSettled)
        #expect(abs(spring.x - 656) < 0.2)
        #expect(abs(spring.y - 958) < 0.2)
    }
}

struct ClosedNotchChromeContractTests {
    @Test func interventionKeywordIsElevenPoint() {
        #expect(BarNotchChrome.interventionKeywordPointSize == 11)
    }

    @Test func collapsedStripHeightIsNonNotchOnly() {
        #expect(BarNotchChrome.collapsedStripHeight == 32)
    }
}
