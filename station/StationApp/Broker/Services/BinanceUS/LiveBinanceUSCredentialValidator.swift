import CryptoKit
import Foundation

public struct BinanceUSApiRestrictions: Equatable, Sendable, Decodable {
    public let enableReading: Bool?
    public let enableSpotAndMarginTrading: Bool?
    public let enableWithdrawals: Bool?

    enum CodingKeys: String, CodingKey {
        case enableReading
        case enableSpotAndMarginTrading
        case enableWithdrawals
    }
}

public enum BinanceUSPermissionClassifier {
    public static func classify(_ restrictions: BinanceUSApiRestrictions) -> BrokerPermissionPosture {
        if restrictions.enableWithdrawals == true {
            return .withdrawDetected
        }
        if restrictions.enableSpotAndMarginTrading == true {
            return .tradeEnabled
        }
        if restrictions.enableReading == true {
            return .readOnlyConfirmed
        }
        return .unverifiable
    }
}

public enum BinanceUSErrorClassifier {
    public static func classify(httpStatus: Int, binanceCode: Int?) -> BrokerCredentialValidationResult {
        if httpStatus == 429 || binanceCode == -1003 {
            return .transientFailure(.rateLimited)
        }
        if (500 ... 599).contains(httpStatus) {
            return .transientFailure(.brokerUnavailable)
        }
        if httpStatus == 401 || httpStatus == 403 || binanceCode == -2015 || binanceCode == -2014 {
            return .permanentFailure(.invalidCredentials)
        }
        if httpStatus == 418 {
            return .transientFailure(.brokerUnavailable)
        }
        return .permanentFailure(.invalidCredentials)
    }
}

public enum BinanceUSSigner {
    public static func signedQuery(apiSecret: String, parameters: [String: String]) -> String {
        let sorted = parameters.sorted { $0.key < $1.key }
        let query = sorted.map { "\($0.key)=\($0.value)" }.joined(separator: "&")
        let key = SymmetricKey(data: Data(apiSecret.utf8))
        let signature = HMAC<SHA256>.authenticationCode(for: Data(query.utf8), using: key)
        let hex = signature.map { String(format: "%02x", $0) }.joined()
        return "\(query)&signature=\(hex)"
    }
}

public struct BinanceUSValidationHTTPResponse: Equatable, Sendable {
    public let statusCode: Int
    public let body: Data

    public init(statusCode: Int, body: Data) {
        self.statusCode = statusCode
        self.body = body
    }
}

public protocol BinanceUSValidationTransport: Sendable {
    func fetchApiRestrictions(
        apiKey: String,
        apiSecret: String
    ) async throws -> BinanceUSValidationHTTPResponse
}

public struct URLSessionBinanceUSValidationTransport: BinanceUSValidationTransport {
    private let baseURL: URL
    private let session: URLSession

    public init(
        baseURL: URL = URL(string: "https://api.binance.us")!,
        session: URLSession = .shared
    ) {
        self.baseURL = baseURL
        self.session = session
    }

    public func fetchApiRestrictions(
        apiKey: String,
        apiSecret: String
    ) async throws -> BinanceUSValidationHTTPResponse {
        let timestamp = String(Int(Date().timeIntervalSince1970 * 1000))
        let signedQuery = BinanceUSSigner.signedQuery(
            apiSecret: apiSecret,
            parameters: ["timestamp": timestamp]
        )
        var request = URLRequest(url: baseURL.appendingPathComponent("/sapi/v1/account/apiRestrictions?\(signedQuery)"))
        request.httpMethod = "GET"
        request.setValue(apiKey, forHTTPHeaderField: "X-MBX-APIKEY")

        let (data, response) = try await session.data(for: request)
        let statusCode = (response as? HTTPURLResponse)?.statusCode ?? 0
        return BinanceUSValidationHTTPResponse(statusCode: statusCode, body: data)
    }
}

public struct LiveBinanceUSCredentialValidator: BrokerCredentialValidating, Sendable {
    private let transport: any BinanceUSValidationTransport

    public init(transport: any BinanceUSValidationTransport = URLSessionBinanceUSValidationTransport()) {
        self.transport = transport
    }

    public func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult {
        guard identity.brokerSlug == "binance_us" else {
            return .permanentFailure(.invalidCredentials)
        }

        do {
            let response = try await transport.fetchApiRestrictions(
                apiKey: credentials.apiKey,
                apiSecret: credentials.apiSecret
            )
            return Self.mapResponse(response)
        } catch {
            return .transientFailure(.networkUnavailable)
        }
    }

    static func mapResponse(_ response: BinanceUSValidationHTTPResponse) -> BrokerCredentialValidationResult {
        guard (200 ... 299).contains(response.statusCode) else {
            let code = Self.parseBinanceCode(from: response.body)
            return BinanceUSErrorClassifier.classify(httpStatus: response.statusCode, binanceCode: code)
        }

        guard let restrictions = try? JSONDecoder().decode(
            BinanceUSApiRestrictions.self,
            from: response.body
        ) else {
            return .success(permissionPosture: .unverifiable)
        }

        return .success(permissionPosture: BinanceUSPermissionClassifier.classify(restrictions))
    }

    private static func parseBinanceCode(from body: Data) -> Int? {
        guard let json = try? JSONSerialization.jsonObject(with: body) as? [String: Any],
              let code = json["code"] as? Int
        else {
            return nil
        }
        return code
    }
}
