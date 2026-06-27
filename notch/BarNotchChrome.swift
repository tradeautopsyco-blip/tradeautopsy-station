import CoreGraphics
import SwiftUI

// MARK: - Notch unified build reference (May 2026) — layout contracts for Bar surfaces
// Bar / PLAN views should use these instead of hardcoding strip heights, rings, or intervention sizing
// so chrome stays consistent (CollapsedView, BarEscrowMatchView, intervention rows).

enum BarNotchChrome {
    /// Collapsed daemon strip — target from mockups; adjust only for hardware notch overlap (document deltas).
    static let collapsedStripHeight: CGFloat = 28

    /// Escrow fidelity / match status ring (diameter in points @1x design baseline).
    static let escrowStatusRingDiameter: CGFloat = 60

    /// Single-word intervention keyword on collapsed pill (matches `CollapsedNotchPresentation` ladder).
    static let interventionKeywordPointSize: CGFloat = 11

    /// Uses `NotchTheme.rounded` for intervention keywords (single-line center in `CollapsedNotchView`).
    static func interventionKeywordFont() -> Font {
        NotchTheme.rounded(interventionKeywordPointSize, weight: .bold)
    }
}
