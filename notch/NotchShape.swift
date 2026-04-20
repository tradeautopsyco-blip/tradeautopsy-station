import SwiftUI

/// Interpolates corner radius between collapsed pill and expanded panel.
struct NotchShape: Shape {
    var expansionProgress: CGFloat

    var animatableData: CGFloat {
        get { expansionProgress }
        set { expansionProgress = newValue }
    }

    func path(in rect: CGRect) -> Path {
        let collapsedRadius: CGFloat = 16
        let expandedRadius: CGFloat = 22
        let radius = collapsedRadius + (expandedRadius - collapsedRadius) * expansionProgress
        return Path(roundedRect: rect, cornerRadius: radius, style: .continuous)
    }
}
