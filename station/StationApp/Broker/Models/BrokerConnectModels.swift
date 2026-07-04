import Foundation

public enum BrokerCredentialField: String, Equatable, Sendable, CaseIterable {
    case apiKey
    case apiSecret
}

public enum BrokerPermissionPosture: Equatable, Sendable {
    case readOnlyConfirmed
    case tradeEnabled
    case unverifiable
    case withdrawDetected
}

public enum BrokerPermissionWarning: Equatable, Sendable, Codable {
    case tradeEnabled
    case unverifiable
}

public enum BrokerCredentialValidationFailure: Equatable, Sendable {
    case invalidCredentials
    case networkUnavailable
    case rateLimited
    case brokerUnavailable
}

public enum BrokerCredentialValidationResult: Equatable, Sendable {
    case success(permissionPosture: BrokerPermissionPosture)
    case transientFailure(BrokerCredentialValidationFailure)
    case permanentFailure(BrokerCredentialValidationFailure)
}

public enum BrokerConnectOutcome: Equatable, Sendable {
    case localValidationFailed(invalidFields: Set<BrokerCredentialField>)
    case blockedWithdrawPermission
    case validationTransientFailure(BrokerCredentialValidationFailure)
    case validationPermanentFailure(BrokerCredentialValidationFailure)
    case connected(permissionWarning: BrokerPermissionWarning?)
}

public protocol BrokerCredentialStoring: Sendable {
    func save(credentials: BrokerCredentials, for identity: BrokerConnectionIdentity) throws
    func read(for identity: BrokerConnectionIdentity) throws -> BrokerCredentials?
    func delete(for identity: BrokerConnectionIdentity) throws
    func hasCredentials(for identity: BrokerConnectionIdentity) -> Bool
}

public protocol BrokerCredentialValidating: Sendable {
    func validate(
        credentials: BrokerCredentials,
        identity: BrokerConnectionIdentity
    ) async -> BrokerCredentialValidationResult
}

@MainActor
public protocol BrokerSyncControlling: AnyObject {
    func startSync(for identity: BrokerConnectionIdentity) async throws
}

public protocol BrokerConnectionMetadataStoring: Sendable {
    func load(for identity: BrokerConnectionIdentity) -> BrokerConnectionMetadata?
    func save(_ metadata: BrokerConnectionMetadata, for identity: BrokerConnectionIdentity)
    func delete(for identity: BrokerConnectionIdentity)
}

public struct BrokerConnectionMetadata: Equatable, Sendable, Codable {
    public var permissionWarning: BrokerPermissionWarning?
    public var lastValidatedAt: Date?
    public var syncPaused: Bool

    enum CodingKeys: String, CodingKey {
        case permissionWarning
        case lastValidatedAt
        case syncPaused
    }

    public init(
        permissionWarning: BrokerPermissionWarning? = nil,
        lastValidatedAt: Date? = nil,
        syncPaused: Bool = false
    ) {
        self.permissionWarning = permissionWarning
        self.lastValidatedAt = lastValidatedAt
        self.syncPaused = syncPaused
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        permissionWarning = try container.decodeIfPresent(BrokerPermissionWarning.self, forKey: .permissionWarning)
        lastValidatedAt = try container.decodeIfPresent(Date.self, forKey: .lastValidatedAt)
        syncPaused = try container.decodeIfPresent(Bool.self, forKey: .syncPaused) ?? false
    }
}
