import Foundation
@testable import Station

@MainActor
final class FakeSessionSurfacePhaseProvider: SessionSurfacePhaseProviding {
    var sessionSurfacePhase: SessionSurfacePhase = .declaration
    var onPhaseChange: ((SessionSurfacePhase) -> Void)?

    func setPhase(_ phase: SessionSurfacePhase) {
        sessionSurfacePhase = phase
        onPhaseChange?(phase)
    }
}
