import CoreGraphics

/// Closed-chip slot on a notched Mac.
/// BoringNotch `getClosedNotchSize` with **Match menu bar height**: width is the
/// aux-area gap, height is the menu bar — no hang under the camera.
enum BarNotchVolumeSlot {
    /// BoringNotch `cornerRadiusInsets.closed.bottom` — drawn inside the window.
    static let chin: CGFloat = 14
    /// BoringNotch `getClosedNotchSize` width fudge (`+ 4`).
    static let housingFudge: CGFloat = 4

    /// Gap between the left and right menu-bar auxiliary areas *is* the housing.
    static func hardwareNotch(
        screenFrame: CGRect,
        leftMenu: CGRect,
        rightMenu: CGRect,
    ) -> CGRect? {
        let leftEdge = leftMenu.isNull || leftMenu.width <= 0 ? screenFrame.minX : leftMenu.maxX
        let rightEdge = rightMenu.isNull || rightMenu.width <= 0 ? screenFrame.maxX : rightMenu.minX
        let width = rightEdge - leftEdge + housingFudge
        guard width >= 80, width <= screenFrame.width * 0.45 else { return nil }
        return CGRect(x: leftEdge - housingFudge / 2, y: 0, width: width, height: 0)
    }

    /// Menu-bar height — same as BoringNotch `WindowHeightMode.matchMenuBar`.
    static func menuBarHeight(screenFrame: CGRect, visibleFrame: CGRect) -> CGFloat {
        max(24, screenFrame.maxY - visibleFrame.maxY)
    }

    /// Island flush to the physical top, cutout width × menu-bar height.
    /// Logo and P&L sit in the ears inside that strip — not a second blob below.
    static func collapsedFrame(
        screenFrame: CGRect,
        visibleFrame: CGRect,
        notchLeft: CGFloat,
        notchWidth: CGFloat,
        notchInset _: CGFloat,
        fallbackWidth: CGFloat,
    ) -> CGRect {
        let housingW = notchWidth > 0 ? notchWidth : fallbackWidth
        let h = menuBarHeight(screenFrame: screenFrame, visibleFrame: visibleFrame)
        return CGRect(
            x: notchLeft,
            y: screenFrame.maxY - h,
            width: housingW,
            height: h,
        )
    }
}
