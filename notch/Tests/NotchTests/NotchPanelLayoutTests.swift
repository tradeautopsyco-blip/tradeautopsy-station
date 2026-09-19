import CoreGraphics
import Foundation
import Testing
@testable import Notch

struct NotchPanelLayoutTests {
    @Test func keepsLockedSizeWhenVisibleFrameUnchanged() {
        let vf = CGRect(x: 0, y: 0, width: 1512, height: 944)
        let locked = CGSize(width: 800, height: 500)
        let resolved = NotchPanelLayout.resolvedExpandedContentSize(
            locked: locked,
            lockedVisibleFrame: vf,
            currentVisibleFrame: vf
        )
        #expect(resolved.width == 800)
        #expect(resolved.height == 500)
    }

    @Test func recomputesWhenVisibleFrameSizeChanges() {
        let lockedVf = CGRect(x: 0, y: 0, width: 800, height: 600)
        let locked = CGSize(width: 752, height: 536)
        let current = CGRect(x: 0, y: 0, width: 1512, height: 944)
        let resolved = NotchPanelLayout.resolvedExpandedContentSize(
            locked: locked,
            lockedVisibleFrame: lockedVf,
            currentVisibleFrame: current
        )
        #expect(resolved.width == 1464)
        #expect(resolved.height == 880)
    }

    @Test func recomputesWhenVisibleFrameOriginChanges() {
        let lockedVf = CGRect(x: 0, y: 0, width: 1512, height: 944)
        let locked = CGSize(width: 800, height: 500)
        let current = CGRect(x: 1920, y: 0, width: 1512, height: 944)
        let resolved = NotchPanelLayout.resolvedExpandedContentSize(
            locked: locked,
            lockedVisibleFrame: lockedVf,
            currentVisibleFrame: current
        )
        #expect(resolved.width == 1464)
        #expect(resolved.height == 880)
    }

    @Test func computesFromCurrentFrameWhenUnlocked() {
        let current = CGRect(x: 0, y: 0, width: 1512, height: 944)
        let resolved = NotchPanelLayout.resolvedExpandedContentSize(
            locked: nil,
            lockedVisibleFrame: nil,
            currentVisibleFrame: current
        )
        #expect(resolved.width == 1464)
        #expect(resolved.height == 880)
    }

    @Test func keepsLockWhenVisibleFrameDiffersBySubpixel() {
        let lockedVf = CGRect(x: 0, y: 0, width: 1512, height: 944)
        let locked = CGSize(width: 800, height: 500)
        let current = CGRect(x: 0.2, y: -0.1, width: 1512.1, height: 943.9)
        let resolved = NotchPanelLayout.resolvedExpandedContentSize(
            locked: locked,
            lockedVisibleFrame: lockedVf,
            currentVisibleFrame: current
        )
        #expect(resolved.width == 800)
        #expect(resolved.height == 500)
    }
}
