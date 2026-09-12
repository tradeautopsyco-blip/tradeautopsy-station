import CoreGraphics

/// Closed-chip slot on a notched Mac.
/// Height matches the menu bar. Width is the housing plus BoringNotch sound ears.
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

    /// Extra width BoringNotch adds for closed music live activity
    /// (`chinWidth += 2 * (effectiveClosedNotchHeight - 12) + 20`).
    static func soundEarExtra(menuBarHeight: CGFloat) -> CGFloat {
        2 * max(0, menuBarHeight - 12) + 20
    }

    /// Island flush to the physical top, elongated like BoringNotch sound:
    /// housing width plus left/right ears for logo and P&L.
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
        let extra = soundEarExtra(menuBarHeight: h)
        let w = housingW + extra
        return CGRect(
            x: notchLeft - extra / 2,
            y: screenFrame.maxY - h,
            width: w,
            height: h,
        )
    }
}
