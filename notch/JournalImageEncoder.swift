import AppKit
import Foundation
import ImageIO
import UniformTypeIdentifiers

enum JournalImageEncoderError: Error {
    case unsupportedType
    case encodeFailed
    case tooLarge
}

enum JournalImageEncoder {
    static let maxEdge: CGFloat = 1920
    static let maxBytes = 2 * 1024 * 1024
    static let jpegQuality = 0.82

    static func encode(_ data: Data, hintedType: String?) throws -> (data: Data, contentType: String) {
        let hinted = (hintedType ?? "").lowercased()
        if hinted == "application/pdf" || hinted.hasPrefix("text/") {
            throw JournalImageEncoderError.unsupportedType
        }
        guard let image = NSImage(data: data) else {
            throw JournalImageEncoderError.unsupportedType
        }
        let size = image.size
        guard size.width > 0, size.height > 0 else {
            throw JournalImageEncoderError.unsupportedType
        }
        let scale = min(1, Self.maxEdge / max(size.width, size.height))
        let target = NSSize(width: floor(size.width * scale), height: floor(size.height * scale))
        guard let cg = rasterize(image, size: target) else {
            throw JournalImageEncoderError.encodeFailed
        }

        if hinted == "image/png" || hinted.isEmpty {
            if let png = pngData(cg), png.count <= Self.maxBytes, scale == 1, hinted == "image/png" {
                return (png, "image/png")
            }
        }

        var q = Self.jpegQuality
        for _ in 0 ..< 3 {
            if let jpeg = jpegData(cg, quality: q), jpeg.count <= Self.maxBytes {
                return (jpeg, "image/jpeg")
            }
            q -= 0.12
        }
        throw JournalImageEncoderError.tooLarge
    }

    private static func rasterize(_ image: NSImage, size: NSSize) -> CGImage? {
        let rect = NSRect(origin: .zero, size: size)
        guard let rep = NSBitmapImageRep(
            bitmapDataPlanes: nil,
            pixelsWide: Int(size.width),
            pixelsHigh: Int(size.height),
            bitsPerSample: 8,
            samplesPerPixel: 4,
            hasAlpha: true,
            isPlanar: false,
            colorSpaceName: .deviceRGB,
            bytesPerRow: 0,
            bitsPerPixel: 0
        ) else { return nil }
        NSGraphicsContext.saveGraphicsState()
        NSGraphicsContext.current = NSGraphicsContext(bitmapImageRep: rep)
        image.draw(in: rect, from: .zero, operation: .copy, fraction: 1)
        NSGraphicsContext.restoreGraphicsState()
        return rep.cgImage
    }

    private static func pngData(_ image: CGImage) -> Data? {
        let mut = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(mut, UTType.png.identifier as CFString, 1, nil) else {
            return nil
        }
        CGImageDestinationAddImage(dest, image, nil)
        guard CGImageDestinationFinalize(dest) else { return nil }
        return mut as Data
    }

    private static func jpegData(_ image: CGImage, quality: CGFloat) -> Data? {
        let mut = NSMutableData()
        guard let dest = CGImageDestinationCreateWithData(mut, UTType.jpeg.identifier as CFString, 1, nil) else {
            return nil
        }
        CGImageDestinationAddImage(dest, image, [kCGImageDestinationLossyCompressionQuality: quality] as CFDictionary)
        guard CGImageDestinationFinalize(dest) else { return nil }
        return mut as Data
    }
}
