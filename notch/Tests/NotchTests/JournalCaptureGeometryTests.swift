import CoreGraphics
import Foundation
import Testing
@testable import Notch

struct JournalCaptureGeometryTests {
    @Test func retina2xPointSizeHalvesPixels() {
        let size = JournalCaptureGeometry.pointSize(pixelWidth: 3024, pixelHeight: 1964, scale: 2)
        #expect(size.width == 1512)
        #expect(size.height == 982)
    }

    @Test func scaleOneKeepsPixelSize() {
        let size = JournalCaptureGeometry.pointSize(pixelWidth: 1512, pixelHeight: 982, scale: 1)
        #expect(size.width == 1512)
        #expect(size.height == 982)
    }

    @Test func fitsLandscapeStillInPortraitBounds() {
        let bounds = CGRect(x: 0, y: 0, width: 1024, height: 1366)
        let fitted = JournalCaptureGeometry.fittedDrawRect(
            imageSize: CGSize(width: 1512, height: 982),
            in: bounds
        )
        #expect(abs(fitted.width - 1024) < 0.5)
        #expect(abs(fitted.height - 665) < 0.5)
        #expect(abs(fitted.minX - 0) < 0.5)
        #expect(abs(fitted.minY - 350.5) < 0.5)
    }

    @Test func mapsSelectionIntoFittedImageCoordinates() {
        let fitted = CGRect(x: 0, y: 350.5, width: 1024, height: 665)
        let local = JournalCaptureGeometry.selectionInFittedImage(
            selection: CGRect(x: 100, y: 400, width: 200, height: 100),
            fittedRect: fitted
        )
        #expect(abs(local.minX - 100) < 0.5)
        #expect(abs(local.minY - 49.5) < 0.5)
        #expect(abs(local.width - 200) < 0.5)
        #expect(abs(local.height - 100) < 0.5)
    }

    @Test func selectionOutsideFittedRectIsEmpty() {
        let fitted = CGRect(x: 0, y: 350.5, width: 1024, height: 665)
        let local = JournalCaptureGeometry.selectionInFittedImage(
            selection: CGRect(x: 10, y: 10, width: 40, height: 40),
            fittedRect: fitted
        )
        #expect(local.width == 0)
        #expect(local.height == 0)
    }
}
