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

    public static func invalidKotakFields(
        consumerKey: String,
        tradeToken: String,
        sid: String,
        baseUrl: String
    ) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if consumerKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.consumerKey)
        }
        if tradeToken.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.tradeToken)
        }
        if sid.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.sid)
        }
        if baseUrl.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.baseUrl)
        }
        return invalid
    }
}
