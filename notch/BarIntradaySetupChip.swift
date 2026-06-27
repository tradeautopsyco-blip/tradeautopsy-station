import Foundation

/// Setup taxonomy chips — unified reference mockup 3 (`2026-05-16-notch-unified-build-reference.md`).
enum BarIntradaySetupChip: String, CaseIterable, Identifiable, Equatable {
    case breakout = "Breakout"
    case pullback = "Pullback"
    case reversal = "Reversal"
    case gapFill = "Gap fill"
    case trendContinuation = "Trend continuation"
    case meanReversion = "Mean reversion"
    case supportBounce = "Support bounce"
    case rangeBreak = "Range break"

    var id: String { rawValue }
}
