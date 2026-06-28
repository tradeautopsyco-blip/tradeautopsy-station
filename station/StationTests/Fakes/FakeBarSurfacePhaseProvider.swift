import Foundation
import Notch
@testable import Station

@MainActor
final class FakeBarSurfacePhaseProvider: BarSurfacePhaseProviding {
    var barSurfacePhase: BarSurfacePhase = .declaration
    var onPhaseChange: ((BarSurfacePhase) -> Void)?

    func setPhase(_ phase: BarSurfacePhase) {
        barSurfacePhase = phase
        onPhaseChange?(phase)
    }
}
