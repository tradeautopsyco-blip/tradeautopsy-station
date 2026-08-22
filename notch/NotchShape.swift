import SwiftUI

/// Interpolates between the hardware-notch chin (closed) and the expanded panel.
struct NotchShape: Shape {
    var expansionProgress: CGFloat
    /// BoringNotch closed silhouette: flush top, rounded bottom chins.
    var hardwareChin: Bool = false

    var animatableData: CGFloat {
        get { expansionProgress }
        set { expansionProgress = newValue }
    }

    func path(in rect: CGRect) -> Path {
        if hardwareChin, expansionProgress < 0.01 {
            return Self.chinPath(in: rect, topCornerRadius: 6, bottomCornerRadius: 14)
        }
        let collapsedRadius: CGFloat = 20
        let expandedRadius: CGFloat = 18
        let radius = collapsedRadius + (expandedRadius - collapsedRadius) * expansionProgress
        return Path(roundedRect: rect, cornerRadius: radius, style: .continuous)
    }

    /// TheBoredTeam/boring.notch `NotchShape` — grows from the camera, not a floating pill.
    private static func chinPath(
        in rect: CGRect,
        topCornerRadius: CGFloat,
        bottomCornerRadius: CGFloat,
    ) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: rect.minX, y: rect.minY))
        path.addQuadCurve(
            to: CGPoint(x: rect.minX + topCornerRadius, y: rect.minY + topCornerRadius),
            control: CGPoint(x: rect.minX + topCornerRadius, y: rect.minY)
        )
        path.addLine(to: CGPoint(x: rect.minX + topCornerRadius, y: rect.maxY - bottomCornerRadius))
        path.addQuadCurve(
            to: CGPoint(x: rect.minX + topCornerRadius + bottomCornerRadius, y: rect.maxY),
            control: CGPoint(x: rect.minX + topCornerRadius, y: rect.maxY)
        )
        path.addLine(to: CGPoint(x: rect.maxX - topCornerRadius - bottomCornerRadius, y: rect.maxY))
        path.addQuadCurve(
            to: CGPoint(x: rect.maxX - topCornerRadius, y: rect.maxY - bottomCornerRadius),
            control: CGPoint(x: rect.maxX - topCornerRadius, y: rect.maxY)
        )
        path.addLine(to: CGPoint(x: rect.maxX - topCornerRadius, y: rect.minY + topCornerRadius))
        path.addQuadCurve(
            to: CGPoint(x: rect.maxX, y: rect.minY),
            control: CGPoint(x: rect.maxX - topCornerRadius, y: rect.minY)
        )
        path.addLine(to: CGPoint(x: rect.minX, y: rect.minY))
        return path
    }
}
