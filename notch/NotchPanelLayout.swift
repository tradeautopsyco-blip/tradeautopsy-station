import CoreGraphics

/// Expanded `NSPanel` metrics for `NotchPanelController.layoutPanel`.
///
/// Standard expanded state fills almost all of `NSScreen.visibleFrame` so PLAN / Pulse / CAP
/// share one large surface (not the legacy 940×200 strip).
enum NotchPanelLayout {
    /// Horizontal inset inside `visibleFrame` (each side).
    static let expandedHorizontalInsetFromVisibleFrame: CGFloat = 24
    /// Vertical inset inside `visibleFrame` (top + bottom of usable area).
    static let expandedVerticalInsetFromVisibleFrame: CGFloat = 32

    static func expandedWidth(in visibleFrame: CGRect) -> CGFloat {
        max(640, visibleFrame.width - 2 * expandedHorizontalInsetFromVisibleFrame)
    }

    /// Body height below the optional hardware-notch spacer (`notchTopInset`).
    static func expandedContentHeight(in visibleFrame: CGRect) -> CGFloat {
        max(400, visibleFrame.height - 2 * expandedVerticalInsetFromVisibleFrame)
    }
}
