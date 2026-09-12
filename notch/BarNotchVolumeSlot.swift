import CoreGraphics

/// Closed-chip slot on a notched Mac.
/// The camera has no pixels. Chrome is the same width as the cutout so it does
/// not cover Integrate / Window; the live row (logo · P&L) hangs **under** it.
enum BarNotchVolumeSlot {
    /// BoringNotch closed chin (`cornerRadiusInsets.closed.bottom`).
    /// Drawn *inside* the housing width — adding it to the window covers the Window menu.
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

    /// Island flush to the physical top, **exactly** the cutout width.
    /// P&L sits in `hang` below the camera — the Dynamic Island waveform slot.
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
        let h = housingH + hang
        return CGRect(
            x: notchLeft,
            y: screenFrame.maxY - h,
            width: housingW,
            height: h,
        )
    }
}
