import CoreGraphics

/// Closed-chip slot on a notched Mac.
/// The camera has no pixels. Volume-style chrome hangs **under** the housing,
/// same width as the cutout, one row (dot · track · label) on a shared axis.
enum BarNotchVolumeSlot {
    /// BoringNotch closed chin (`cornerRadiusInsets.closed.bottom`).
    static let chin: CGFloat = 14
    /// Visible strip below the housing — same idea as their sneak/HUD drop.
    static let hang: CGFloat = 22
    /// `getClosedNotchSize` adds 4pt to the aux-area gap.
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

    /// Island flush to the physical top; meter sits in `hang` below the camera.
    static func collapsedFrame(
        screenFrame: CGRect,
        visibleFrame: CGRect,
        notchLeft: CGFloat,
        notchWidth: CGFloat,
        notchInset: CGFloat,
        fallbackWidth: CGFloat,
    ) -> CGRect {
        let menuBarH = max(24, screenFrame.maxY - visibleFrame.maxY)
        let housingH = max(menuBarH, notchInset)
        let housingW = notchWidth > 0 ? notchWidth : fallbackWidth
        let w = housingW + chin * 2
        let h = housingH + hang
        let housingMid = notchLeft + housingW / 2
        return CGRect(
            x: housingMid - w / 2,
            y: screenFrame.maxY - h,
            width: w,
            height: h,
        )
    }
}
