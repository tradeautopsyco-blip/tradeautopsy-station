import CoreGraphics
import Foundation

/// Closed-island tug on a notched Mac: 1:1 follow, rubber-band past the slot, spring home.
/// Spring-home target is always `collapsedFrame.origin` — never a saved parked origin.
enum CollapsedIslandMotion {
    static let rubberbandConstant: CGFloat = 0.55
    static let decelerationRate: CGFloat = 0.998
    static let flickSpeed: CGFloat = 420
    static let springResponse: CGFloat = 0.35
    static let settledDamping: CGFloat = 1.0
    static let flickDamping: CGFloat = 0.8

    /// Apple *Designing Fluid Interfaces* rubber-band: progressive resistance past an edge.
    static func rubberband(
        overshoot: CGFloat,
        dimension: CGFloat,
        constant: CGFloat = rubberbandConstant,
    ) -> CGFloat {
        guard dimension > 0 else { return 0 }
        return (overshoot * dimension * constant) / (dimension + constant * abs(overshoot))
    }

    /// Apple projection: `current + (v/1000)·d/(1−d)`, `d ≈ 0.998`.
    static func project(
        velocity: CGFloat,
        decelerationRate: CGFloat = decelerationRate,
    ) -> CGFloat {
        (velocity / 1000) * decelerationRate / (1 - decelerationRate)
    }

    /// Rubber-band a 1:1 unconstrained origin around the volume slot.
    static func rubberbandedOrigin(unconstrained: CGPoint, slot: CGRect) -> CGPoint {
        CGPoint(
            x: slot.origin.x + rubberband(
                overshoot: unconstrained.x - slot.origin.x,
                dimension: slot.width,
            ),
            y: slot.origin.y + rubberband(
                overshoot: unconstrained.y - slot.origin.y,
                dimension: slot.height,
            ),
        )
    }

    /// Home is the slot. A parked origin on a notched Mac is a second blob.
    static func springHomeTarget(slot: CGRect) -> CGPoint {
        slot.origin
    }

    static func dampingRatio(releaseSpeed: CGFloat) -> CGFloat {
        releaseSpeed > flickSpeed ? flickDamping : settledDamping
    }

    static func springAccel(
        position: CGFloat,
        velocity: CGFloat,
        target: CGFloat,
        response: CGFloat = springResponse,
        dampingRatio: CGFloat,
    ) -> CGFloat {
        let omega = (2 * CGFloat.pi) / response
        let damp = 2 * dampingRatio * omega
        return -omega * omega * (position - target) - damp * velocity
    }

    struct SpringState: Equatable {
        var x: CGFloat
        var y: CGFloat
        var vx: CGFloat
        var vy: CGFloat
        var target: CGPoint
        var dampingRatio: CGFloat

        mutating func step(dt: CGFloat) {
            let ax = CollapsedIslandMotion.springAccel(
                position: x,
                velocity: vx,
                target: target.x,
                dampingRatio: dampingRatio,
            )
            let ay = CollapsedIslandMotion.springAccel(
                position: y,
                velocity: vy,
                target: target.y,
                dampingRatio: dampingRatio,
            )
            vx += ax * dt
            vy += ay * dt
            x += vx * dt
            y += vy * dt
            if abs(x - target.x) < 0.15, abs(vx) < 4 { x = target.x; vx = 0 }
            if abs(y - target.y) < 0.15, abs(vy) < 4 { y = target.y; vy = 0 }
        }

        var isSettled: Bool {
            abs(x - target.x) < 0.15
                && abs(y - target.y) < 0.15
                && abs(vx) < 4
                && abs(vy) < 4
        }
    }
}
