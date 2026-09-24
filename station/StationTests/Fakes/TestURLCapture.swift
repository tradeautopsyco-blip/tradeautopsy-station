import Foundation

/// Sendable-safe URL list for OAuth lifecycle tests (Swift 6 closure isolation).
final class TestURLCapture: @unchecked Sendable {
    private(set) var urls: [URL] = []

    func append(_ url: URL) {
        urls.append(url)
    }
}
