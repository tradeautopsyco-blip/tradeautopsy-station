import CoreGraphics
import Foundation
import Testing
@testable import Notch

@MainActor
struct CollapsedPillDragReducerTests {
    @Test func endedBelowHysteresisExpands() {
        var reducer = CollapsedPillDragReducer()
        #expect(reducer.changed(translation: .zero) == .none)
        #expect(reducer.ended(translation: CGSize(width: 3, height: 4)) == .expand)
        #expect(CollapsedPillDragReducer.distance(CGSize(width: 3, height: 4)) == 5)
    }

    @Test func ninePointsStillExpands() {
        var reducer = CollapsedPillDragReducer()
        let t = CGSize(width: 9, height: 0)
        #expect(reducer.changed(translation: t) == .none)
        #expect(reducer.ended(translation: t) == .expand)
    }

    @Test func tenPointsStartsMove() {
        var reducer = CollapsedPillDragReducer()
        let t = CGSize(width: 10, height: 0)
        #expect(reducer.changed(translation: t) == .move)
        #expect(reducer.ended(translation: t) == .moveEnded)
    }

    @Test func returningInsideHysteresisStaysMove() {
        var reducer = CollapsedPillDragReducer()
        #expect(reducer.changed(translation: CGSize(width: 20, height: 0)) == .move)
        #expect(reducer.changed(translation: .zero) == .move)
        #expect(reducer.ended(translation: .zero) == .moveEnded)
    }
}

@MainActor
struct CollapsedPillDragCallbackTests {
    @Test func dragCallbacksFireWithoutExpanding() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var fromScreen = 0
        var ended = 0
        vm.onCollapsedPillDragFromScreen = { fromScreen += 1 }
        vm.onCollapsedPillDragEnded = { ended += 1 }

        vm.applyCollapsedPillDragFromScreen()
        vm.endCollapsedPillDrag()

        #expect(fromScreen == 1)
        #expect(ended == 1)
        #expect(vm.isExpanded == false)
    }

    @Test func dragFromScreenIgnoredWhileExpanded() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        var fromScreen = 0
        vm.onCollapsedPillDragFromScreen = { fromScreen += 1 }
        vm.expandFromCollapsedChromeTap()
        vm.applyCollapsedPillDragFromScreen()
        #expect(fromScreen == 0)
        #expect(vm.isExpanded == true)
    }
}
