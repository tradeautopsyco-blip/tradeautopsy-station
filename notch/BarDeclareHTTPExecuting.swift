import Foundation

/// Composition-root seam for `POST …/api/daemon/bar/declare`. Production uses `URLSession`; tests inject stubs (#120).
protocol BarDeclareHTTPExecuting: Sendable {
    func data(for request: URLRequest) async throws -> (Data, URLResponse)
}

struct URLSessionBarDeclareHTTPExecutor: BarDeclareHTTPExecuting {
    private let session: URLSession

    init(session: URLSession = .shared) {
        self.session = session
    }

    func data(for request: URLRequest) async throws -> (Data, URLResponse) {
        try await session.data(for: request)
    }
}
