import Foundation
import Testing
@testable import Notch

struct RegionCropTests {
    @Test func mapsViewRectOntoPixelSpaceWithFlippedY() {
        let pixel = RegionCrop.pixelRect(
            viewRect: CGRect(x: 10, y: 10, width: 20, height: 20),
            imageWidth: 200,
            imageHeight: 200,
            viewSize: CGSize(width: 100, height: 100)
        )
        #expect(pixel.minX == 20)
        #expect(pixel.width == 40)
        #expect(pixel.height == 40)
        #expect(pixel.minY == 140)
    }
}
