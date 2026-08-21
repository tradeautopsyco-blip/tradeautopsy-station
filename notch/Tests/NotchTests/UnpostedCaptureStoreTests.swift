import Foundation
import Testing
@testable import Notch

struct UnpostedCaptureStoreTests {
    private func tempDir() -> URL {
        FileManager.default.temporaryDirectory.appendingPathComponent("unposted-\(UUID().uuidString)", isDirectory: true)
    }

    @Test func insertThenAllReturnsTheCapture() throws {
        let dir = tempDir()
        let store = UnpostedCaptureStore(directory: dir)
        let rec = try store.insert(
            imageData: Data([0x89, 0x50, 0x4E, 0x47]),
            contentType: "image/png",
            caption: "chart",
            capturePhase: "during"
        )
        #expect(store.all().count == 1)
        #expect(store.all()[0].id == rec.id)
        #expect(store.all()[0].caption == "chart")
        #expect(FileManager.default.fileExists(atPath: store.fileURL(for: rec).path))
    }

    @Test func sweepDropsOldestWhenOverCap() throws {
        let dir = tempDir()
        var t = Date(timeIntervalSince1970: 1_700_000_000)
        let store = UnpostedCaptureStore(directory: dir, now: { t })
        for i in 0 ..< 21 {
            t = t.addingTimeInterval(1)
            _ = try store.insert(
                imageData: Data([UInt8(i)]),
                contentType: "image/png",
                caption: "\(i)",
                capturePhase: "post"
            )
        }
        #expect(store.all().count == UnpostedCaptureStore.maxItems)
    }

    @Test func deleteRemovesFileAndIndex() throws {
        let dir = tempDir()
        let store = UnpostedCaptureStore(directory: dir)
        let rec = try store.insert(
            imageData: Data([1, 2, 3]),
            contentType: "image/jpeg",
            caption: "",
            capturePhase: "pre"
        )
        store.delete(id: rec.id)
        #expect(store.all().isEmpty)
        #expect(!FileManager.default.fileExists(atPath: store.fileURL(for: rec).path))
    }
}
