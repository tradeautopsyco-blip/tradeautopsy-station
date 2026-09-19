import CoreGraphics
import Foundation

/// Point-space mapping for the freeze overlay: ScreenCaptureKit pixels vs `NSScreen.frame`.
enum JournalCaptureGeometry {
    static func pointSize(pixelWidth: Int, pixelHeight: Int, scale: CGFloat) -> CGSize {
        let s = max(scale, 0.0001)
        return CGSize(
            width: CGFloat(pixelWidth) / s,
            height: CGFloat(pixelHeight) / s
        )
    }

    /// Aspect-fit `imageSize` inside `bounds` (AppKit origin: bottom-left). Never stretches.
    static func fittedDrawRect(imageSize: CGSize, in bounds: CGRect) -> CGRect {
        guard imageSize.width > 0, imageSize.height > 0, bounds.width > 0, bounds.height > 0 else {
            return .zero
        }
        let imageAspect = imageSize.width / imageSize.height
        let boundsAspect = bounds.width / bounds.height
        let size: CGSize
        if imageAspect > boundsAspect {
            size = CGSize(width: bounds.width, height: bounds.width / imageAspect)
        } else {
            size = CGSize(width: bounds.height * imageAspect, height: bounds.height)
        }
        return CGRect(
            x: bounds.midX - size.width / 2,
            y: bounds.midY - size.height / 2,
            width: size.width,
            height: size.height
        )
    }

    /// Selection in view space → rect inside the fitted image (still bottom-left).
    static func selectionInFittedImage(selection: CGRect, fittedRect: CGRect) -> CGRect {
        let inter = selection.intersection(fittedRect)
        guard !inter.isNull, !inter.isInfinite, inter.width > 0, inter.height > 0 else {
            return .zero
        }
        return CGRect(
            x: inter.minX - fittedRect.minX,
            y: inter.minY - fittedRect.minY,
            width: inter.width,
            height: inter.height
        )
    }
}
