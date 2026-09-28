import SwiftUI

/// Workspace rail + Book autopsy report. Uses StationDS; teal only for selection / primary.
enum WorkspaceChrome {
    static let railWidth: CGFloat = 232
    static let railCollapsedWidth: CGFloat = 52

    static let ground = StationDS.Fill.appPanel
    static let rail = StationDS.Fill.sidebar
    static let card = StationDS.Fill.elevated
    static let text = StationDS.Text.primary
    static let muted = StationDS.Text.secondary
    static let faint = StationDS.Text.muted
    static let selection = Color.white.opacity(0.08)
    static let line = StationDS.Border.subtle
    static let accent = StationDS.Accent.teal
    static let profit = StationDS.Accent.green
    static let loss = StationDS.Accent.red

    static func metricFont(_ size: CGFloat = 28) -> Font {
        StationDS.monoFont(size, weight: .medium)
    }
}
