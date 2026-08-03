import Foundation

@MainActor
public final class DefaultSessionSurfacePhaseProvider: SessionSurfacePhaseProviding {
    public var sessionSurfacePhase: SessionSurfacePhase = .declaration
    public var onPhaseChange: ((SessionSurfacePhase) -> Void)?

    public init(initialPhase: SessionSurfacePhase = .declaration) {
        self.sessionSurfacePhase = initialPhase
    }

    public func setPhase(_ phase: SessionSurfacePhase) {
        guard sessionSurfacePhase != phase else { return }
        sessionSurfacePhase = phase
        onPhaseChange?(phase)
    }
}
