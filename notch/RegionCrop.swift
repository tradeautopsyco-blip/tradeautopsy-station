import AppKit
import CoreGraphics
import Foundation

enum RegionCrop {
    /// Map a view-space rect (AppKit, origin bottom-left) onto CGImage pixels (origin top-left).
    static func pixelRect(viewRect: CGRect, imageWidth: Int, imageHeight: Int, viewSize: CGSize) -> CGRect {
        guard viewSize.width > 0, viewSize.height > 0, imageWidth > 0, imageHeight > 0 else {
            return .zero
        }
        let sx = CGFloat(imageWidth) / viewSize.width
        let sy = CGFloat(imageHeight) / viewSize.height
        let x = viewRect.minX * sx
        let h = viewRect.height * sy
        let y = CGFloat(imageHeight) - viewRect.minY * sy - h
        return CGRect(x: x, y: y, width: viewRect.width * sx, height: h).integral
    }
}
