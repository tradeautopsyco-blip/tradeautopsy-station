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

    public static func invalidKotakLoginFields(
        consumerKey: String,
        mobileNumber: String,
        ucc: String,
        totp: String,
        mpin: String
    ) -> Set<BrokerCredentialField> {
        var invalid = Set<BrokerCredentialField>()
        if consumerKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.consumerKey)
        }
        if mobileNumber.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.mobileNumber)
        }
        if ucc.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.ucc)
        }
        if totp.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.totp)
        }
        if mpin.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
            invalid.insert(.mpin)
        }
        return invalid
    }
}
