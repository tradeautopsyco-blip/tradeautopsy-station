import Foundation
import Notch

@MainActor
public final class DefaultBarSurfacePhaseProvider: BarSurfacePhaseProviding {
    public var barSurfacePhase: BarSurfacePhase = .declaration
    public var onPhaseChange: ((BarSurfacePhase) -> Void)?

    public init(initialPhase: BarSurfacePhase = .declaration) {
        self.barSurfacePhase = initialPhase
    }

    public func setPhase(_ phase: BarSurfacePhase) {
        guard barSurfacePhase != phase else { return }
        barSurfacePhase = phase
        onPhaseChange?(phase)
    }
}
