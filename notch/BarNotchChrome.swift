import CoreGraphics
import SwiftUI

// MARK: - Notch unified build reference (May 2026) — layout contracts for Bar surfaces
// Bar / PLAN views should use these instead of hardcoding strip heights, rings, or intervention sizing
// so chrome stays consistent (CollapsedView, BarEscrowMatchView, intervention rows).

enum BarNotchChrome {
    /// HTML `.pill` height (32px).
    static let collapsedStripHeight: CGFloat = 32
    /// Hug `dot + 0.00 + CALM` — not a stretched 200pt slab.
    static let collapsedPillWidth: CGFloat = 128

    /// Escrow fidelity / match status ring (diameter in points @1x design baseline).
    static let escrowStatusRingDiameter: CGFloat = 60

    /// Single-word intervention keyword on collapsed pill (matches `CollapsedNotchPresentation` ladder).
    static let interventionKeywordPointSize: CGFloat = 11

    /// Uses `NotchTheme.rounded` for intervention keywords (single-line center in `CollapsedNotchView`).
    static func interventionKeywordFont() -> Font {
        NotchTheme.rounded(interventionKeywordPointSize, weight: .bold)
    }
}
