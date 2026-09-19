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

    /// Keep the session lock while `visibleFrame` is unchanged so SwiftUI content
    /// churn does not resize the HUD. Recompute when the display moves or rotates.
    static let visibleFrameMatchEpsilon: CGFloat = 0.5

    static func visibleFramesMatch(_ a: CGRect, _ b: CGRect) -> Bool {
        abs(a.origin.x - b.origin.x) <= visibleFrameMatchEpsilon
            && abs(a.origin.y - b.origin.y) <= visibleFrameMatchEpsilon
            && abs(a.size.width - b.size.width) <= visibleFrameMatchEpsilon
            && abs(a.size.height - b.size.height) <= visibleFrameMatchEpsilon
    }

    static func resolvedExpandedContentSize(
        locked: CGSize?,
        lockedVisibleFrame: CGRect?,
        currentVisibleFrame: CGRect
    ) -> CGSize {
        if let locked, let lockedVisibleFrame,
           visibleFramesMatch(lockedVisibleFrame, currentVisibleFrame)
        {
            return locked
        }
        return CGSize(
            width: expandedWidth(in: currentVisibleFrame),
            height: expandedContentHeight(in: currentVisibleFrame)
        )
    }
}
