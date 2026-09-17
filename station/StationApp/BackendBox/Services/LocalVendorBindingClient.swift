import Foundation

public enum VendorBindingError: Error, Equatable, Sendable {
    case requestFailed
    case urlIsNotAKey
    case unknownVendor
    case missingKey
}

public struct VendorBindingPut: Equatable, Sendable {
    public let adapterId: String
    public let enabled: Bool
    public let apiKey: String?
    public let historyBudget: UInt32?

    public init(adapterId: String, enabled: Bool, apiKey: String? = nil, historyBudget: UInt32? = nil) {
        self.adapterId = adapterId
        self.enabled = enabled
        self.apiKey = apiKey
        self.historyBudget = historyBudget
    }
}

@MainActor
public protocol VendorBindingClienting: AnyObject {
    func putBinding(_ body: VendorBindingPut) async throws
}

@MainActor
public final class LocalVendorBindingClient: VendorBindingClienting {
    private let port: UInt16
    private let session: URLSession
    private let signRequest: StationWireClient.RequestSigner

    public init(
        port: UInt16 = AgentLoopback.port,
        session: URLSession = .shared,
        daemonSecret: String? = nil
    ) {
        self.port = port
        self.session = session
        let secret = daemonSecret ?? AgentDaemonSecret.resolveForSession()
        self.signRequest = StationWireClient.requestSigner(daemonSecret: secret)
    }

    public func putBinding(_ body: VendorBindingPut) async throws {
        let path = "/api/daemon/vendor-bindings"
        let payload = VendorBindingWireBody(
            adapterId: body.adapterId,
            enabled: body.enabled,
            apiKey: body.apiKey,
            historyBudget: body.historyBudget
        )
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        let data = try encoder.encode(payload)
        var request = signRequest("PUT", path, data)
        request.httpBody = data
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.url = URL(string: "http://127.0.0.1:\(port)\(path)")
        let (respData, response) = try await session.data(for: request)
        guard let http = response as? HTTPURLResponse else {
            throw VendorBindingError.requestFailed
        }
        if http.statusCode == 200 {
            return
        }
        if let decoded = try? JSONDecoder().decode(VendorBindingErrorBody.self, from: respData) {
            switch decoded.errorClass {
            case "url_is_not_a_key":
                throw VendorBindingError.urlIsNotAKey
            case "unknown_vendor":
                throw VendorBindingError.unknownVendor
            case "missing_key":
                throw VendorBindingError.missingKey
            default:
                throw VendorBindingError.requestFailed
            }
        }
        throw VendorBindingError.requestFailed
    }
}

private struct VendorBindingWireBody: Encodable {
    let adapterId: String
    let enabled: Bool
    let apiKey: String?
    let historyBudget: UInt32?
}

private struct VendorBindingErrorBody: Decodable {
    let errorClass: String
    let message: String?

    enum CodingKeys: String, CodingKey {
        case errorClass = "error_class"
        case message
    }
}
