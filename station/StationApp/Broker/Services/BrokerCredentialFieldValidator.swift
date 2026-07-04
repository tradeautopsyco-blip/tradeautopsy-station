import Foundation

public enum BrokerCredentialFieldValidator {
    public static func invalidFields(apiKey: String, apiSecret: String) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiKey)
        }
        if apiSecret.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.apiSecret)
        }
        return invalid
    }
}
