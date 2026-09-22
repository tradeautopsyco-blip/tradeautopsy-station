import Foundation

/// UI-safe device login challenge (A8): `user_code` only — never `device_code`.
public struct DeviceLoginChallenge: Equatable, Sendable, Codable {
    public let userCode: String
    public let verificationURI: String
    public let verificationURIComplete: String
    public let browserURL: String?
    public let expiresIn: UInt64
    public let interval: UInt64

    public init(
        userCode: String,
        verificationURI: String,
        verificationURIComplete: String,
        browserURL: String? = nil,
        expiresIn: UInt64,
        interval: UInt64
    ) {
        self.userCode = userCode
        self.verificationURI = verificationURI
        self.verificationURIComplete = verificationURIComplete
        self.browserURL = browserURL
        self.expiresIn = expiresIn
        self.interval = interval
    }

    enum CodingKeys: String, CodingKey {
        case userCode = "user_code"
        case verificationURI = "verification_uri"
        case verificationURIComplete = "verification_uri_complete"
        case browserURL = "browser_url"
        case expiresIn = "expires_in"
        case interval
    }
}

/// Identity returned after proving Console `GET /api/auth/station/session`.
public struct StationSessionIdentity: Equatable, Sendable, Codable {
    public let profileID: String
    public let email: String
    public let aud: String

    public init(profileID: String, email: String, aud: String) {
        self.profileID = profileID
        self.email = email
        self.aud = aud
    }

    enum CodingKeys: String, CodingKey {
        case profileID = "profile_id"
        case email
        case aud
    }
}

public enum DeviceLoginPhase: Equatable, Sendable {
    case idle
    case starting
    case awaitingBrowser
    case completing
    case signedIn
    case error
}

public enum DeviceLoginClientError: Error, Equatable, LocalizedError {
    case unavailable(String)
    case rejected(String)

    public var errorDescription: String? {
        switch self {
        case .unavailable(let message), .rejected(let message):
            return message
        }
    }
}

public protocol DeviceLoginClient: Sendable {
    func begin() async throws -> DeviceLoginChallenge
    func complete() async throws -> StationSessionIdentity
    func currentSession() async throws -> StationSessionIdentity?
    func signOut() async throws
}
