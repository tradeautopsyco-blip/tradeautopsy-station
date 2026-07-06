import Foundation

public enum ProviderKeyNamespace: String, Codable, Hashable, Sendable {
    case marketData = "market-data"
    case aiWorkflow = "ai-workflow"
}

public enum ProviderKeyValidationState: String, Codable, Equatable, Sendable {
    case notValidated
    case unknown
}

public enum MarketDataProvider: String, CaseIterable, Codable, Hashable, Sendable, Identifiable {
    case openBB = "OpenBB"
    case polygon = "Polygon"
    case alphaVantage = "Alpha Vantage"

    public var id: String { rawValue }
}

public enum AIWorkflowProvider: String, CaseIterable, Codable, Hashable, Sendable, Identifiable {
    case openAI = "OpenAI"
    case anthropic = "Anthropic"
    case ollama = "Ollama"

    public var id: String { rawValue }
}

public struct ProviderAPIKeyIdentity: Equatable, Hashable, Sendable {
    public let namespace: ProviderKeyNamespace
    public let providerSlug: String
    public let keyID: UUID

    public init(namespace: ProviderKeyNamespace, providerSlug: String, keyID: UUID) {
        self.namespace = namespace
        self.providerSlug = providerSlug
        self.keyID = keyID
    }
}

public struct ProviderAPIKeyRecord: Equatable, Codable, Sendable {
    public let apiKey: String
    public let validationState: ProviderKeyValidationState

    public init(apiKey: String, validationState: ProviderKeyValidationState) {
        self.apiKey = apiKey
        self.validationState = validationState
    }
}

public struct ProviderAPIKeyListItem: Identifiable, Equatable, Sendable {
    public let id: UUID
    public let providerLabel: String
    public let maskedValue: String
    public let validationState: ProviderKeyValidationState

    public init(
        id: UUID,
        providerLabel: String,
        maskedValue: String,
        validationState: ProviderKeyValidationState
    ) {
        self.id = id
        self.providerLabel = providerLabel
        self.maskedValue = maskedValue
        self.validationState = validationState
    }
}

public enum ProviderAPIKeyMasking {
    public static func maskedValue(for apiKey: String) -> String {
        guard apiKey.count > 4 else {
            return String(repeating: "•", count: max(apiKey.count, 4))
        }
        let suffix = apiKey.suffix(4)
        return String(repeating: "•", count: 8) + suffix
    }
}
